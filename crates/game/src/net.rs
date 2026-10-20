//! Online play: the connection to the multiplayer server (`crates/server`).
//!
//! **How a game stays in sync.** The server never sends game state. It sends
//! the lobby's seed and seats once, then an ordered log of
//! [`Step`](battler_sim::run::Step)s, and this client applies the same steps
//! to its own copy of the lobby; the sim is deterministic, so every copy agrees.
//!
//! **Feeling instant.** Waiting for the server before showing a purchase would
//! feel sluggish, so [`Online`] keeps two lobbies: the *confirmed* one (only
//! steps from the server) and the one on screen (confirmed plus this player's
//! commands still on their way, re-applied on top). When the server echoes a
//! command back it moves into the confirmed lobby; if it refuses one, the
//! command just disappears the next time the screen's lobby is rebuilt.
//!
//! **Bevy details.** A browser WebSocket can't move between threads, so the
//! connection is a *non-send* resource (`Link`), and the system that pumps it
//! is an exclusive system (`fn pump(world: &mut World)`), which always runs on
//! the main thread with the whole `World` to itself.

use std::collections::{BTreeMap, VecDeque};

use battler_proto::{
    ClientMsg, Frame, Intent, PROTOCOL_VERSION, Phase, SeatInfo, ServerMsg, decode, encode,
};
use battler_sim::run::{Command, Lobby, PlayerId, Step};
use bevy::prelude::*;
use ewebsock::{WsEvent, WsMessage, WsReceiver, WsSender};

use crate::battle::{self, Playback};
use crate::editor;
use crate::state::{AppState, GameRun, Planning, Rev, RunPhase};

/// Where the server is, unless overridden (`?server=` on the web,
/// `BATTLER_SERVER` natively, e.g. `ws://localhost:10000/ws` for a local one).
pub const DEFAULT_SERVER: &str = "wss://battler-server.onrender.com/ws";

/// Seconds between keep-alive pings when nothing else is sent. Render's free
/// plan sleeps after 15 minutes without incoming traffic.
const PING_SECS: f64 = 25.0;

/// Reconnect attempts after losing a game's connection, a few seconds apart.
const RECONNECT_TRIES: u32 = 20;
const RECONNECT_SECS: f64 = 3.0;

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Net>().add_systems(Update, pump);
    }
}

/// What the connection is doing, for the menus.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Offline,
    Connecting,
    InRoom,
    InGame,
    /// Lost the connection mid-game; trying to rejoin.
    Reconnecting,
    /// Something went wrong; the reason is shown on the main menu.
    Failed(String),
}

/// The waiting room, as the server last described it.
#[derive(Debug, Clone)]
pub struct RoomView {
    pub code: String,
    pub seats: Vec<SeatInfo>,
    pub host: usize,
    pub you: usize,
}

/// The online side of the app (the connection itself is the `Link`).
#[derive(Resource, Default)]
pub struct Net {
    pub status: Status,
    pub room: Option<RoomView>,
    /// A short message for the room screen ("Invite link copied").
    pub notice: Option<String>,
    /// The room code and our token in it (to rejoin).
    session: Option<(String, u64)>,
    /// The name we joined with (reused when reconnecting).
    name: String,
    /// Messages for the server outside a game (Start, Kick, Leave).
    outbox: Vec<ClientMsg>,
    /// A connection the UI asked for, opened by the next `pump`.
    want: Option<Intent>,
    /// Close the link once the outbox is sent.
    hang_up: bool,
    reconnect: Option<Reconnect>,
}

#[derive(Debug, Clone, Copy)]
struct Reconnect {
    at: f64,
    tries: u32,
}

impl Net {
    /// Opens a connection to host, join or rejoin a room.
    pub fn connect(&mut self, intent: Intent, name: &str) {
        self.name = battler_proto::clean_name(name);
        save("battler.name", &self.name);
        self.want = Some(intent);
        self.status = Status::Connecting;
        self.notice = None;
        self.reconnect = None;
    }

    pub fn send(&mut self, msg: ClientMsg) {
        self.outbox.push(msg);
    }

    /// Leaves the room or game (a bot takes over a game seat) and hangs up.
    pub fn leave(&mut self) {
        if self.status == Status::Offline {
            return;
        }
        self.outbox.push(ClientMsg::Leave);
        self.hang_up = true;
        self.status = Status::Offline;
        self.room = None;
        self.session = None;
        self.reconnect = None;
        forget("battler.session");
    }

    pub fn fail(&mut self, reason: impl Into<String>) {
        self.status = Status::Failed(reason.into());
    }

    /// Words for the main menu about what's happening, if anything.
    pub fn status_line(&self) -> Option<String> {
        match &self.status {
            Status::Connecting => {
                Some("Connecting... (the server may take a minute to wake up)".into())
            }
            Status::Reconnecting => Some("Connection lost - reconnecting...".into()),
            Status::Failed(reason) => Some(reason.clone()),
            _ => None,
        }
    }
}

/// The online half of a [`GameRun`].
pub struct Online {
    pub code: String,
    /// The lobby exactly as the server's log has it.
    confirmed: Lobby,
    /// Steps applied to `confirmed` (the log's length).
    seq: u32,
    /// This player's commands sent but not yet back in the log, oldest first.
    pending: VecDeque<Command>,
    /// Messages for the server from the game screens.
    outbox: Vec<ClientMsg>,
    pub phase: Phase,
    /// Length of the current planning phase, for the timer bar.
    pub clock_total: f32,
    /// Pressed Ready this planning phase.
    pub ready_sent: bool,
    /// Who is ready, and which human seats are connected (from Presence).
    ready: Vec<PlayerId>,
    connected: Vec<bool>,
}

impl Online {
    /// A command was applied on screen: send it, and remember it until the
    /// server's log confirms it.
    pub(crate) fn sent(&mut self, command: Command) {
        self.pending.push_back(command.clone());
        self.outbox.push(ClientMsg::Command(command));
    }

    /// Done planning: tell the server once.
    pub fn ready(&mut self) {
        if !self.ready_sent && self.phase == Phase::Planning {
            self.ready_sent = true;
            self.outbox.push(ClientMsg::Ready);
        }
    }

    /// A short word for the players list: a bot, away, or ready.
    pub fn marker(&self, id: PlayerId, bot: bool) -> Option<&'static str> {
        if bot {
            Some("bot")
        } else if self.connected.get(id.0) == Some(&false) {
            Some("away")
        } else if self.phase == Phase::Planning && self.ready.contains(&id) {
            Some("ready")
        } else {
            None
        }
    }
}

/// Rebuilds the lobby on screen: the confirmed lobby plus pending commands.
/// Commands that no longer apply (the server will refuse them) are dropped.
fn rebuild_display(run: &mut GameRun) {
    let Some(online) = run.online.as_mut() else { return };
    let mut lobby = online.confirmed.clone();
    // `retain` keeps the commands for which the closure returns true, and the
    // closure can mutate `lobby` as it goes: each command is re-applied in order.
    online.pending.retain(|c| lobby.apply(run.my_id, c.clone()).is_ok());
    run.lobby = lobby;
}

// ----- The connection -------------------------------------------------------

/// The open WebSocket. Not `Send` on the web, hence a non-send resource.
struct Link {
    tx: WsSender,
    rx: WsReceiver,
    open: bool,
    /// Sent as soon as the socket opens.
    hello: Option<ClientMsg>,
    /// Messages that arrived early, by number, and the next number due.
    early: BTreeMap<u32, ServerMsg>,
    next: u32,
    last_sent: f64,
}

impl Link {
    fn send(&mut self, msg: &ClientMsg, now: f64) {
        self.tx.send(WsMessage::Binary(encode(msg)));
        self.last_sent = now;
    }
}

/// Moves messages between the server and the game, once per frame.
fn pump(world: &mut World) {
    let now = world.resource::<Time<Real>>().elapsed_secs_f64();

    // Time to try reconnecting?
    let retry = {
        let net = world.resource::<Net>();
        match (net.reconnect, &net.session) {
            (Some(r), Some((code, token))) if now >= r.at => {
                Some(Intent::Rejoin { code: code.clone(), token: *token })
            }
            _ => None,
        }
    };
    if let Some(intent) = retry {
        let mut net = world.resource_mut::<Net>();
        net.want = Some(intent);
        if let Some(r) = net.reconnect.as_mut() {
            r.at = now + RECONNECT_SECS;
        }
    }

    // Open a connection if the UI (or a retry) asked for one.
    if let Some(intent) = world.resource_mut::<Net>().want.take() {
        world.remove_non_send::<Link>();
        let name = world.resource::<Net>().name.clone();
        match ewebsock::connect(server_url(), ewebsock::Options::default()) {
            Ok((tx, rx)) => {
                let hello = ClientMsg::Hello { version: PROTOCOL_VERSION, name, intent };
                let link = Link {
                    tx,
                    rx,
                    open: false,
                    hello: Some(hello),
                    early: BTreeMap::new(),
                    next: 0,
                    last_sent: now,
                };
                world.insert_non_send(link);
            }
            Err(e) => world.resource_mut::<Net>().fail(format!("Can't reach the server: {e}")),
        }
    }

    // Take the link out of the world while we work, so handling a message can
    // borrow the world freely; it goes back in at the end if it's still alive.
    let Some(mut link) = world.remove_non_send::<Link>() else { return };
    let mut lost = None;
    while let Some(event) = link.rx.try_recv() {
        match event {
            WsEvent::Opened => {
                link.open = true;
                if let Some(hello) = link.hello.take() {
                    link.send(&hello, now);
                }
            }
            WsEvent::Message(WsMessage::Binary(bytes)) => {
                if let Ok(frame) = decode::<Frame>(&bytes) {
                    link.early.insert(frame.n, frame.msg);
                }
            }
            WsEvent::Message(_) => {}
            WsEvent::Error(e) => lost = Some(e),
            WsEvent::Closed => lost = Some("closed".to_string()),
        }
    }
    // Handle messages strictly in the server's order.
    while let Some(msg) = link.early.remove(&link.next) {
        link.next += 1;
        handle(world, msg);
    }

    // Send what's waiting.
    if link.open {
        let mut out = std::mem::take(&mut world.resource_mut::<Net>().outbox);
        if let Some(mut run) = world.get_resource_mut::<GameRun>()
            && let Some(online) = run.online.as_mut()
        {
            out.append(&mut online.outbox);
        }
        for msg in &out {
            link.send(msg, now);
        }
        if now - link.last_sent > PING_SECS {
            link.send(&ClientMsg::Ping, now);
        }
    }

    let mut net = world.resource_mut::<Net>();
    if std::mem::take(&mut net.hang_up) {
        // Dropping the link closes the socket (after what was just sent).
        net.outbox.clear();
        return;
    }
    match lost {
        None => world.insert_non_send(link),
        Some(error) => connection_lost(world, error, link.open, now),
    }
}

/// The socket closed without us asking.
fn connection_lost(world: &mut World, error: String, was_open: bool, now: f64) {
    let in_game = world
        .get_resource::<GameRun>()
        .and_then(|run| run.online.as_ref())
        .is_some_and(|o| o.phase != Phase::Over);
    let mut net = world.resource_mut::<Net>();
    match net.status.clone() {
        Status::Failed(_) | Status::Offline => {}
        Status::InGame | Status::Reconnecting if in_game && net.session.is_some() => {
            let tries = net.reconnect.map_or(0, |r| r.tries) + 1;
            if tries > RECONNECT_TRIES {
                net.reconnect = None;
                net.fail("Lost the connection to the game. Use Rejoin to try again.");
                world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
            } else {
                net.status = Status::Reconnecting;
                net.reconnect = Some(Reconnect { at: now + RECONNECT_SECS, tries });
            }
        }
        Status::Connecting if !was_open => {
            net.fail(format!(
                "Couldn't reach the server ({error}). It may still be waking up - try again in a minute."
            ));
        }
        _ => {
            net.fail("Lost the connection to the room.");
            net.room = None;
            world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
        }
    }
    world.resource_mut::<Rev>().bump();
}

/// One message from the server.
fn handle(world: &mut World, msg: ServerMsg) {
    world.resource_mut::<Rev>().bump();
    match msg {
        ServerMsg::Welcome { code, token } => {
            save("battler.session", &format!("{code}:{token}"));
            let mut net = world.resource_mut::<Net>();
            net.session = Some((code, token));
            net.reconnect = None;
        }
        ServerMsg::Refused { reason } => {
            let mut net = world.resource_mut::<Net>();
            if matches!(net.status, Status::Reconnecting) || net.reconnect.is_some() {
                // The game is gone (or our seat is): stop trying.
                net.session = None;
                net.reconnect = None;
                forget("battler.session");
            }
            net.fail(reason);
            net.hang_up = true;
            world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
        }
        ServerMsg::Room { code, seats, host, you } => {
            let mut net = world.resource_mut::<Net>();
            net.room = Some(RoomView { code, seats, host, you });
            net.status = Status::InRoom;
            world.resource_mut::<NextState<AppState>>().set(AppState::Room);
        }
        ServerMsg::Kicked => {
            let mut net = world.resource_mut::<Net>();
            net.fail("The host removed you from the room.");
            net.room = None;
            net.session = None;
            net.hang_up = true;
            forget("battler.session");
            world.resource_mut::<NextState<AppState>>().set(AppState::MainMenu);
        }
        ServerMsg::Game { seed, seats, me, log, phase, ms_left } => {
            start_game(world, seed, &seats, me, &log, phase, ms_left);
        }
        ServerMsg::Step { seq, step } => on_step(world, seq, step),
        ServerMsg::Rejected { reason } => {
            let Some(mut run) = world.remove_resource::<GameRun>() else { return };
            if let Some(online) = run.online.as_mut() {
                // The server handles commands in order, so the refused one is
                // the oldest still waiting.
                online.pending.pop_front();
            }
            rebuild_display(&mut run);
            let mut planning = world.resource_mut::<Planning>();
            planning.toast(reason);
            // The unit's program may have gone back to what it was.
            editor::load_draft(run.me(), &mut planning);
            world.insert_resource(run);
        }
        ServerMsg::Clock { phase, ms_left } => {
            let secs = ms_left as f32 / 1000.0;
            let mut online_game = false;
            if let Some(mut run) = world.get_resource_mut::<GameRun>()
                && let Some(online) = run.online.as_mut()
            {
                online_game = true;
                online.phase = phase;
                if phase == Phase::Planning {
                    online.clock_total = secs;
                    online.ready_sent = false;
                }
            }
            if online_game && phase == Phase::Planning {
                world.resource_mut::<Planning>().time_left = secs;
            }
        }
        ServerMsg::Presence { ready, connected } => {
            if let Some(mut run) = world.get_resource_mut::<GameRun>()
                && let Some(online) = run.online.as_mut()
            {
                online.ready = ready;
                online.connected = connected;
            }
        }
        ServerMsg::Check { seq, hash } => {
            if let Some(mut run) = world.get_resource_mut::<GameRun>()
                && let Some(online) = run.online.as_mut()
                && online.seq == seq
                && online.confirmed.state_hash() != hash
            {
                warn!("lobby out of sync with the server at step {seq}; asking for a resync");
                online.outbox.push(ClientMsg::Resync);
            }
        }
    }
}

/// Builds the game from the server's description: a new game, a rejoin, or a
/// resync after drifting out of step.
fn start_game(
    world: &mut World,
    seed: u64,
    seats: &[battler_sim::run::Seat],
    me: PlayerId,
    log: &[Step],
    phase: Phase,
    ms_left: u32,
) {
    let mut lobby = Lobby::with_seats(seed, seats);
    lobby.replay(log);
    let code = world.resource::<Net>().session.as_ref().map(|s| s.0.clone()).unwrap_or_default();
    let secs = ms_left as f32 / 1000.0;
    let online = Online {
        code,
        confirmed: lobby.clone(),
        seq: log.len() as u32,
        pending: VecDeque::new(),
        outbox: Vec::new(),
        phase,
        clock_total: if phase == Phase::Planning { secs } else { 60.0 },
        ready_sent: false,
        ready: Vec::new(),
        connected: vec![true; seats.len()],
    };
    let already_playing = *world.resource::<State<AppState>>().get() == AppState::InRun;
    world.insert_resource(GameRun { lobby, my_id: me, online: Some(online) });
    {
        let mut net = world.resource_mut::<Net>();
        net.status = Status::InGame;
        net.room = None;
        net.reconnect = None;
    }
    let mut planning = world.resource_mut::<Planning>();
    if !already_playing {
        *planning = Planning::default();
    }
    if phase == Phase::Planning {
        planning.time_left = secs;
    }
    let next = match phase {
        Phase::Over => AppState::GameOver,
        // Rejoining mid-battle shows the planning screen until the round
        // resolves; commands wait for the next planning phase.
        Phase::Planning | Phase::Battle => AppState::InRun,
    };
    world.resource_mut::<NextState<AppState>>().set(next);
}

/// The next step of the log.
fn on_step(world: &mut World, seq: u32, step: Step) {
    let Some(mut run) = world.remove_resource::<GameRun>() else { return };
    let my_id = run.my_id;
    if run.online.is_none() {
        world.insert_resource(run);
        return;
    }
    let online = run.online.as_mut().expect("checked above");
    if seq != online.seq + 1 {
        // A gap: something was lost. Ask for the whole game again.
        online.outbox.push(ClientMsg::Resync);
        world.insert_resource(run);
        return;
    }
    if online.confirmed.step(&step).is_err() {
        warn!("step {seq} failed on the client; asking for a resync");
        online.outbox.push(ClientMsg::Resync);
    }
    online.seq = seq;
    if let Step::Command { player, command } = &step
        && *player == my_id
        && online.pending.front() == Some(command)
    {
        online.pending.pop_front();
    }
    if step == Step::StartRound {
        online.phase = Phase::Battle;
        online.ready_sent = false;
    }
    rebuild_display(&mut run);

    let mut planning = world.remove_resource::<Planning>().expect("planning state");
    match step {
        Step::StartRound => {
            if let Some(playback) = battle::begin_battle(&run, &mut planning) {
                world.insert_resource(playback);
                if let Some(mut next) = world.get_resource_mut::<NextState<RunPhase>>() {
                    next.set(RunPhase::Battle);
                }
            }
        }
        Step::Resolve => {
            if let Some(mut playback) = world.get_resource_mut::<Playback>() {
                playback.finish();
            }
            planning.toast = None;
            world.resource_scope(|world, mut next_app: Mut<NextState<AppState>>| {
                let mut next_phase = world.get_resource_mut::<NextState<RunPhase>>();
                battle::finish_round(&run, &mut planning, next_phase.as_deref_mut(), &mut next_app);
            });
        }
        _ => {}
    }
    editor::sync_draft(run.me(), &mut planning);
    world.insert_resource(planning);
    world.insert_resource(run);
}

// ----- Browser helpers ------------------------------------------------------

/// The server's WebSocket address.
fn server_url() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        query_param("server").unwrap_or_else(|| DEFAULT_SERVER.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("BATTLER_SERVER").unwrap_or_else(|_| DEFAULT_SERVER.to_string())
    }
}

/// The game this browser was last in, as (room code, token), for Rejoin.
pub fn saved_session() -> Option<(String, u64)> {
    let saved = load("battler.session")?;
    let (code, token) = saved.split_once(':')?;
    Some((code.to_string(), token.parse().ok()?))
}

/// The name typed last time.
pub fn saved_name() -> Option<String> {
    load("battler.name")
}

/// A room code from an invite link (`?room=KQZT`).
pub fn invited_code() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        query_param("room").map(|c| battler_proto::clean_code(&c))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

/// Copies the invite link for a room to the clipboard (web only).
pub fn copy_invite_link(code: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let Some(window) = web_sys::window() else { return };
        let page = window.location().href().unwrap_or_default();
        let base = page.split(['?', '#']).next().unwrap_or_default();
        let link = format!("{base}?room={code}");
        // `battlerCopy` is a tiny helper in index.html around the browser's
        // clipboard API. Looking it up on `window` keeps this crate free of
        // web-sys's unstable clipboard bindings.
        if let Ok(f) = js_sys::Reflect::get(&window, &"battlerCopy".into())
            && let Ok(f) = f.dyn_into::<js_sys::Function>()
        {
            let _ = f.call1(&wasm_bindgen::JsValue::NULL, &link.into());
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = code;
    }
}

#[cfg(target_arch = "wasm32")]
fn query_param(key: &str) -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    search.trim_start_matches('?').split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key && !v.is_empty()).then(|| v.to_string())
    })
}

/// Browser storage (`localStorage`), so names and sessions survive a reload.
/// Natively there's nowhere to keep them, so they're simply not kept.
#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

fn load(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        storage()?.get_item(key).ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn save(key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (key, value);
}

fn forget(key: &str) {
    #[cfg(target_arch = "wasm32")]
    if let Some(s) = storage() {
        let _ = s.remove_item(key);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = key;
}

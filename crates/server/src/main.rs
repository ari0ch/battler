//! Battler's multiplayer server.
//!
//! - `GET /health` answers `ok`. Render uses it to check the service is up,
//!   and the web build calls it on page load to wake the server from sleep.
//! - `GET /ws` is the game's WebSocket (messages in `battler-proto`).
//!
//! **Structure.** Each room runs as its own tokio task that owns a
//! [`room::Room`] (the rules, with no networking in it). Connections talk to
//! their room through channels. This "actor" style means no locks around game
//! state: only the room's task ever touches it, one event at a time. The one
//! shared structure is the registry from room codes to room tasks.

mod room;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use battler_proto::{
    CODE_LEN, CODE_LETTERS, ClientMsg, Frame, Intent, PROTOCOL_VERSION, ServerMsg, decode, encode,
};
use tokio::sync::{mpsc, oneshot};

use room::{ConnId, Room};

/// Sent as the body of `/`, so a browser can tell it reached the right server.
const GREETING: &str = concat!("battler-server ", env!("CARGO_PKG_VERSION"));

/// The largest message a client may send. Commands are tiny; this only stops
/// abuse.
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// How long a new connection has to say Hello.
const HELLO_TIMEOUT: Duration = Duration::from_secs(10);

/// Room codes to the channel into each room's task.
///
/// `Arc` shares it between connections (cloning an `Arc` copies a pointer, not
/// the map); `Mutex` lets them take turns changing it. It's a std mutex, not
/// tokio's, because it's never held across an `.await`.
type Registry = Arc<Mutex<HashMap<String, mpsc::UnboundedSender<RoomEvent>>>>;

/// Everything request handlers share.
#[derive(Clone)]
struct AppState {
    rooms: Registry,
    /// The server's clock: rooms measure time in ms since this.
    started: Instant,
}

/// What a connection tells its room.
enum RoomEvent {
    /// Join (or with a token, rejoin). Outgoing messages go to `tx`; `reply`
    /// says whether the room took them.
    Join {
        conn: ConnId,
        name: String,
        rejoin: Option<u64>,
        tx: mpsc::UnboundedSender<ServerMsg>,
        reply: oneshot::Sender<bool>,
    },
    Msg {
        conn: ConnId,
        msg: ClientMsg,
    },
    Gone {
        conn: ConnId,
    },
}

// `#[tokio::main]` turns `async fn main` into a normal `fn main` that starts
// tokio's runtime and blocks on this future until it finishes.
#[tokio::main]
async fn main() {
    // Render tells the service which port to listen on through `PORT`. Bind to
    // 0.0.0.0 (every interface), not 127.0.0.1, or outside traffic can't reach us.
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(10000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let state = AppState { rooms: Registry::default(), started: Instant::now() };

    let app = Router::new()
        .route("/", get(|| async { GREETING }))
        // Any page may ask (the game is on another domain, GitHub Pages).
        .route("/health", get(|| async { ([(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")], "ok") }))
        .route("/ws", get(ws_upgrade))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind the port");
    println!("{GREETING} listening on {addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

/// Accepts the HTTP request that asks to become a WebSocket. The connection
/// itself is handled by `connection` once the upgrade completes.
async fn ws_upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.max_message_size(MAX_MESSAGE_BYTES).on_upgrade(move |socket| connection(socket, state))
}

/// A fresh, unpredictable 64-bit number (room seeds, tokens, codes).
///
/// The standard library has no random number API, but `RandomState` (what
/// `HashMap` uses to resist attacks) is seeded randomly by the OS. Hashing a
/// counter and the time with it gives numbers no client can guess, without a
/// `rand` dependency.
fn random_u64() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    RandomState::new().hash_one((COUNTER.fetch_add(1, Ordering::Relaxed), SystemTime::now()))
}

fn new_code(rooms: &HashMap<String, mpsc::UnboundedSender<RoomEvent>>) -> String {
    loop {
        let mut n = random_u64();
        let code: String = (0..CODE_LEN)
            .map(|_| {
                let c = CODE_LETTERS[(n % CODE_LETTERS.len() as u64) as usize];
                n /= CODE_LETTERS.len() as u64;
                c as char
            })
            .collect();
        if !rooms.contains_key(&code) {
            return code;
        }
    }
}

/// The sending half of one connection: numbers each message (see [`Frame`]).
struct Sender {
    next: u32,
}

impl Sender {
    async fn send(&mut self, socket: &mut WebSocket, msg: &ServerMsg) -> bool {
        let frame = Frame { n: self.next, msg: msg.clone() };
        self.next += 1;
        socket.send(Message::Binary(encode(&frame).into())).await.is_ok()
    }
}

/// One client's connection, from Hello to goodbye.
async fn connection(mut socket: WebSocket, state: AppState) {
    static NEXT_CONN: AtomicU64 = AtomicU64::new(1);
    let conn: ConnId = NEXT_CONN.fetch_add(1, Ordering::Relaxed);
    let mut out = Sender { next: 0 };

    // 1. Hello: who are you, and which room?
    let hello = tokio::time::timeout(HELLO_TIMEOUT, socket.recv()).await;
    let Ok(Some(Ok(Message::Binary(bytes)))) = hello else { return };
    let (name, intent) = match decode::<ClientMsg>(&bytes) {
        Ok(ClientMsg::Hello { version, name, intent }) if version == PROTOCOL_VERSION => {
            (name, intent)
        }
        _ => {
            let reason = "this page is out of date - please reload it".to_string();
            out.send(&mut socket, &ServerMsg::Refused { reason }).await;
            return;
        }
    };
    let (room, rejoin) = match intent {
        Intent::Host => {
            let mut rooms = state.rooms.lock().expect("registry lock");
            let code = new_code(&rooms);
            let (tx, rx) = mpsc::unbounded_channel();
            rooms.insert(code.clone(), tx.clone());
            tokio::spawn(run_room(Room::new(code), rx, state.clone()));
            (Some(tx), None)
        }
        Intent::Join { code } | Intent::Rejoin { code, .. } if code.is_empty() => {
            let reason = "type a room code".to_string();
            out.send(&mut socket, &ServerMsg::Refused { reason }).await;
            return;
        }
        Intent::Join { code } => (lookup(&state, &code), None),
        Intent::Rejoin { code, token } => (lookup(&state, &code), Some(token)),
    };
    let Some(room) = room else {
        let reason = "no room with that code".to_string();
        out.send(&mut socket, &ServerMsg::Refused { reason }).await;
        return;
    };

    // 2. Ask the room to take us.
    let (tx, mut outbox) = mpsc::unbounded_channel();
    let (reply, accepted) = oneshot::channel();
    let join = RoomEvent::Join { conn, name, rejoin, tx, reply };
    let accepted = room.send(join).is_ok() && accepted.await.unwrap_or(false);
    if !accepted {
        // Pass on the room's reason, if it gave one.
        while let Ok(msg) = outbox.try_recv() {
            out.send(&mut socket, &msg).await;
        }
        if room.is_closed() {
            let reason = "no room with that code".to_string();
            out.send(&mut socket, &ServerMsg::Refused { reason }).await;
        }
        return;
    }

    // 3. Pass messages both ways until either side stops.
    loop {
        // `select!` waits on both at once and runs whichever is ready first;
        // the other future is dropped, which is why both may borrow `socket`.
        tokio::select! {
            incoming = socket.recv() => {
                let Some(Ok(message)) = incoming else { break };
                match message {
                    Message::Binary(bytes) => {
                        if let Ok(msg) = decode::<ClientMsg>(&bytes)
                            && room.send(RoomEvent::Msg { conn, msg }).is_err()
                        {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    // axum answers pings itself; text frames aren't part of the protocol.
                    _ => {}
                }
            }
            outgoing = outbox.recv() => {
                let Some(msg) = outgoing else { break };
                let kicked = msg == ServerMsg::Kicked;
                if !out.send(&mut socket, &msg).await || kicked {
                    break;
                }
            }
        }
    }
    let _ = room.send(RoomEvent::Gone { conn });
}

fn lookup(state: &AppState, code: &str) -> Option<mpsc::UnboundedSender<RoomEvent>> {
    let code = battler_proto::clean_code(code);
    state.rooms.lock().expect("registry lock").get(&code).cloned()
}

/// A room's task: handles its events one at a time, and wakes up for the
/// round clock. Ends (and leaves the registry) once the room is dead.
async fn run_room(mut room: Room, mut events: mpsc::UnboundedReceiver<RoomEvent>, state: AppState) {
    let mut conns: HashMap<ConnId, mpsc::UnboundedSender<ServerMsg>> = HashMap::new();
    let now = || state.started.elapsed().as_millis() as u64;
    loop {
        // Sleep until the next deadline, or a while if there is none (then
        // check whether the room has died).
        let wait = room.next_deadline().map_or(30_000, |d| d.saturating_sub(now()));
        let out = tokio::select! {
            event = events.recv() => {
                let Some(event) = event else { break };
                match event {
                    RoomEvent::Join { conn, name, rejoin, tx, reply } => {
                        let result = match rejoin {
                            Some(token) => room.rejoin(conn, token, now()),
                            None => room.join(conn, &name, random_u64()),
                        };
                        match result {
                            Ok(out) => {
                                conns.insert(conn, tx);
                                let _ = reply.send(true);
                                out
                            }
                            Err(reason) => {
                                let _ = tx.send(ServerMsg::Refused { reason });
                                let _ = reply.send(false);
                                Vec::new()
                            }
                        }
                    }
                    RoomEvent::Msg { conn, msg } => room.handle(conn, msg, now(), random_u64()),
                    RoomEvent::Gone { conn } => {
                        conns.remove(&conn);
                        room.disconnect(conn, false, now())
                    }
                }
            }
            () = tokio::time::sleep(Duration::from_millis(wait)) => room.tick(now()),
        };
        for (conn, msg) in out {
            if let Some(tx) = conns.get(&conn) {
                let _ = tx.send(msg);
            }
        }
        if room.is_dead(now()) {
            break;
        }
    }
    state.rooms.lock().expect("registry lock").remove(room.code());
}

/// Resolves when the process is asked to stop. Render sends SIGTERM before a
/// redeploy; finishing cleanly lets open connections close properly.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("listen for ctrl-c");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    // `select!` waits for whichever future finishes first.
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
}

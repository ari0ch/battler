//! One room: a waiting list of players, then a game.
//!
//! This is a plain state machine with no networking and no clock of its own:
//! the caller passes in each event with the current time (in milliseconds)
//! and sends out the messages that come back. That keeps every rule here easy
//! to test (see the tests at the bottom), with time simply made up.
//!
//! During a game the room holds the authoritative [`Lobby`] and decides the
//! order of [`Step`]s; clients replay the same steps on their own copies.

use std::collections::BTreeSet;

use battler_proto::{ClientMsg, MAX_PLAYERS, Phase, SeatInfo, ServerMsg, clean_name};
use battler_sim::run::{Lobby, PlayerId, Seat, Step, timing};

/// Identifies one WebSocket connection.
pub type ConnId = u64;

/// A message to send to one connection.
pub type Out = (ConnId, ServerMsg);

/// Rounds a disconnected player's board keeps fighting on its own before a
/// bot takes the seat over.
pub const ROUNDS_BEFORE_BOT: u32 = 3;

/// Added to the longest battle before resolving, to cover network delay and
/// slow frames on the clients.
pub const BATTLE_SLACK_MS: u32 = 750;

/// How long a room with nobody connected is kept, in case they come back.
pub const EMPTY_ROOM_MS: u64 = 5 * 60 * 1000;

struct Member {
    /// The connection this member is on (meaningless while disconnected).
    conn: ConnId,
    name: String,
    /// Proves who you are when rejoining.
    token: u64,
    connected: bool,
    /// Rounds resolved while disconnected (reset on rejoin).
    missed: u32,
}

/// A game in progress. Member `i` plays as `PlayerId(i)`.
struct Game {
    seed: u64,
    seats: Vec<Seat>,
    lobby: Lobby,
    log: Vec<Step>,
    phase: Phase,
    /// When the current phase ends (ms, same clock as `now`).
    deadline: u64,
    ready: BTreeSet<PlayerId>,
}

pub struct Room {
    code: String,
    members: Vec<Member>,
    /// Index into `members` of the host (while waiting).
    host: usize,
    game: Option<Game>,
    /// When the last member disconnected, if nobody is connected now.
    empty_since: Option<u64>,
}

impl Room {
    pub fn new(code: String) -> Self {
        Self { code, members: Vec::new(), host: 0, game: None, empty_since: None }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    /// True once the room can be thrown away: nobody has been connected for
    /// [`EMPTY_ROOM_MS`], or nobody ever joined.
    pub fn is_dead(&self, now: u64) -> bool {
        self.members.is_empty()
            || self.empty_since.is_some_and(|since| now.saturating_sub(since) >= EMPTY_ROOM_MS)
    }

    /// When [`Room::tick`] next has something to do.
    pub fn next_deadline(&self) -> Option<u64> {
        self.game.as_ref().filter(|g| g.phase != Phase::Over).map(|g| g.deadline)
    }

    // ----- Joining and leaving ---------------------------------------------

    /// A new player joins the waiting room. `token` is their secret for
    /// rejoining (the caller makes it up). Returns why not, if they can't.
    pub fn join(&mut self, conn: ConnId, name: &str, token: u64) -> Result<Vec<Out>, String> {
        if self.game.is_some() {
            return Err("that game has already started".into());
        }
        if self.members.len() >= MAX_PLAYERS {
            return Err("that room is full".into());
        }
        let name = self.unique_name(&clean_name(name));
        self.members.push(Member { conn, name, token, connected: true, missed: 0 });
        self.empty_since = None;
        let mut out = vec![(conn, ServerMsg::Welcome { code: self.code.clone(), token })];
        out.extend(self.room_messages());
        Ok(out)
    }

    /// A player comes back on a new connection.
    pub fn rejoin(&mut self, conn: ConnId, token: u64, now: u64) -> Result<Vec<Out>, String> {
        let Some(i) = self.members.iter().position(|m| m.token == token) else {
            return Err("that game has no seat for you any more".into());
        };
        let player = PlayerId(i);
        let mut out = Vec::new();
        // Copy out what we need first: `push_step` borrows the whole room
        // mutably, which it couldn't while a borrow of `self.game` was alive.
        if let Some((true, phase)) = self.game.as_ref().map(|g| (g.lobby.is_bot(player), g.phase))
            && phase != Phase::Over
        {
            // Back from a bot: the seat is theirs again. This goes to the
            // others now, and to the returning player inside the full log
            // below (they aren't marked connected yet, so they don't get it twice).
            out.extend(self.push_step(Step::SetBot { player, bot: false }));
        }
        let member = &mut self.members[i];
        member.conn = conn;
        member.connected = true;
        member.missed = 0;
        self.empty_since = None;
        out.push((conn, ServerMsg::Welcome { code: self.code.clone(), token }));
        if self.game.is_some() {
            out.push((conn, self.game_message(i, now)));
            out.extend(self.presence());
        } else {
            out.extend(self.room_messages());
        }
        Ok(out)
    }

    /// The connection dropped (or the player pressed Leave, with `left`).
    pub fn disconnect(&mut self, conn: ConnId, left: bool, now: u64) -> Vec<Out> {
        let Some(i) = self.members.iter().position(|m| m.conn == conn && m.connected) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if self.game.is_none() {
            // Waiting: just drop out of the list. The next player hosts if needed.
            self.members.remove(i);
            if self.host > i || self.host >= self.members.len() {
                self.host = self.host.saturating_sub(1);
            }
            out.extend(self.room_messages());
        } else {
            self.members[i].connected = false;
            let player = PlayerId(i);
            if left && self.game.as_ref().is_some_and(|g| g.phase != Phase::Over) {
                out.extend(self.push_step(Step::SetBot { player, bot: true }));
            }
            out.extend(self.presence());
            out.extend(self.maybe_start_round(now));
        }
        if self.members.iter().all(|m| !m.connected) {
            self.empty_since = Some(now);
        }
        out
    }

    // ----- Messages from players -------------------------------------------

    /// A message from a member's connection. `seed` is used if this starts the
    /// game (the caller makes it up, so tests can fix it).
    pub fn handle(&mut self, conn: ConnId, msg: ClientMsg, now: u64, seed: u64) -> Vec<Out> {
        let Some(i) = self.members.iter().position(|m| m.conn == conn && m.connected) else {
            return Vec::new();
        };
        match msg {
            ClientMsg::Start if self.game.is_none() && i == self.host => self.start(seed, now),
            ClientMsg::Kick { seat } if self.game.is_none() && i == self.host && seat != i => {
                if seat >= self.members.len() {
                    return Vec::new();
                }
                let kicked = self.members.remove(seat);
                if self.host > seat {
                    self.host -= 1;
                }
                let mut out = vec![(kicked.conn, ServerMsg::Kicked)];
                out.extend(self.room_messages());
                out
            }
            ClientMsg::Leave => self.disconnect(conn, true, now),
            ClientMsg::Command(command) => self.command(i, conn, command),
            ClientMsg::Ready => {
                let Some(game) = &mut self.game else { return Vec::new() };
                if game.phase != Phase::Planning || !game.ready.insert(PlayerId(i)) {
                    return Vec::new();
                }
                let mut out = self.presence();
                out.extend(self.maybe_start_round(now));
                out
            }
            ClientMsg::Resync if self.game.is_some() => vec![(conn, self.game_message(i, now))],
            // Hello is handled before a connection reaches a room; Ping only
            // keeps the connection busy; the rest don't apply right now.
            _ => Vec::new(),
        }
    }

    fn start(&mut self, seed: u64, now: u64) -> Vec<Out> {
        let seats: Vec<Seat> = self.members.iter().map(|m| Seat::human(m.name.clone())).collect();
        let lobby = Lobby::with_seats(seed, &seats);
        let humans: Vec<PlayerId> = (0..seats.len()).map(PlayerId).collect();
        let deadline = now + u64::from(lobby.shared_planning_ms(&humans));
        self.game = Some(Game {
            seed,
            seats,
            lobby,
            log: Vec::new(),
            phase: Phase::Planning,
            deadline,
            ready: BTreeSet::new(),
        });
        let mut out: Vec<Out> = (0..self.members.len())
            .filter(|&i| self.members[i].connected)
            .map(|i| (self.members[i].conn, self.game_message(i, now)))
            .collect();
        out.extend(self.presence());
        out
    }

    fn command(&mut self, i: usize, conn: ConnId, command: battler_sim::run::Command) -> Vec<Out> {
        let Some(game) = &mut self.game else { return Vec::new() };
        let step = Step::Command { player: PlayerId(i), command };
        // Try it on the real lobby: only steps that worked go in the log.
        match game.lobby.step(&step) {
            Ok(_) => {
                game.log.push(step.clone());
                let seq = game.log.len() as u32;
                self.broadcast(ServerMsg::Step { seq, step })
            }
            Err(e) => vec![(conn, ServerMsg::Rejected { reason: e.to_string() })],
        }
    }

    // ----- The round clock -------------------------------------------------

    /// Moves the game on if its deadline has passed.
    pub fn tick(&mut self, now: u64) -> Vec<Out> {
        let Some(game) = &self.game else { return Vec::new() };
        if game.phase == Phase::Over || now < game.deadline {
            return Vec::new();
        }
        let phase = game.phase;
        match phase {
            Phase::Planning => self.start_round(now),
            Phase::Battle => self.resolve(now),
            Phase::Over => Vec::new(),
        }
    }

    /// Starts the round early once every human who could still act is ready.
    fn maybe_start_round(&mut self, now: u64) -> Vec<Out> {
        let Some(game) = &self.game else { return Vec::new() };
        if game.phase != Phase::Planning {
            return Vec::new();
        }
        let waiting_on = self.members.iter().enumerate().any(|(i, m)| {
            let p = PlayerId(i);
            m.connected
                && game.lobby.player(p).is_alive()
                && !game.lobby.is_bot(p)
                && !game.ready.contains(&p)
        });
        if waiting_on { Vec::new() } else { self.start_round(now) }
    }

    fn start_round(&mut self, now: u64) -> Vec<Out> {
        let mut out = self.push_step(Step::StartRound);
        let game = self.game.as_mut().expect("in a game");
        let ticks = game.lobby.longest_battle_ticks().unwrap_or(0);
        let ms = timing::battle_ms(ticks) + BATTLE_SLACK_MS;
        game.phase = Phase::Battle;
        game.deadline = now + u64::from(ms);
        out.extend(self.broadcast(ServerMsg::Clock { phase: Phase::Battle, ms_left: ms }));
        out
    }

    fn resolve(&mut self, now: u64) -> Vec<Out> {
        let mut out = self.push_step(Step::Resolve);
        let game = self.game.as_ref().expect("in a game");
        let check = ServerMsg::Check { seq: game.log.len() as u32, hash: game.lobby.state_hash() };
        out.extend(self.broadcast(check));
        if game.lobby.is_over() {
            self.game.as_mut().expect("in a game").phase = Phase::Over;
            out.extend(self.broadcast(ServerMsg::Clock { phase: Phase::Over, ms_left: 0 }));
            return out;
        }
        // Players away for too long: a bot takes over (and plans at once).
        for i in 0..self.members.len() {
            let player = PlayerId(i);
            let lobby = &self.game.as_ref().expect("in a game").lobby;
            if self.members[i].connected || lobby.is_bot(player) || !lobby.player(player).is_alive()
            {
                continue;
            }
            self.members[i].missed += 1;
            if self.members[i].missed >= ROUNDS_BEFORE_BOT {
                out.extend(self.push_step(Step::SetBot { player, bot: true }));
            }
        }
        let game = self.game.as_mut().expect("in a game");
        let humans: Vec<PlayerId> =
            (0..game.seats.len()).map(PlayerId).filter(|&p| !game.lobby.is_bot(p)).collect();
        let ms = game.lobby.shared_planning_ms(&humans);
        game.phase = Phase::Planning;
        game.deadline = now + u64::from(ms);
        game.ready.clear();
        out.extend(self.broadcast(ServerMsg::Clock { phase: Phase::Planning, ms_left: ms }));
        out.extend(self.presence());
        out
    }

    // ----- Helpers ---------------------------------------------------------

    /// Applies a step to the server's lobby, logs it and sends it to everyone.
    fn push_step(&mut self, step: Step) -> Vec<Out> {
        let game = self.game.as_mut().expect("in a game");
        if game.lobby.step(&step).is_err() {
            return Vec::new();
        }
        game.log.push(step.clone());
        let seq = game.log.len() as u32;
        self.broadcast(ServerMsg::Step { seq, step })
    }

    fn broadcast(&self, msg: ServerMsg) -> Vec<Out> {
        self.members.iter().filter(|m| m.connected).map(|m| (m.conn, msg.clone())).collect()
    }

    /// The full game for member `i`, to build or rebuild their lobby.
    fn game_message(&self, i: usize, now: u64) -> ServerMsg {
        let game = self.game.as_ref().expect("in a game");
        ServerMsg::Game {
            seed: game.seed,
            seats: game.seats.clone(),
            me: PlayerId(i),
            log: game.log.clone(),
            phase: game.phase,
            ms_left: game.deadline.saturating_sub(now).min(u64::from(u32::MAX)) as u32,
        }
    }

    fn presence_message(&self) -> ServerMsg {
        let ready =
            self.game.as_ref().map(|g| g.ready.iter().copied().collect()).unwrap_or_default();
        ServerMsg::Presence { ready, connected: self.members.iter().map(|m| m.connected).collect() }
    }

    fn presence(&self) -> Vec<Out> {
        self.broadcast(self.presence_message())
    }

    /// The waiting room as each member sees it (it says which seat is theirs).
    fn room_messages(&self) -> Vec<Out> {
        let seats: Vec<SeatInfo> = self
            .members
            .iter()
            .map(|m| SeatInfo { name: m.name.clone(), connected: m.connected })
            .collect();
        self.members
            .iter()
            .enumerate()
            .filter(|(_, m)| m.connected)
            .map(|(you, m)| {
                let msg = ServerMsg::Room {
                    code: self.code.clone(),
                    seats: seats.clone(),
                    host: self.host,
                    you,
                };
                (m.conn, msg)
            })
            .collect()
    }

    /// `name`, or `name 2`, `name 3`... if someone already has it.
    fn unique_name(&self, name: &str) -> String {
        let taken = |n: &str| self.members.iter().any(|m| m.name == n);
        if !taken(name) {
            return name.to_string();
        }
        (2..).map(|k| format!("{name} {k}")).find(|n| !taken(n)).expect("some number is free")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battler_sim::run::Command;

    /// Messages for one connection.
    fn to(out: &[Out], conn: ConnId) -> Vec<&ServerMsg> {
        out.iter().filter(|(c, _)| *c == conn).map(|(_, m)| m).collect()
    }

    fn room_with(names: &[&str]) -> Room {
        let mut room = Room::new("BCDF".into());
        for (i, name) in names.iter().enumerate() {
            room.join(i as ConnId, name, 100 + i as u64).unwrap();
        }
        room
    }

    /// A client's copy of the game, kept up to date from the messages.
    struct Client {
        lobby: Option<Lobby>,
        seq: u32,
    }

    impl Client {
        fn new() -> Self {
            Self { lobby: None, seq: 0 }
        }
        fn take(&mut self, msgs: Vec<&ServerMsg>) {
            for msg in msgs {
                match msg {
                    ServerMsg::Game { seed, seats, log, .. } => {
                        let mut lobby = Lobby::with_seats(*seed, seats);
                        lobby.replay(log);
                        self.seq = log.len() as u32;
                        self.lobby = Some(lobby);
                    }
                    ServerMsg::Step { seq, step } => {
                        assert_eq!(*seq, self.seq + 1, "no gaps");
                        self.seq = *seq;
                        self.lobby.as_mut().unwrap().step(step).unwrap();
                    }
                    ServerMsg::Check { seq, hash } => {
                        assert_eq!(*seq, self.seq);
                        assert_eq!(self.lobby.as_ref().unwrap().state_hash(), *hash, "in sync");
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn joining_names_seats_and_hosts() {
        let mut room = room_with(&["Ari", "Ari"]);
        let out = room.join(2, "  Bea ", 7).unwrap();
        let seats = match to(&out, 2).last() {
            Some(ServerMsg::Room { seats, host: 0, you: 2, .. }) => seats.clone(),
            other => panic!("expected the room, got {other:?}"),
        };
        let names: Vec<&str> = seats.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Ari", "Ari 2", "Bea"]);
        // The host leaves: the next player hosts.
        let out = room.disconnect(0, false, 0);
        assert!(matches!(to(&out, 1).last(), Some(ServerMsg::Room { host: 0, you: 0, .. })));
    }

    #[test]
    fn only_the_host_starts_and_late_joiners_are_turned_away() {
        let mut room = room_with(&["Ari", "Bea"]);
        assert!(room.handle(1, ClientMsg::Start, 0, 5).is_empty(), "not the host");
        let out = room.handle(0, ClientMsg::Start, 0, 5);
        assert!(matches!(to(&out, 1)[0], ServerMsg::Game { me: PlayerId(1), .. }));
        assert!(room.join(9, "Cal", 1).is_err());
    }

    #[test]
    fn a_full_game_keeps_every_client_in_sync() {
        let mut room = room_with(&["Ari", "Bea"]);
        let mut clients = [Client::new(), Client::new()];
        let mut now = 0;
        let deliver = |out: Vec<Out>, clients: &mut [Client; 2]| {
            for (i, c) in clients.iter_mut().enumerate() {
                c.take(to(&out, i as ConnId));
            }
        };
        deliver(room.handle(0, ClientMsg::Start, now, 11), &mut clients);
        for _ in 0..40 {
            if room.next_deadline().is_none() {
                break;
            }
            for conn in 0..2 {
                for command in [Command::Buy(0), Command::Reroll, Command::Buy(1)] {
                    deliver(room.handle(conn, ClientMsg::Command(command), now, 0), &mut clients);
                }
                deliver(room.handle(conn, ClientMsg::Ready, now, 0), &mut clients);
            }
            // Both ready: the round started without waiting for the clock.
            assert!(room.game.as_ref().unwrap().phase != Phase::Planning);
            now = room.next_deadline().unwrap();
            deliver(room.tick(now), &mut clients);
        }
        let game = room.game.as_ref().unwrap();
        assert_eq!(game.phase, Phase::Over, "someone won within 40 rounds");
        for c in &clients {
            assert_eq!(c.lobby.as_ref().unwrap().state_hash(), game.lobby.state_hash());
        }
    }

    #[test]
    fn the_round_ends_when_the_longest_battle_does() {
        let mut room = room_with(&["Ari"]);
        room.handle(0, ClientMsg::Start, 0, 3);
        room.handle(0, ClientMsg::Command(Command::Buy(0)), 0, 0);
        let out = room.handle(0, ClientMsg::Ready, 1000, 0);
        let ticks = room.game.as_ref().unwrap().lobby.longest_battle_ticks().unwrap();
        let expect = timing::battle_ms(ticks) + BATTLE_SLACK_MS;
        assert!(
            out.iter()
                .any(|(_, m)| *m == ServerMsg::Clock { phase: Phase::Battle, ms_left: expect })
        );
        assert_eq!(room.next_deadline(), Some(1000 + u64::from(expect)));
        assert!(room.tick(1000 + u64::from(expect) - 1).is_empty(), "not yet");
        assert!(!room.tick(1000 + u64::from(expect)).is_empty());
    }

    #[test]
    fn the_planning_clock_starts_the_round_if_someone_dawdles() {
        let mut room = room_with(&["Ari", "Bea"]);
        room.handle(0, ClientMsg::Start, 0, 3);
        room.handle(0, ClientMsg::Ready, 10, 0);
        assert_eq!(room.game.as_ref().unwrap().phase, Phase::Planning, "Bea isn't ready");
        let deadline = room.next_deadline().unwrap();
        room.tick(deadline);
        assert_eq!(room.game.as_ref().unwrap().phase, Phase::Battle);
    }

    #[test]
    fn a_disconnected_player_gets_a_bot_after_three_rounds_and_can_come_back() {
        let mut room = room_with(&["Ari", "Bea"]);
        room.handle(0, ClientMsg::Start, 0, 9);
        room.disconnect(1, false, 0);
        let bea = PlayerId(1);
        let mut now = 0;
        for round in 1..=ROUNDS_BEFORE_BOT {
            // Only Ari is connected, so his Ready starts the round.
            room.handle(0, ClientMsg::Ready, now, 0);
            now = room.next_deadline().unwrap();
            room.tick(now);
            let is_bot = room.game.as_ref().unwrap().lobby.is_bot(bea);
            assert_eq!(is_bot, round == ROUNDS_BEFORE_BOT, "round {round}");
        }
        // Bea comes back with her token: the seat is hers again, and she gets
        // the whole log to catch up.
        let out = room.rejoin(5, 101, now).unwrap();
        assert!(!room.game.as_ref().unwrap().lobby.is_bot(bea));
        let mut bea_client = Client::new();
        bea_client.take(to(&out, 5));
        assert_eq!(
            bea_client.lobby.unwrap().state_hash(),
            room.game.as_ref().unwrap().lobby.state_hash()
        );
        assert!(room.rejoin(6, 12345, now).is_err(), "wrong token");
    }

    #[test]
    fn leaving_mid_game_hands_the_seat_to_a_bot_at_once() {
        let mut room = room_with(&["Ari", "Bea"]);
        room.handle(0, ClientMsg::Start, 0, 4);
        room.handle(1, ClientMsg::Leave, 0, 0);
        assert!(room.game.as_ref().unwrap().lobby.is_bot(PlayerId(1)));
    }

    #[test]
    fn refused_commands_only_go_to_the_sender() {
        let mut room = room_with(&["Ari", "Bea"]);
        room.handle(0, ClientMsg::Start, 0, 4);
        let out = room.handle(0, ClientMsg::Command(Command::Buy(99)), 0, 0);
        assert!(matches!(to(&out, 0)[..], [ServerMsg::Rejected { .. }]));
        assert!(to(&out, 1).is_empty());
    }

    #[test]
    fn empty_rooms_die_after_a_while() {
        let mut room = room_with(&["Ari"]);
        room.handle(0, ClientMsg::Start, 0, 4);
        room.disconnect(0, false, 1000);
        assert!(!room.is_dead(1000 + EMPTY_ROOM_MS - 1));
        assert!(room.is_dead(1000 + EMPTY_ROOM_MS));
    }
}

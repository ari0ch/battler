//! The messages Battler's client and multiplayer server exchange over a
//! WebSocket, one message per binary frame, encoded with
//! [postcard](https://docs.rs/postcard) (compact, and small in WASM).
//!
//! The protocol leans on the sim being deterministic. The server never sends
//! game state: it sends the lobby's seed and seats once, then the ordered
//! [`Step`] log, and every client applies the same steps to its own copy of the
//! lobby. A [`ServerMsg::Check`] after each round lets a client spot that its
//! copy drifted and ask for the log again.
//!
//! A connection goes:
//! 1. The client sends [`ClientMsg::Hello`] to host, join or rejoin a room.
//! 2. While the room waits, the server sends [`ServerMsg::Room`] on every change.
//! 3. The host sends [`ClientMsg::Start`]; everyone gets [`ServerMsg::Game`].
//! 4. During play: commands and Ready from clients; steps and clocks from the server.
//!
//! Every server message travels inside a numbered [`Frame`].

use battler_sim::run::{Command, PlayerId, Seat, Step};
use serde::{Deserialize, Serialize};

/// Bumped whenever a message changes shape, so an old page gets a clear
/// "please reload" instead of garbled messages.
pub const PROTOCOL_VERSION: u32 = 1;

/// Players per room (the lobby's size): empty seats are filled by bots.
pub const MAX_PLAYERS: usize = battler_sim::run::economy::LOBBY_SIZE;

/// The longest player name.
pub const MAX_NAME_LEN: usize = 12;

/// Letters used in room codes: no vowels (so no accidental words) and nothing
/// easily confused, like 0/O or 1/I.
pub const CODE_LETTERS: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";

/// Letters in a room code.
pub const CODE_LEN: usize = 4;

/// What the client wants from a new connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    /// Open a new room and be its host.
    Host,
    /// Take a free seat in a waiting room.
    Join { code: String },
    /// Come back to a game in progress after losing the connection.
    Rejoin { code: String, token: u64 },
}

/// Client to server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClientMsg {
    /// The first message on every connection. `version` comes first so that
    /// even a client with a different protocol can be told so.
    Hello { version: u32, name: String, intent: Intent },
    /// Host only, while waiting: start the game.
    Start,
    /// Host only, while waiting: remove a player (by seat index).
    Kick { seat: usize },
    /// Leave the room. In a game, a bot takes the seat straight away.
    Leave,
    /// A planning command for this player.
    Command(Command),
    /// Done planning this round.
    Ready,
    /// The state check failed: send the whole game again.
    Resync,
    /// Keeps the connection (and the free server) awake while nothing else is sent.
    Ping,
}

/// One seat as the waiting room shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatInfo {
    pub name: String,
    pub connected: bool,
}

/// What the round clock is counting down to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    /// Planning: the round starts when everyone is ready or time runs out.
    Planning,
    /// Battles are playing: the round resolves when the longest one ends.
    Battle,
    /// The game has finished.
    Over,
}

/// Server to client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerMsg {
    /// The Hello was accepted. Keep `token` (with the code) to rejoin later.
    Welcome { code: String, token: u64 },
    /// The Hello was refused, with a reason to show.
    Refused { reason: String },
    /// The waiting room: who's in it, who hosts, and which seat is yours.
    Room { code: String, seats: Vec<SeatInfo>, host: usize, you: usize },
    /// The host removed you.
    Kicked,
    /// Everything needed to build (or rebuild) the game: build a lobby with
    /// `Lobby::with_seats(seed, &seats)`, then replay `log`.
    Game {
        seed: u64,
        seats: Vec<Seat>,
        me: PlayerId,
        log: Vec<Step>,
        phase: Phase,
        /// Milliseconds left in `phase`.
        ms_left: u32,
    },
    /// The next step of the log. `seq` is the log's length after it, so a
    /// client can tell it missed nothing.
    Step { seq: u32, step: Step },
    /// One of your commands was refused (another player got there first).
    Rejected { reason: String },
    /// A new phase began, or the clock was corrected.
    Clock { phase: Phase, ms_left: u32 },
    /// Who has pressed Ready this planning phase, and which seats are online.
    Presence { ready: Vec<PlayerId>, connected: Vec<bool> },
    /// The server's lobby hash once the log reached `seq` steps.
    Check { seq: u32, hash: u64 },
}

/// How every server message travels: numbered per connection from 0, so the
/// client can put messages back in order. (A browser may hand binary messages
/// over out of order when it reads them asynchronously, and the step log only
/// works in order.)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    pub n: u32,
    pub msg: ServerMsg,
}

/// Encodes a message into one WebSocket frame.
pub fn encode<T: Serialize>(message: &T) -> Vec<u8> {
    postcard::to_allocvec(message).expect("in-memory encoding can't fail")
}

/// Decodes one WebSocket frame.
pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, postcard::Error> {
    postcard::from_bytes(bytes)
}

/// Tidies a player name: printable ASCII only (the game's fonts are ASCII),
/// trimmed, at most [`MAX_NAME_LEN`] characters, "Player" if nothing is left.
pub fn clean_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let cut: String = cleaned.chars().take(MAX_NAME_LEN).collect();
    let cut = cut.trim_end().to_string();
    if cut.is_empty() { "Player".to_string() } else { cut }
}

/// Upper-cases a typed room code and keeps only code letters.
pub fn clean_code(code: &str) -> String {
    code.chars()
        .map(|c| c.to_ascii_uppercase())
        .filter(|c| c.is_ascii() && CODE_LETTERS.contains(&(*c as u8)))
        .take(CODE_LEN)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use battler_sim::run::Location;
    use battler_sim::{Action, Condition, Node, Pos, Program, Target};

    #[test]
    fn messages_survive_a_round_trip() {
        let program =
            Program::new([Node::new(Condition::EnemyAdjacent, Action::Attack(Target::It))]);
        let msgs = [
            ClientMsg::Hello {
                version: PROTOCOL_VERSION,
                name: "Ari".into(),
                intent: Intent::Rejoin { code: "BCDF".into(), token: 99 },
            },
            ClientMsg::Command(Command::SetProgram(Location::Board(Pos::new(1, 2)), program)),
            ClientMsg::Command(Command::Equip { item: 0, to: Location::Bench(3), slot: Some(1) }),
            ClientMsg::Ready,
        ];
        for msg in msgs {
            let back: ClientMsg = decode(&encode(&msg)).unwrap();
            assert_eq!(back, msg);
        }
        let game = ServerMsg::Game {
            seed: 7,
            seats: vec![Seat::human("Ari")],
            me: PlayerId(0),
            log: vec![Step::StartRound, Step::SetBot { player: PlayerId(0), bot: true }],
            phase: Phase::Battle,
            ms_left: 1234,
        };
        let frame = Frame { n: 3, msg: game };
        let back: Frame = decode(&encode(&frame)).unwrap();
        assert_eq!(back, frame);
    }

    #[test]
    fn hello_starts_with_the_version() {
        // Variant 0, then the version as a varint: an old server can always
        // read at least this much.
        let bytes = encode(&ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: String::new(),
            intent: Intent::Host,
        });
        assert_eq!(bytes[0], 0);
        assert_eq!(bytes[1] as u32, PROTOCOL_VERSION);
    }

    #[test]
    fn names_and_codes_are_tidied() {
        assert_eq!(clean_name("  Ari  the   Bold "), "Ari the Bold");
        assert_eq!(clean_name("Ünïcode!"), "ncode!");
        assert_eq!(clean_name("   "), "Player");
        assert_eq!(clean_name("abcdefghijklmnopq"), "abcdefghijkl");
        assert_eq!(clean_code(" kqzt "), "KQZT");
        assert_eq!(clean_code("a1o0bcdfg"), "BCDF");
    }
}

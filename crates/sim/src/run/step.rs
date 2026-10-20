//! The step log: everything that happens to a lobby, in order.
//!
//! A lobby is a deterministic state machine. Given the same seed and seats,
//! and then the same [`Step`]s in the same order, every copy of it ends up in
//! exactly the same state. Multiplayer is built on that: the server decides
//! the order of steps and sends it to every client, and each client applies
//! the steps to its own copy. Rejoining a game is replaying the whole log.

use super::{Applied, Command, Lobby, PlayerId, RunError};

/// One entry in a lobby's log.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Step {
    /// A player's planning command (see [`Lobby::apply`]).
    Command { player: PlayerId, command: Command },
    /// Planning is over: lock in the round's battles ([`Lobby::start_round`]).
    StartRound,
    /// The battles are over: play them out and start the next planning phase
    /// ([`Lobby::resolve`]).
    Resolve,
    /// A seat changes hands between its human and a bot ([`Lobby::set_bot`]).
    SetBot { player: PlayerId, bot: bool },
}

impl Lobby {
    /// Applies one step. Commands report what they did; the other steps
    /// return `Ok(None)`.
    pub fn step(&mut self, step: &Step) -> Result<Option<Applied>, RunError> {
        match step {
            Step::Command { player, command } => self.apply(*player, command.clone()).map(Some),
            Step::StartRound => self.start_round().map(|()| None),
            Step::Resolve => {
                if !self.in_battle() {
                    return Err(RunError::NoBattle);
                }
                self.resolve();
                Ok(None)
            }
            Step::SetBot { player, bot } => {
                self.set_bot(*player, *bot);
                Ok(None)
            }
        }
    }

    /// Applies steps in order, skipping any that fail (a command the server
    /// refused is never in the log, so in practice none do).
    pub fn replay<'a>(&mut self, steps: impl IntoIterator<Item = &'a Step>) {
        for step in steps {
            let _ = self.step(step);
        }
    }
}

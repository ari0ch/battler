//! A whole run: a lobby of eight players fighting until one is left.
//!
//! - [`Pool`]: the character copies every shop draws from, shared by the lobby.
//! - [`Player`]: one player's health, gold, level, shop, bench and board.
//! - [`Lobby`]: the players, the pool, matchmaking and round resolution.
//! - [`bot`]: computer players. They change their [`Player`] only through
//!   [`Lobby::apply`], exactly like the human, so a bot's board is always a
//!   real, legal player board.
//!
//! The game crate drives the human player through [`Command`]s and only displays
//! the state. All randomness comes from the lobby's seed, so a lobby replays
//! identically from its seed plus the human's commands.

pub mod bot;
pub mod economy;
mod lobby;
mod player;
mod pool;
mod step;
pub mod timing;

#[cfg(test)]
mod tests;

use std::fmt;

use crate::program::ProgramError;
use crate::{Battle, Character, Class, Item, ItemSlots, Level, Pos, Program, Team, UnitSpec, Wave};
use economy::*;

pub use lobby::{Applied, Command, Lobby, RoundKind, Seat, round_kind};
pub use player::Player;
pub use pool::Pool;
pub use step::Step;

/// Identifies a player within a lobby (also their index in it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlayerId(pub usize);

/// Identifies one owned unit for the whole run, even as it moves around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnedId(pub u32);

/// A unit a player owns, on the board or the bench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedUnit {
    pub id: OwnedId,
    pub character: Character,
    pub level: Level,
    pub program: Program,
    /// Equipped items (see `item.rs`).
    pub items: ItemSlots,
}

impl OwnedUnit {
    pub fn class(&self) -> Class {
        self.character.class()
    }

    pub fn spec(&self) -> UnitSpec {
        UnitSpec::new(self.character, self.level, self.program.clone()).with_items(self.items)
    }

    /// Gold returned when sold: the price of every copy inside.
    pub fn sell_value(&self) -> u32 {
        self.level.copies() * self.character.rarity().cost()
    }
}

/// Where a unit sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Location {
    Bench(usize),
    Board(Pos),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Still in the game.
    Planning,
    /// Last one standing.
    Victory,
    /// Knocked out (see [`Player::placement`]).
    Defeat,
}

/// Who a player fought in a round.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opponent {
    /// A PvE round.
    Monsters(Wave),
    Player(PlayerId),
    /// A copy of this player's board (odd number of players left). Losing to a
    /// ghost costs no health.
    Ghost(PlayerId),
}

impl Opponent {
    pub fn is_pve(self) -> bool {
        matches!(self, Opponent::Monsters(_))
    }
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    /// This player is out, or the lobby has finished.
    RunOver,
    /// Commands only work while planning.
    BattleInProgress,
    NotEnoughGold {
        need: u32,
        have: u32,
    },
    NoOffer(usize),
    BenchFull,
    BoardFull {
        max: usize,
    },
    /// Only one copy of each character may be on the board.
    AlreadyOnBoard(Character),
    NotPlayerSide(Pos),
    Empty(Location),
    MaxLevel,
    InvalidProgram(ProgramError),
    /// No item at this stash index.
    NoItem(usize),
    /// Both of the unit's item slots are taken.
    ItemSlotsFull,
    /// Nothing equipped in that slot.
    EmptyItemSlot,
    /// There is no locked-in round to resolve.
    NoBattle,
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::RunOver => write!(f, "the run is over"),
            RunError::BattleInProgress => write!(f, "wait for the battle to finish"),
            RunError::NotEnoughGold { need, have } => write!(f, "need {need} gold, have {have}"),
            RunError::NoOffer(i) => write!(f, "shop slot {} is empty", i + 1),
            RunError::BenchFull => write!(f, "the bench is full"),
            RunError::BoardFull { max } => {
                write!(f, "the board holds {max} units at this level - buy XP to grow it")
            }
            RunError::AlreadyOnBoard(c) => write!(f, "{} is already on the board", c.name()),
            RunError::NotPlayerSide(_) => write!(f, "units can only go on your side"),
            RunError::Empty(_) => write!(f, "there's no unit there"),
            RunError::MaxLevel => write!(f, "already at the maximum level"),
            RunError::InvalidProgram(e) => write!(f, "{e}"),
            RunError::NoItem(_) => write!(f, "that item isn't in your stash"),
            RunError::ItemSlotsFull => write!(f, "both item slots are taken"),
            RunError::EmptyItemSlot => write!(f, "nothing equipped there"),
            RunError::NoBattle => write!(f, "no battle to resolve"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<ProgramError> for RunError {
    fn from(e: ProgramError) -> Self {
        RunError::InvalidProgram(e)
    }
}

/// Everything needed to (re)create one battle: both sides and the seed.
/// Stored per round, so any battle can be replayed exactly.
///
/// `player` is the home side (left half, [`Team::Player`]); `enemy` is the away
/// side, already mirrored onto the right half ([`Team::Enemy`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
    pub round: u32,
    pub seed: u64,
    pub player: Vec<(Pos, UnitSpec)>,
    pub enemy: Vec<(Pos, UnitSpec)>,
}

impl BattleSetup {
    /// Builds the battle. Units spawn alternating between the sides (home,
    /// away, home, ...), so neither side always acts first within a tick.
    pub fn build(&self) -> Battle {
        let mut battle = Battle::new(BOARD_DEPTH, BOARD_LANES, MAX_TICKS, self.seed);
        let longest = self.player.len().max(self.enemy.len());
        for i in 0..longest {
            let pair = [(Team::Player, self.player.get(i)), (Team::Enemy, self.enemy.get(i))];
            for (team, unit) in pair {
                if let Some((pos, spec)) = unit {
                    battle.spawn(team, *pos, spec.clone()).expect("setups only hold valid units");
                }
            }
        }
        battle
    }
}

/// What happened to one player in a round, for the result screen and summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundReport {
    pub round: u32,
    pub opponent: Opponent,
    /// The side this player fought on in the battle's setup.
    pub side: Team,
    /// `Some(true)` won, `Some(false)` lost, `None` drawn.
    pub result: Option<bool>,
    pub ticks: u32,
    pub player_survivors: usize,
    pub damage: u32,
    pub hp_after: u32,
    pub banked: u32,
    pub income: Income,
    pub gold_after: u32,
    /// Positive for a win streak, negative for a loss streak.
    pub streak: i32,
    /// Player levels reached thanks to this round's XP.
    pub level_ups: Vec<u32>,
    /// Set if this round ended the player's game (1 = won the lobby).
    pub placement: Option<u32>,
    /// Items dropped by the monsters, added to the stash.
    pub loot: Vec<Item>,
}

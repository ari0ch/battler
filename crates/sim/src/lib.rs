//! Battle simulation for Battler.
//!
//! The simulation advances in discrete ticks. Each tick, every living unit
//! (in ascending [`UnitId`] order) takes one turn: it checks its rule cards
//! top to bottom and runs the first one whose condition holds and whose action
//! can be carried out. If none fires, the locked default rule runs: attack the
//! nearest enemy, advancing toward it if it's out of range.
//!
//! Units are named [`Character`]s, each with a [`Class`] and [`Rarity`].
//! Fielding several of a class activates its synergy (see [`synergy`]).
//!
//! Battles are **reproducible**: randomness (crits) comes from a seeded [`Rng`]
//! stored in the [`Battle`], so the same units, programs and seed always play
//! out identically. Rules for keeping it that way:
//! - No floats in game logic; positions and stats are integers.
//! - No hash-ordered collections; iterate in a stable order.
//! - No wall-clock time or OS randomness; time is the tick counter.

mod battle;
mod class;
mod exec;
mod grid;
pub mod monster;
mod program;
mod rng;
pub mod run;
pub mod synergy;
mod unit;

#[cfg(test)]
mod tests;

pub use battle::{
    Battle, CRIT_CHANCE_PERCENT, CRIT_DAMAGE_PERCENT, Event, Outcome, SpawnError, StatusKind,
};
pub use class::{Ability, Character, Class, Level, Passive, Rarity, Stats};
pub use grid::Pos;
pub use monster::{Monster, Wave};
pub use program::{
    Action, CardRef, Condition, MAX_CARDS, Node, Program, ProgramError, Requirement, Scope, Signal,
    Target, Threshold, Unlocks, WEAK_PERCENT, palette_actions, palette_conditions,
};
pub use rng::Rng;
pub use synergy::Synergies;
pub use unit::{Statuses, Team, Unit, UnitId, UnitKind, UnitSpec};

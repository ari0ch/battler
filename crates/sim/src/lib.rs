//! Battle simulation for Battler.
//!
//! The simulation advances in discrete ticks. Each tick, every living unit
//! (in ascending [`UnitId`] order) takes one turn: it checks its rule cards
//! top to bottom and runs the first one whose condition holds and whose action
//! can be carried out. If none fires, the locked default rule runs: attack the
//! nearest enemy, advancing toward it if it's out of range.
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
mod program;
mod rng;
pub mod run;
mod unit;

#[cfg(test)]
mod tests;

pub use battle::{
    Battle, CRIT_CHANCE_PERCENT, CRIT_DAMAGE_PERCENT, Event, Outcome, SpawnError, StatusKind,
};
pub use class::{Ability, Class, Level, Stats};
pub use grid::Pos;
pub use program::{
    Action, CardRef, Condition, Node, Program, ProgramError, Signal, Target, Threshold,
};
pub use rng::Rng;
pub use unit::{Statuses, Team, Unit, UnitId, UnitSpec};

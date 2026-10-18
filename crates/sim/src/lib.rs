//! Deterministic battle simulation for Battler.
//!
//! The simulation advances in discrete ticks. Each tick, every living unit
//! (in ascending [`UnitId`] order) executes one instruction from its
//! [`Program`]. Programs loop when they reach the end.
//!
//! Rules for keeping it deterministic:
//! - No floats in game logic; positions and stats are integers.
//! - No hash-ordered collections; iterate in a stable order.
//! - No wall-clock time or OS randomness; time is the tick counter, and any
//!   randomness must come from a seeded RNG stored in the battle.

mod battle;
mod grid;
mod program;
mod unit;

pub use battle::{Battle, Event, Outcome};
pub use grid::Pos;
pub use program::{Instruction, Program};
pub use unit::{Team, Unit, UnitId, UnitStats};

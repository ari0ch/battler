use crate::{Pos, Program};

/// Stable identifier for a unit within one battle. Also defines turn order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    Player,
    Enemy,
}

impl Team {
    pub fn opponent(self) -> Team {
        match self {
            Team::Player => Team::Enemy,
            Team::Enemy => Team::Player,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitStats {
    pub max_hp: u32,
    pub attack: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub id: UnitId,
    pub team: Team,
    pub pos: Pos,
    pub stats: UnitStats,
    pub hp: u32,
    pub program: Program,
    /// Index of the next instruction to run (wraps around the program).
    pub pc: usize,
    pub guarding: bool,
}

impl Unit {
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }
}

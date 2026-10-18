use crate::{Ability, Class, Level, Pos, Program, Stats};

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

    pub(crate) fn index(self) -> usize {
        match self {
            Team::Player => 0,
            Team::Enemy => 1,
        }
    }
}

/// What the player chooses for a unit before a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSpec {
    pub class: Class,
    pub level: Level,
    pub program: Program,
}

impl UnitSpec {
    pub fn new(class: Class, level: Level, program: Program) -> Self {
        Self { class, level, program }
    }
}

/// Timed effects on a unit. Each `*_until` field holds the last tick on which
/// the effect is active (0 = never set).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Statuses {
    /// Half damage until this unit's next turn.
    pub guarding: bool,
    pub shielded_until: u32,
    pub immune_until: u32,
    pub frenzied_until: u32,
    /// Forced to target this unit while it is alive.
    pub taunted_by: Option<(UnitId, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub id: UnitId,
    pub team: Team,
    pub class: Class,
    pub level: Level,
    pub stats: Stats,
    pub pos: Pos,
    pub hp: u32,
    pub program: Program,
    pub statuses: Statuses,
    /// Tick from which each ability may be used again, indexed by [`Ability::index`].
    pub(crate) ready_at: [u32; Ability::COUNT],
}

impl Unit {
    pub fn is_alive(&self) -> bool {
        self.hp > 0
    }

    pub fn is_ranged(&self) -> bool {
        self.stats.range > 1
    }

    /// HP as a percentage of max, rounded down.
    pub fn hp_percent(&self) -> u32 {
        self.hp * 100 / self.stats.max_hp
    }

    pub fn ability_ready(&self, ability: Ability, tick: u32) -> bool {
        tick >= self.ready_at[ability.index()]
    }
}

use crate::{Ability, Character, Class, ItemSlots, Level, Monster, Passive, Pos, Program, Stats};

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

/// What a unit is: one of the named characters, or a monster.
///
/// Most of the battle code only needs a unit's class, stats, abilities and
/// passive, which both kinds provide, so it never has to match on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnitKind {
    Character(Character),
    Monster(Monster),
}

impl UnitKind {
    pub fn name(self) -> &'static str {
        match self {
            UnitKind::Character(c) => c.name(),
            UnitKind::Monster(m) => m.name(),
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            UnitKind::Character(c) => c.short_name(),
            UnitKind::Monster(m) => m.short_name(),
        }
    }

    pub fn class(self) -> Class {
        match self {
            UnitKind::Character(c) => c.class(),
            UnitKind::Monster(m) => m.class(),
        }
    }

    pub fn base_stats(self) -> Stats {
        match self {
            UnitKind::Character(c) => c.base_stats(),
            UnitKind::Monster(m) => m.base_stats(),
        }
    }

    pub fn abilities(self) -> &'static [Ability] {
        match self {
            UnitKind::Character(c) => c.abilities(),
            UnitKind::Monster(m) => m.abilities(),
        }
    }

    pub fn has_ability(self, ability: Ability) -> bool {
        self.abilities().contains(&ability)
    }

    pub fn passive(self) -> Option<Passive> {
        match self {
            UnitKind::Character(c) => c.passive(),
            UnitKind::Monster(m) => m.passive(),
        }
    }

    /// The character, if this is one (only characters count for synergies).
    pub fn character(self) -> Option<Character> {
        match self {
            UnitKind::Character(c) => Some(c),
            UnitKind::Monster(_) => None,
        }
    }

    pub fn monster(self) -> Option<Monster> {
        match self {
            UnitKind::Monster(m) => Some(m),
            UnitKind::Character(_) => None,
        }
    }
}

// These `From` impls let any function taking `impl Into<UnitKind>` accept a
// `Character` or a `Monster` directly.
impl From<Character> for UnitKind {
    fn from(c: Character) -> Self {
        UnitKind::Character(c)
    }
}

impl From<Monster> for UnitKind {
    fn from(m: Monster) -> Self {
        UnitKind::Monster(m)
    }
}

/// Everything needed to spawn a unit in a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitSpec {
    pub kind: UnitKind,
    pub level: Level,
    pub program: Program,
    /// Extra HP and attack in percent, on top of level scaling (later bosses).
    pub boost_percent: u32,
    /// Equipped items (see `item.rs`).
    pub items: ItemSlots,
}

impl UnitSpec {
    pub fn new(kind: impl Into<UnitKind>, level: Level, program: Program) -> Self {
        Self {
            kind: kind.into(),
            level,
            program,
            boost_percent: 0,
            items: [None; crate::ITEM_SLOTS],
        }
    }

    pub fn with_items(mut self, items: ItemSlots) -> Self {
        self.items = items;
        self
    }
}

/// One-shot item effects already used up this battle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ItemsSpent {
    pub draught: bool,
    pub whetstone: bool,
    pub phoenix: bool,
}

/// Timed and battle-long effects on a unit. Each `*_until` field holds the last
/// tick on which the effect is active (0 = never set).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Statuses {
    /// Half damage until this unit's next turn.
    pub guarding: bool,
    pub shielded_until: u32,
    pub immune_until: u32,
    pub frenzied_until: u32,
    /// Can't be targeted by enemies (Vanish).
    pub untargetable_until: u32,
    /// Forced to target this unit while it is alive.
    pub taunted_by: Option<(UnitId, u32)>,
    /// Turns to skip (Frost Nova), counted from the tick after `frozen_on`.
    pub frozen_turns: u8,
    /// The tick the unit was frozen on.
    pub frozen_on: u32,
    /// Hit by Kestrel: takes 15% more damage for the rest of the battle.
    pub hunters_marked: bool,
    /// The last tick this unit took damage (0 = never).
    pub damaged_on: u32,
    /// Who dealt that damage: `was hit -> Attack it` strikes back at them.
    pub hit_by: Option<UnitId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub id: UnitId,
    pub team: Team,
    pub kind: UnitKind,
    pub class: Class,
    pub level: Level,
    pub stats: Stats,
    pub pos: Pos,
    pub hp: u32,
    pub program: Program,
    pub statuses: Statuses,
    /// Chance of a critical hit, in percent.
    pub crit_chance: u32,
    /// Equipped items.
    pub items: ItemSlots,
    /// One-shot item effects already used.
    pub items_spent: ItemsSpent,
    /// Tick from which each ability may be used again, indexed by [`Ability::index`].
    pub(crate) ready_at: [u32; Ability::COUNT],
    /// Abilities used so far (for Arcane Surge).
    pub(crate) abilities_used: u32,
    /// Undying has been used up.
    pub(crate) cheated_death: bool,
    /// Test-only: the unit never takes a turn (a target dummy).
    #[cfg(test)]
    pub(crate) inert: bool,
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

    pub fn has_passive(&self, passive: Passive) -> bool {
        self.kind.passive() == Some(passive)
    }

    pub fn has_item(&self, item: crate::Item) -> bool {
        crate::item::has(&self.items, item)
    }

    /// True if hit during the previous tick or this one.
    pub fn recently_hit(&self, tick: u32) -> bool {
        self.statuses.damaged_on > 0 && self.statuses.damaged_on + 1 >= tick
    }
}

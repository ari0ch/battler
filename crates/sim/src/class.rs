//! Unit classes, levels, and signature abilities.

/// A unit's class: its stats and which signature abilities it brings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    Knight,
    Ranger,
    Mage,
    Cleric,
    Rogue,
    Berserker,
}

impl Class {
    pub const ALL: [Class; 6] =
        [Class::Knight, Class::Ranger, Class::Mage, Class::Cleric, Class::Rogue, Class::Berserker];

    /// Level 1 stats. Higher levels scale HP and attack (see [`Level::scale`]).
    pub fn base_stats(self) -> Stats {
        let (max_hp, attack, range) = match self {
            Class::Knight => (600, 40, 1),
            Class::Ranger => (350, 55, 3),
            Class::Mage => (350, 45, 3),
            Class::Cleric => (400, 25, 2),
            Class::Rogue => (400, 60, 1),
            Class::Berserker => (550, 50, 1),
        };
        Stats { max_hp, attack, range }
    }

    /// The two signature abilities this class adds to its block palette.
    pub fn abilities(self) -> [Ability; 2] {
        match self {
            Class::Knight => [Ability::Taunt, Ability::ShieldWall],
            Class::Ranger => [Ability::AimedShot, Ability::Kite],
            Class::Mage => [Ability::Fireball, Ability::Blink],
            Class::Cleric => [Ability::Heal, Ability::Sanctuary],
            Class::Rogue => [Ability::Backstab, Ability::Shadowstep],
            Class::Berserker => [Ability::Frenzy, Ability::Cleave],
        }
    }

    pub fn has_ability(self, ability: Ability) -> bool {
        self.abilities().contains(&ability)
    }

    /// Units with a range above 1 count as ranged for targeting purposes.
    pub fn is_ranged(self) -> bool {
        self.base_stats().range > 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub max_hp: u32,
    pub attack: u32,
    /// Basic attack range in grid (Manhattan) distance.
    pub range: u32,
}

/// Unit level, raised by merging three copies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    One,
    Two,
    Three,
}

impl Level {
    /// How many rule cards (nodes) a unit of this level can hold.
    pub fn max_nodes(self) -> usize {
        match self {
            Level::One => 2,
            Level::Two => 3,
            Level::Three => 4,
        }
    }

    /// Stat multiplier in percent (×1.8 per level, as in TFT).
    pub fn scale_percent(self) -> u32 {
        match self {
            Level::One => 100,
            Level::Two => 180,
            Level::Three => 324,
        }
    }

    /// Applies this level's scaling to a class's base stats.
    pub fn scale(self, base: Stats) -> Stats {
        let pct = self.scale_percent();
        Stats { max_hp: base.max_hp * pct / 100, attack: base.attack * pct / 100, ..base }
    }
}

/// A class signature ability. Each has a cooldown in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ability {
    Taunt,
    ShieldWall,
    AimedShot,
    Kite,
    Fireball,
    Blink,
    Heal,
    Sanctuary,
    Backstab,
    Shadowstep,
    Frenzy,
    Cleave,
}

impl Ability {
    pub const COUNT: usize = 12;

    /// Ticks before the ability can be used again.
    pub fn cooldown(self) -> u32 {
        match self {
            Ability::Taunt => 6,
            Ability::ShieldWall => 6,
            Ability::AimedShot => 4,
            Ability::Kite => 2,
            Ability::Fireball => 5,
            Ability::Blink => 6,
            Ability::Heal => 3,
            Ability::Sanctuary => 8,
            Ability::Backstab => 4,
            Ability::Shadowstep => 5,
            Ability::Frenzy => 8,
            Ability::Cleave => 3,
        }
    }

    /// Dense index, for storing per-ability state in a fixed-size array.
    pub fn index(self) -> usize {
        self as usize
    }
}

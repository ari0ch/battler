//! Classes, characters, rarities, levels, abilities and passives.
//!
//! A *character* is a unique, named unit (Sir Aldric, Kestrel...). Each belongs to
//! one *class*, which sets its stat profile and counts towards class synergies,
//! and has a *rarity*, which sets its price and how strong it is.

/// A unit class: stat profile and synergy group.
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

    /// Level 1 stats of a Common character of this class.
    pub fn base_stats(self) -> Stats {
        let (max_hp, attack, range) = match self {
            Class::Knight => (650, 42, 1),
            Class::Ranger => (300, 44, 3),
            Class::Mage => (350, 45, 3),
            Class::Cleric => (380, 25, 2),
            Class::Rogue => (480, 72, 1),
            Class::Berserker => (550, 50, 1),
        };
        Stats { max_hp, attack, range }
    }

    pub fn is_ranged(self) -> bool {
        self.base_stats().range > 1
    }

    pub fn name(self) -> &'static str {
        match self {
            Class::Knight => "Knight",
            Class::Ranger => "Ranger",
            Class::Mage => "Mage",
            Class::Cleric => "Cleric",
            Class::Rogue => "Rogue",
            Class::Berserker => "Berserker",
        }
    }

    /// Plural, for synergy labels ("2 Knights").
    pub fn plural(self) -> &'static str {
        match self {
            Class::Knight => "Knights",
            Class::Ranger => "Rangers",
            Class::Mage => "Mages",
            Class::Cleric => "Clerics",
            Class::Rogue => "Rogues",
            Class::Berserker => "Berserkers",
        }
    }

    pub fn role(self) -> &'static str {
        match self {
            Class::Knight => "Tank",
            Class::Ranger => "Ranged damage",
            Class::Mage => "Area damage",
            Class::Cleric => "Healer",
            Class::Rogue => "Assassin",
            Class::Berserker => "Bruiser",
        }
    }

    /// Dense index (0..6), for per-class arrays.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// How rare (and how strong and expensive) a character is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    Common,
    Rare,
    Epic,
}

impl Rarity {
    pub const ALL: [Rarity; 3] = [Rarity::Common, Rarity::Rare, Rarity::Epic];

    /// Gold to buy one copy.
    pub fn cost(self) -> u32 {
        match self {
            Rarity::Common => 1,
            Rarity::Rare => 2,
            Rarity::Epic => 3,
        }
    }

    /// Copies of each character of this rarity in the shared pool.
    pub fn pool_copies(self) -> u32 {
        match self {
            Rarity::Common => 12,
            Rarity::Rare => 10,
            Rarity::Epic => 8,
        }
    }

    /// HP and attack multiplier, in percent, relative to a Common.
    pub fn stat_percent(self) -> u32 {
        match self {
            Rarity::Common => 100,
            Rarity::Rare => 110,
            Rarity::Epic => 120,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }
}

/// A unique named character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Character {
    BramGatewarden,
    SirAldric,
    Ysolde,
    PipQuickfeather,
    Kestrel,
    Elowen,
    Tobble,
    Mirelle,
    Vorn,
    SisterAgnes,
    BrotherCalloway,
    Seraphine,
    Nettle,
    Vex,
    GreyWidow,
    Grub,
    Hilda,
    Korgath,
}

impl Character {
    pub const COUNT: usize = 18;
    pub const ALL: [Character; Self::COUNT] = [
        Character::BramGatewarden,
        Character::SirAldric,
        Character::Ysolde,
        Character::PipQuickfeather,
        Character::Kestrel,
        Character::Elowen,
        Character::Tobble,
        Character::Mirelle,
        Character::Vorn,
        Character::SisterAgnes,
        Character::BrotherCalloway,
        Character::Seraphine,
        Character::Nettle,
        Character::Vex,
        Character::GreyWidow,
        Character::Grub,
        Character::Hilda,
        Character::Korgath,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Character::BramGatewarden => "Bram Gatewarden",
            Character::SirAldric => "Sir Aldric",
            Character::Ysolde => "Ysolde the Unbroken",
            Character::PipQuickfeather => "Pip Quickfeather",
            Character::Kestrel => "Kestrel",
            Character::Elowen => "Elowen Starshot",
            Character::Tobble => "Tobble the Apprentice",
            Character::Mirelle => "Mirelle Frostweaver",
            Character::Vorn => "Archmage Vorn",
            Character::SisterAgnes => "Sister Agnes",
            Character::BrotherCalloway => "Brother Calloway",
            Character::Seraphine => "Seraphine Dawnbringer",
            Character::Nettle => "Nettle",
            Character::Vex => "Vex",
            Character::GreyWidow => "The Grey Widow",
            Character::Grub => "Grub Ironjaw",
            Character::Hilda => "Hilda Bloodaxe",
            Character::Korgath => "Korgath the Undying",
        }
    }

    /// A short name for tight spaces.
    pub fn short_name(self) -> &'static str {
        match self {
            Character::BramGatewarden => "Bram",
            Character::SirAldric => "Aldric",
            Character::Ysolde => "Ysolde",
            Character::PipQuickfeather => "Pip",
            Character::Kestrel => "Kestrel",
            Character::Elowen => "Elowen",
            Character::Tobble => "Tobble",
            Character::Mirelle => "Mirelle",
            Character::Vorn => "Vorn",
            Character::SisterAgnes => "Agnes",
            Character::BrotherCalloway => "Calloway",
            Character::Seraphine => "Seraphine",
            Character::Nettle => "Nettle",
            Character::Vex => "Vex",
            Character::GreyWidow => "Widow",
            Character::Grub => "Grub",
            Character::Hilda => "Hilda",
            Character::Korgath => "Korgath",
        }
    }

    pub fn class(self) -> Class {
        use Character::*;
        match self {
            BramGatewarden | SirAldric | Ysolde => Class::Knight,
            PipQuickfeather | Kestrel | Elowen => Class::Ranger,
            Tobble | Mirelle | Vorn => Class::Mage,
            SisterAgnes | BrotherCalloway | Seraphine => Class::Cleric,
            Nettle | Vex | GreyWidow => Class::Rogue,
            Grub | Hilda | Korgath => Class::Berserker,
        }
    }

    pub fn rarity(self) -> Rarity {
        use Character::*;
        match self {
            BramGatewarden | PipQuickfeather | Tobble | SisterAgnes | Nettle | Grub => {
                Rarity::Common
            }
            SirAldric | Kestrel | Mirelle | BrotherCalloway | Vex | Hilda => Rarity::Rare,
            Ysolde | Elowen | Vorn | Seraphine | GreyWidow | Korgath => Rarity::Epic,
        }
    }

    /// The signature abilities this character brings to its program.
    pub fn abilities(self) -> &'static [Ability] {
        use Ability as A;
        match self {
            Character::BramGatewarden => &[A::Taunt],
            Character::SirAldric => &[A::ShieldWall],
            Character::Ysolde => &[A::Taunt, A::SecondWind],
            Character::PipQuickfeather => &[A::AimedShot],
            Character::Kestrel => &[A::Kite],
            Character::Elowen => &[A::AimedShot, A::Volley],
            Character::Tobble => &[A::Firebolt],
            Character::Mirelle => &[A::FrostNova],
            Character::Vorn => &[A::Fireball, A::Blink],
            Character::SisterAgnes => &[A::Heal],
            Character::BrotherCalloway => &[A::Sanctuary],
            Character::Seraphine => &[A::Heal, A::MassHeal],
            Character::Nettle => &[A::Backstab],
            Character::Vex => &[A::Shadowstep],
            Character::GreyWidow => &[A::Shadowstep, A::Assassinate],
            Character::Grub => &[A::Cleave],
            Character::Hilda => &[A::Frenzy],
            Character::Korgath => &[A::Cleave, A::Rampage],
        }
    }

    pub fn has_ability(self, ability: Ability) -> bool {
        self.abilities().contains(&ability)
    }

    /// Rare and Epic characters have a passive.
    pub fn passive(self) -> Option<Passive> {
        use Passive as P;
        match self {
            Character::SirAldric => Some(P::Bulwark),
            Character::Ysolde => Some(P::LastStand),
            Character::Kestrel => Some(P::HuntersMark),
            Character::Elowen => Some(P::EagleEye),
            Character::Mirelle => Some(P::Chill),
            Character::Vorn => Some(P::ArcaneSurge),
            Character::BrotherCalloway => Some(P::Blessing),
            Character::Seraphine => Some(P::Resurrection),
            Character::Vex => Some(P::Opportunist),
            Character::GreyWidow => Some(P::Vanish),
            Character::Hilda => Some(P::Bloodlust),
            Character::Korgath => Some(P::Undying),
            _ => None,
        }
    }

    /// Level 1 stats: the class profile scaled by rarity.
    pub fn base_stats(self) -> Stats {
        let class = self.class().base_stats();
        let pct = self.rarity().stat_percent();
        let range = class.range + u32::from(self.passive() == Some(Passive::EagleEye));
        Stats { max_hp: class.max_hp * pct / 100, attack: class.attack * pct / 100, range }
    }

    /// Dense index (0..18), for per-character arrays.
    pub fn index(self) -> usize {
        self as usize
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
    /// 1, 2 or 3.
    pub fn number(self) -> u32 {
        match self {
            Level::One => 1,
            Level::Two => 2,
            Level::Three => 3,
        }
    }

    /// The level a merge of three of these produces, if any.
    pub fn next(self) -> Option<Level> {
        match self {
            Level::One => Some(Level::Two),
            Level::Two => Some(Level::Three),
            Level::Three => None,
        }
    }

    /// How many level 1 copies a unit of this level is made of.
    pub fn copies(self) -> u32 {
        match self {
            Level::One => 1,
            Level::Two => 3,
            Level::Three => 9,
        }
    }

    /// Rule cards (nodes) a unit of this level can hold, before synergy bonuses.
    pub fn max_nodes(self) -> usize {
        match self {
            Level::One => 2,
            Level::Two => 3,
            Level::Three => 4,
        }
    }

    /// Stat multiplier in percent (x1.8 per level, as in TFT).
    pub fn scale_percent(self) -> u32 {
        match self {
            Level::One => 100,
            Level::Two => 180,
            Level::Three => 324,
        }
    }

    /// Applies this level's scaling to base stats.
    pub fn scale(self, base: Stats) -> Stats {
        let pct = self.scale_percent();
        Stats { max_hp: base.max_hp * pct / 100, attack: base.attack * pct / 100, ..base }
    }
}

/// A signature ability. Each has a cooldown in ticks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ability {
    Taunt,
    ShieldWall,
    SecondWind,
    AimedShot,
    Kite,
    Volley,
    Firebolt,
    FrostNova,
    Fireball,
    Blink,
    Heal,
    Sanctuary,
    MassHeal,
    Backstab,
    Shadowstep,
    Assassinate,
    Frenzy,
    Cleave,
    Rampage,
}

impl Ability {
    pub const COUNT: usize = 19;

    /// Ticks before the ability can be used again (before synergy bonuses).
    pub fn cooldown(self) -> u32 {
        match self {
            Ability::Taunt => 6,
            Ability::ShieldWall => 6,
            Ability::SecondWind => 8,
            Ability::AimedShot => 4,
            Ability::Kite => 2,
            Ability::Volley => 5,
            Ability::Firebolt => 4,
            Ability::FrostNova => 6,
            Ability::Fireball => 5,
            Ability::Blink => 6,
            Ability::Heal => 3,
            Ability::Sanctuary => 8,
            Ability::MassHeal => 6,
            Ability::Backstab => 4,
            Ability::Shadowstep => 5,
            Ability::Assassinate => 5,
            Ability::Frenzy => 8,
            Ability::Cleave => 3,
            Ability::Rampage => 8,
        }
    }

    /// Dense index, for storing per-ability state in a fixed-size array.
    pub fn index(self) -> usize {
        self as usize
    }
}

/// A character's always-on passive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Passive {
    /// Sir Aldric: takes 10% less damage per adjacent ally (up to 30%).
    Bulwark,
    /// Ysolde: takes half damage while below 25% HP.
    LastStand,
    /// Kestrel: enemies she hits are marked; marked enemies take 15% more damage.
    HuntersMark,
    /// Elowen: +1 range (built into her stats).
    EagleEye,
    /// Mirelle: frozen enemies take 20% more damage.
    Chill,
    /// Vorn: every third ability he uses crits.
    ArcaneSurge,
    /// Brother Calloway: his Sanctuary also heals for 3x his attack.
    Blessing,
    /// Seraphine: the first ally to fall returns at 30% HP (once per battle).
    Resurrection,
    /// Vex: +30% damage to enemies below half HP.
    Opportunist,
    /// The Grey Widow: can't be targeted for the first 2 ticks.
    Vanish,
    /// Hilda: while below half HP, heals 20% of the damage she deals.
    Bloodlust,
    /// Korgath: survives the first killing blow on 1 HP.
    Undying,
}

impl Passive {
    pub fn name(self) -> &'static str {
        match self {
            Passive::Bulwark => "Bulwark",
            Passive::LastStand => "Last Stand",
            Passive::HuntersMark => "Hunter's Mark",
            Passive::EagleEye => "Eagle Eye",
            Passive::Chill => "Chill",
            Passive::ArcaneSurge => "Arcane Surge",
            Passive::Blessing => "Blessing",
            Passive::Resurrection => "Resurrection",
            Passive::Opportunist => "Opportunist",
            Passive::Vanish => "Vanish",
            Passive::Bloodlust => "Bloodlust",
            Passive::Undying => "Undying",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Passive::Bulwark => "Takes 10% less damage per adjacent ally (up to 30%).",
            Passive::LastStand => "Takes half damage while below 25% HP.",
            Passive::HuntersMark => "Enemies she hits are marked and take 15% more damage.",
            Passive::EagleEye => "+1 range.",
            Passive::Chill => "Frozen enemies take 20% more damage.",
            Passive::ArcaneSurge => "Every third ability he uses is a critical hit.",
            Passive::Blessing => "His Sanctuary also heals the target for 3x his attack.",
            Passive::Resurrection => "The first ally to fall returns at 30% HP, once per battle.",
            Passive::Opportunist => "+30% damage to enemies below half HP.",
            Passive::Vanish => "Can't be targeted for the first 2 ticks.",
            Passive::Bloodlust => "While below half HP, heals 20% of the damage she deals.",
            Passive::Undying => "Survives the first killing blow on 1 HP.",
        }
    }
}

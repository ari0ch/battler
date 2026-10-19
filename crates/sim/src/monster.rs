//! Monsters: the opponents of PvE rounds.
//!
//! Monsters are not characters. They never appear in the shop or the pool, don't
//! merge and don't count towards synergies. Each has fixed stats and a fixed
//! program; a boss's program uses HP thresholds to change behaviour in phases.
//!
//! Every monster still has a [`Class`], but only as a behaviour family for the
//! interpreter (class bonuses need a synergy, and monsters never have one).

use crate::{
    Ability, Action, Class, Condition::*, Node, Passive, Pos, Program, Stats, Target, Threshold,
    UnitSpec,
};
use crate::{
    Level,
    run::economy::{BOARD_DEPTH, BOARD_LANES},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Monster {
    Slime,
    Wolf,
    GoblinBrute,
    GoblinArcher,
    Skeleton,
    OgreChieftain,
    FrostWyrm,
    Lich,
}

impl Monster {
    pub const ALL: [Monster; 8] = [
        Monster::Slime,
        Monster::Wolf,
        Monster::GoblinBrute,
        Monster::GoblinArcher,
        Monster::Skeleton,
        Monster::OgreChieftain,
        Monster::FrostWyrm,
        Monster::Lich,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Monster::Slime => "Slime",
            Monster::Wolf => "Wolf",
            Monster::GoblinBrute => "Goblin Brute",
            Monster::GoblinArcher => "Goblin Archer",
            Monster::Skeleton => "Skeleton",
            Monster::OgreChieftain => "Ogre Chieftain",
            Monster::FrostWyrm => "Frost Wyrm",
            Monster::Lich => "The Lich",
        }
    }

    /// A name short enough for a unit tile.
    pub fn short_name(self) -> &'static str {
        match self {
            Monster::Slime => "Slime",
            Monster::Wolf => "Wolf",
            Monster::GoblinBrute => "Brute",
            Monster::GoblinArcher => "Archer",
            Monster::Skeleton => "Skeleton",
            Monster::OgreChieftain => "Ogre",
            Monster::FrostWyrm => "Wyrm",
            Monster::Lich => "Lich",
        }
    }

    pub fn is_boss(self) -> bool {
        matches!(self, Monster::OgreChieftain | Monster::FrostWyrm | Monster::Lich)
    }

    /// Behaviour family (see the module docs).
    pub fn class(self) -> Class {
        match self {
            Monster::Slime | Monster::Skeleton => Class::Knight,
            Monster::Wolf => Class::Rogue,
            Monster::GoblinBrute | Monster::OgreChieftain => Class::Berserker,
            Monster::GoblinArcher => Class::Ranger,
            Monster::FrostWyrm | Monster::Lich => Class::Mage,
        }
    }

    /// Stats before any boss scaling. Monsters are always level 1.
    pub fn base_stats(self) -> Stats {
        let (max_hp, attack, range) = match self {
            Monster::Slime => (260, 18, 1),
            Monster::Wolf => (220, 28, 1),
            Monster::GoblinBrute => (520, 34, 1),
            Monster::GoblinArcher => (200, 32, 3),
            Monster::Skeleton => (420, 40, 1),
            Monster::OgreChieftain => (6200, 132, 1),
            Monster::FrostWyrm => (10000, 165, 2),
            Monster::Lich => (9600, 180, 3),
        };
        Stats { max_hp, attack, range }
    }

    /// Abilities the monster's program may use.
    pub fn abilities(self) -> &'static [Ability] {
        use Ability as A;
        match self {
            Monster::OgreChieftain => &[A::Cleave, A::Frenzy],
            Monster::FrostWyrm => &[A::FrostNova, A::Fireball],
            Monster::Lich => &[A::Fireball, A::MassHeal],
            _ => &[],
        }
    }

    pub fn passive(self) -> Option<Passive> {
        match self {
            Monster::Lich => Some(Passive::Undying),
            _ => None,
        }
    }

    /// The monster's fixed program, best cards first.
    pub fn program(self) -> Program {
        use Ability as A;
        let sig = |a| Action::for_ability(a);
        let nodes = match self {
            Monster::Slime | Monster::GoblinBrute | Monster::Skeleton => vec![],
            // Wolves pile onto the weakest target: heal or protect it.
            Monster::Wolf => vec![Node::new(Always, Action::Attack(Target::Weakest))],
            Monster::GoblinArcher => vec![Node::new(Always, Action::Attack(Target::Weakest))],
            // Phase 2 below half HP: frenzy, then keep cleaving.
            Monster::OgreChieftain => vec![
                Node::new(MyHpBelow(Threshold::P50), sig(A::Frenzy)),
                Node::new(EnemyAdjacent, sig(A::Cleave)),
            ],
            // Freezes anyone who closes in; breathes fire once wounded.
            Monster::FrostWyrm => vec![
                Node::new(EnemyAdjacent, sig(A::FrostNova)),
                Node::new(MyHpBelow(Threshold::P75), sig(A::Fireball)),
            ],
            // Keeps its skeletons standing, and casts fire at range.
            Monster::Lich => vec![
                Node::new(AllyHpBelow(Threshold::P50), sig(A::MassHeal)),
                Node::new(Always, sig(A::Fireball)),
                Node::new(Always, Action::Attack(Target::Weakest)),
            ],
        };
        Program::new(nodes)
    }
}

/// A PvE round's opponents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Wave {
    Slimes,
    Wolves,
    Goblins,
    /// A boss with its escort; `tier` 0 is the first appearance, and each
    /// later cycle through the bosses makes them stronger.
    Boss {
        boss: Monster,
        tier: u32,
    },
}

impl Wave {
    pub fn name(self) -> &'static str {
        match self {
            Wave::Slimes => "Slimes",
            Wave::Wolves => "Wolf pack",
            Wave::Goblins => "Goblin raiders",
            Wave::Boss { boss, .. } => boss.name(),
        }
    }

    /// One line of advice shown before the round.
    pub fn hint(self) -> &'static str {
        match self {
            Wave::Slimes => "Slow and weak. Any board will do.",
            Wave::Wolves => "Wolves gang up on your weakest unit. Heal or protect it.",
            Wave::Goblins => "Archers hide behind a brute. Target the back line.",
            Wave::Boss { boss: Monster::OgreChieftain, .. } => {
                "Cleaves everyone next to it, and frenzies below half HP."
            }
            Wave::Boss { boss: Monster::FrostWyrm, .. } => {
                "Freezes units that close in, and breathes fire once wounded."
            }
            Wave::Boss { .. } => "Heals its skeletons and survives one killing blow.",
        }
    }

    pub fn is_boss(self) -> bool {
        matches!(self, Wave::Boss { .. })
    }

    /// The units, in the wave's own coordinates (its left half, as if it were a
    /// player); battles mirror it onto the right.
    pub fn units(self) -> Vec<(Pos, UnitSpec)> {
        use Monster::*;
        // Formations are written as (row, lane offset from the middle lane).
        let at = |x, dy: i32, m: Monster| (Pos::new(x, BOARD_LANES / 2 + dy), m);
        let list = match self {
            Wave::Slimes => vec![at(3, -1, Slime), at(3, 0, Slime), at(3, 1, Slime)],
            Wave::Wolves => {
                vec![at(3, -2, Wolf), at(3, -1, Wolf), at(3, 1, Wolf), at(3, 2, Wolf)]
            }
            Wave::Goblins => {
                vec![at(3, 0, GoblinBrute), at(0, -1, GoblinArcher), at(0, 1, GoblinArcher)]
            }
            Wave::Boss { boss: OgreChieftain, .. } => {
                vec![at(3, 0, OgreChieftain), at(3, -2, GoblinBrute), at(3, 2, GoblinBrute)]
            }
            Wave::Boss { boss: FrostWyrm, .. } => vec![at(2, 0, FrostWyrm)],
            Wave::Boss { boss, .. } => {
                vec![at(0, 0, boss), at(3, -1, Skeleton), at(3, 0, Skeleton), at(3, 1, Skeleton)]
            }
        };
        let tier = match self {
            Wave::Boss { tier, .. } => tier,
            _ => 0,
        };
        list.into_iter()
            .map(|(pos, m)| {
                let mut spec = UnitSpec::new(m, Level::One, m.program());
                spec.boost_percent = BOSS_TIER_BOOST * tier;
                (pos, spec)
            })
            .collect()
    }
}

/// Extra HP and attack, in percent, for each later cycle of the bosses.
pub const BOSS_TIER_BOOST: u32 = 80;

/// Mirrors a position from a player's half onto the right half of the battlefield.
pub fn mirror(pos: Pos) -> Pos {
    Pos::new(BOARD_DEPTH - 1 - pos.x, pos.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitKind;

    #[test]
    fn programs_only_use_the_monsters_own_abilities() {
        for m in Monster::ALL {
            assert_eq!(m.program().validate_basic(UnitKind::Monster(m)), Ok(()), "{m:?}");
        }
    }

    #[test]
    fn bosses_scale_with_tier() {
        let first = Wave::Boss { boss: Monster::Lich, tier: 0 }.units();
        let later = Wave::Boss { boss: Monster::Lich, tier: 2 }.units();
        assert_eq!(first[0].1.boost_percent, 0);
        assert_eq!(later[0].1.boost_percent, 2 * BOSS_TIER_BOOST);
    }
}

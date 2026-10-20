//! Class synergies: fielding 2 or 3 characters of a class unlocks extras.
//!
//! | Class      | 2: new block (scope)                  | 3: class bonus                    |
//! |------------|---------------------------------------|-----------------------------------|
//! | Knights    | `ally attacked` (Knights only)        | +1 card slot for Knights          |
//! | Rangers    | `marked` target (team)                | +1 range for Rangers              |
//! | Mages      | `enemies grouped` (Mages only)        | Mage signature cooldowns -2       |
//! | Clerics    | Send Signal / signal received (team)  | receiving a signal heals 10%      |
//! | Rogues     | `enemy isolated` (Rogues only)        | Rogue crit chance doubles         |
//! | Berserkers | `surrounded` (Berserkers only)        | Fury: card chaining               |

use crate::program::Scope;
use crate::{Character, Class, Level};

/// Units needed for a synergy's first and second tier.
pub const TIERS: [u8; 2] = [2, 3];

/// How many distinct characters of each class are on a board.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Synergies {
    counts: [u8; 6],
}

impl Synergies {
    /// Counts distinct characters (a character appearing twice counts once).
    pub fn from_characters(characters: impl IntoIterator<Item = Character>) -> Self {
        Self::from_members(characters.into_iter().map(|c| (c, None)))
    }

    /// Counts distinct characters per class, where a character may also count
    /// for an extra class (a Banner item). Each character counts at most once
    /// per class, however many copies or Banners there are.
    pub fn from_members(members: impl IntoIterator<Item = (Character, Option<Class>)>) -> Self {
        let mut seen = [[false; 6]; Character::COUNT];
        let mut counts = [0u8; 6];
        for (c, extra) in members {
            for class in [Some(c.class()), extra].into_iter().flatten() {
                if !std::mem::replace(&mut seen[c.index()][class.index()], true) {
                    counts[class.index()] += 1;
                }
            }
        }
        Self { counts }
    }

    pub fn count(&self, class: Class) -> u8 {
        self.counts[class.index()]
    }

    /// 0 (inactive), 2 or 3.
    pub fn tier(&self, class: Class) -> u8 {
        match self.count(class) {
            0..=1 => 0,
            2 => 2,
            _ => 3,
        }
    }

    pub fn active(&self, class: Class) -> bool {
        self.tier(class) >= 2
    }

    /// True if the class's tier 3 bonus applies.
    pub fn full(&self, class: Class) -> bool {
        self.tier(class) >= 3
    }

    /// True if `character` may use a block unlocked by `class`'s synergy.
    pub fn allows(&self, class: Class, scope: Scope, character: Character) -> bool {
        self.active(class) && (scope == Scope::Team || character.class() == class)
    }

    /// Card slots for a unit: its level's, plus 1 for Knights with 3 Knights.
    pub fn card_slots(&self, character: Character, level: Level) -> usize {
        let bonus = character.class() == Class::Knight && self.full(Class::Knight);
        level.max_nodes() + usize::from(bonus)
    }

    /// Number of active synergies.
    pub fn active_count(&self) -> usize {
        Class::ALL.iter().filter(|&&c| self.active(c)).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Character::*;

    #[test]
    fn counts_distinct_characters_per_class() {
        let s = Synergies::from_characters([BramGatewarden, SirAldric, BramGatewarden, Kestrel]);
        assert_eq!(s.count(Class::Knight), 2);
        assert_eq!(s.tier(Class::Knight), 2);
        assert_eq!(s.tier(Class::Ranger), 0);
    }

    #[test]
    fn three_knights_add_a_card_slot_for_knights_only() {
        let s = Synergies::from_characters([BramGatewarden, SirAldric, Ysolde]);
        assert_eq!(s.card_slots(Ysolde, Level::Three), 5);
        assert_eq!(s.card_slots(Kestrel, Level::Three), 4);
    }

    #[test]
    fn class_only_blocks_need_the_class() {
        let s = Synergies::from_characters([Grub, Hilda]);
        assert!(s.allows(Class::Berserker, Scope::ClassOnly, Korgath));
        assert!(!s.allows(Class::Berserker, Scope::ClassOnly, Nettle));
        let clerics = Synergies::from_characters([SisterAgnes, Seraphine]);
        assert!(clerics.allows(Class::Cleric, Scope::Team, Nettle));
    }
}

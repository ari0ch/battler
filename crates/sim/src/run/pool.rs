//! The shared pool of character copies that every shop in a lobby draws from.
//!
//! Buying a copy takes it out of everyone's reach, so players compete for the
//! characters they want; selling a unit or rerolling past an offer puts the
//! copies back.

use crate::{Character, Rarity, Rng};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pool {
    copies: [u32; Character::COUNT],
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Pool {
    /// A full pool: [`Rarity::pool_copies`] of every character.
    pub fn new() -> Self {
        let mut copies = [0; Character::COUNT];
        for c in Character::ALL {
            copies[c.index()] = c.rarity().pool_copies();
        }
        Self { copies }
    }

    pub fn remaining(&self, character: Character) -> u32 {
        self.copies[character.index()]
    }

    pub fn give_back(&mut self, character: Character, copies: u32) {
        self.copies[character.index()] += copies;
    }

    /// Draws one copy of the given rarity, weighted by copies left.
    pub fn draw(&mut self, rarity: Rarity, rng: &mut Rng) -> Option<Character> {
        let candidates = Character::ALL.iter().filter(|c| c.rarity() == rarity);
        let total: u32 = candidates.clone().map(|c| self.remaining(*c)).sum();
        if total == 0 {
            return None;
        }
        let mut pick = rng.below(total);
        for &c in candidates {
            let left = self.remaining(c);
            if pick < left {
                self.copies[c.index()] -= 1;
                return Some(c);
            }
            pick -= left;
        }
        None
    }

    /// Total copies left, for tests and stats.
    pub fn total(&self) -> u32 {
        self.copies.iter().sum()
    }
}

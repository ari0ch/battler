//! Run tuning numbers and the gold/damage formulas.
//!
//! Every value here is an agreed starting point, to tune after playtesting
//! (see `docs/DESIGN.md`).

pub const START_HP: u32 = 100;
pub const START_GOLD: u32 = 10;
pub const BASE_INCOME: u32 = 5;
pub const WIN_BONUS: u32 = 1;
/// +1 gold of interest per this much gold banked...
pub const INTEREST_STEP: u32 = 10;
/// ...up to this much.
pub const MAX_INTEREST: u32 = 5;

pub const UNIT_COST: u32 = 3;
pub const REROLL_COST: u32 = 2;
pub const SHOP_SIZE: usize = 5;
pub const BENCH_SIZE: usize = 9;
/// Level 1 copies of each class in the shared pool.
pub const POOL_COPIES: u32 = 12;
/// Surviving this round with HP left wins the run.
pub const FINAL_ROUND: u32 = 15;

/// Battlefield size, and the columns the player may place units in.
pub const BOARD_WIDTH: i32 = 8;
pub const BOARD_HEIGHT: i32 = 5;
pub const PLAYER_COLUMNS: i32 = 4;
/// Ticks before an undecided battle is called a draw.
pub const MAX_TICKS: u32 = 200;

/// How many units may be on the board in a given round.
pub fn board_size(round: u32) -> usize {
    match round {
        0..=2 => 3,
        3..=5 => 4,
        6..=9 => 5,
        _ => 6,
    }
}

/// Interest on banked gold.
pub fn interest(banked: u32) -> u32 {
    (banked / INTEREST_STEP).min(MAX_INTEREST)
}

/// Streak bonus for a run of consecutive wins or losses.
pub fn streak_bonus(length: u32) -> u32 {
    match length {
        0..=1 => 0,
        2..=3 => 1,
        4..=5 => 2,
        _ => 3,
    }
}

/// HP lost when a battle is lost: 2 plus the levels of the enemy units still alive.
pub fn loss_damage(surviving_enemy_levels: u32) -> u32 {
    2 + surviving_enemy_levels
}

/// The gold a player receives at the end of a round, itemised for the result screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Income {
    pub base: u32,
    pub win: u32,
    pub interest: u32,
    pub streak: u32,
}

impl Income {
    pub fn total(&self) -> u32 {
        self.base + self.win + self.interest + self.streak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interest_is_capped() {
        assert_eq!(interest(9), 0);
        assert_eq!(interest(23), 2);
        assert_eq!(interest(99), MAX_INTEREST);
    }

    #[test]
    fn streak_bonus_steps() {
        assert_eq!([1, 2, 3, 4, 5, 6, 9].map(streak_bonus), [0, 1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn board_grows_with_rounds() {
        assert_eq!([1, 3, 6, 10, 15].map(board_size), [3, 4, 5, 6, 6]);
    }
}

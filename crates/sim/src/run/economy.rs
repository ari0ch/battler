//! Run tuning numbers and the gold/XP/damage formulas.
//!
//! Every value here is an agreed starting point, to tune after simulation and
//! playtesting (see `docs/DESIGN.md`).

pub const START_HP: u32 = 100;
pub const START_GOLD: u32 = 10;
pub const BASE_INCOME: u32 = 5;
pub const WIN_BONUS: u32 = 1;
/// +1 gold of interest per this much gold banked...
pub const INTEREST_STEP: u32 = 10;
/// ...up to this much.
pub const MAX_INTEREST: u32 = 5;

pub const REROLL_COST: u32 = 2;
pub const SHOP_SIZE: usize = 5;
pub const BENCH_SIZE: usize = 9;

/// Players in a lobby. The last one standing wins.
pub const LOBBY_SIZE: usize = 8;

/// PvE rounds: monsters on rounds 1-3, then a boss every `BOSS_EVERY` rounds.
pub const FIRST_BOSS_ROUND: u32 = 7;
pub const BOSS_EVERY: u32 = 4;
/// Gold for beating a monster round or a boss.
pub const MONSTER_REWARD: u32 = 2;
pub const BOSS_REWARD: u32 = 4;
/// HP lost to a monster round. Boss losses scale with the round instead.
pub const MONSTER_LOSS_DAMAGE: u32 = 2;
/// A boss loss costs this much more than a versus round's base damage.
pub const BOSS_EXTRA_DAMAGE: u32 = 5;

/// Player levels. Board size equals the player's level.
pub const START_LEVEL: u32 = 3;
pub const MAX_LEVEL: u32 = 8;
/// XP gained automatically at the end of every round.
pub const XP_PER_ROUND: u32 = 2;
/// Buying XP: this much gold for this much XP.
pub const XP_COST: u32 = 4;
pub const XP_PER_BUY: u32 = 4;

/// Battlefield size. In sim coordinates `x` runs from the player's back row
/// (0) to the enemy's back row (`BOARD_DEPTH - 1`), and `y` is the lane. The
/// game draws it rotated: lanes across the screen, the enemy half above.
pub const BOARD_DEPTH: i32 = 8;
/// Lanes: how wide the board is on screen.
pub const BOARD_LANES: i32 = 7;
/// Rows of each side's half: the player owns `x` in `0..SIDE_DEPTH`.
pub const SIDE_DEPTH: i32 = 4;
/// Lanes from the middle outwards, for placing units automatically.
pub const LANES_FROM_CENTRE: [i32; BOARD_LANES as usize] = [3, 2, 4, 1, 5, 0, 6];
/// Ticks before an undecided battle is called a draw.
pub const MAX_TICKS: u32 = 200;

/// XP needed to go from `level` to the next one, or `None` at the maximum.
pub fn xp_to_next(level: u32) -> Option<u32> {
    match level {
        3 => Some(6),
        4 => Some(10),
        5 => Some(20),
        6 => Some(36),
        7 => Some(56),
        _ => None,
    }
}

/// Shop odds (Common, Rare, Epic) in percent at a player level.
pub fn shop_odds(level: u32) -> [u32; 3] {
    match level {
        0..=3 => [80, 20, 0],
        4 => [65, 30, 5],
        5 => [45, 40, 15],
        6 => [30, 45, 25],
        7 => [20, 40, 40],
        _ => [15, 35, 50],
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

/// The part of a loss's damage that depends only on the round: 2 + round +
/// round²/6. Gentle early (8 at round 4) and steep late (54 at round 15), which
/// keeps lobbies to about 16 rounds (see `docs/DESIGN.md`, Playtest notes).
pub fn round_damage(round: u32) -> u32 {
    2 + round + round * round / 6
}

/// HP lost when a versus battle is lost: the round's damage plus the levels of
/// the enemy units still alive, so a narrow loss hurts less than a wipe.
pub fn loss_damage(round: u32, surviving_enemy_levels: u32) -> u32 {
    round_damage(round) + surviving_enemy_levels
}

/// HP lost when a PvE round is lost: a flat amount to monsters; to a boss, more
/// than a versus loss in the same round would cost.
pub fn pve_loss_damage(boss: bool, round: u32) -> u32 {
    if boss { round_damage(round) + BOSS_EXTRA_DAMAGE } else { MONSTER_LOSS_DAMAGE }
}

/// Gold for winning a PvE round.
pub fn pve_reward(boss: bool) -> u32 {
    if boss { BOSS_REWARD } else { MONSTER_REWARD }
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
    fn lanes_from_centre_covers_every_lane_once() {
        let mut lanes = LANES_FROM_CENTRE;
        assert_eq!(lanes[0], BOARD_LANES / 2);
        lanes.sort_unstable();
        assert!(lanes.iter().copied().eq(0..BOARD_LANES));
    }

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
    fn loss_damage_starts_gentle_and_steepens() {
        assert_eq!([4, 8, 12, 15].map(round_damage), [8, 20, 38, 54]);
        assert_eq!(loss_damage(4, 3), 11);
        // Each round costs more than the one before, by a growing amount.
        let steps: Vec<u32> = (4..20).map(|r| round_damage(r + 1) - round_damage(r)).collect();
        assert!(steps.windows(2).all(|w| w[1] >= w[0]));
    }

    #[test]
    fn boss_losses_cost_more_than_versus_losses() {
        for round in [7, 11, 15, 19] {
            assert!(pve_loss_damage(true, round) > loss_damage(round, 0));
        }
        assert_eq!(pve_loss_damage(false, 2), MONSTER_LOSS_DAMAGE);
    }

    #[test]
    fn odds_add_up_and_shift_towards_epics() {
        for level in START_LEVEL..=MAX_LEVEL {
            assert_eq!(shop_odds(level).iter().sum::<u32>(), 100, "level {level}");
        }
        assert!(shop_odds(MAX_LEVEL)[2] > shop_odds(START_LEVEL)[2]);
        assert_eq!(xp_to_next(MAX_LEVEL), None);
    }
}

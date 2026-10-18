//! AI opponents: enemy boards that get stronger each round.
//!
//! Each class has a pre-written program; a unit gets as many of its cards as
//! its level allows. Board size follows the same rule as the player's, and
//! levels rise as the run goes on.

use super::economy::{BOARD_HEIGHT, BOARD_WIDTH, PLAYER_COLUMNS, board_size};
use crate::{
    Action::*, Class, Condition::*, Level, Node, Pos, Program, Rng, Target, Threshold, UnitSpec,
};

/// The AI's full program for a class, best cards first.
pub fn preset_program(class: Class) -> Vec<Node> {
    let card = Node::new;
    match class {
        Class::Knight => vec![
            card(EnemyAdjacent, Taunt),
            card(AllyHpBelow(Threshold::P50), ShieldWall),
            card(MyHpBelow(Threshold::P25), Guard),
            card(Always, ProtectAlly(Target::LowestHp)),
        ],
        Class::Ranger => vec![
            card(EnemyAdjacent, Kite(Target::Nearest)),
            card(EnemyInRange, AimedShot(Target::LowestHp)),
            card(EnemyInRange, Attack(Target::LowestHp)),
            card(MyHpBelow(Threshold::P25), Retreat),
        ],
        Class::Mage => vec![
            card(EnemyAdjacent, Blink),
            card(EnemyInRange, Fireball(Target::Nearest)),
            card(EnemyAdjacent, Retreat),
            card(EnemyInRange, Attack(Target::Ranged)),
        ],
        Class::Cleric => vec![
            card(AllyHpBelow(Threshold::P75), Heal(Target::LowestHp)),
            card(MyHpBelow(Threshold::P50), Sanctuary(Target::Nearest)),
            card(AllyHpBelow(Threshold::P25), Sanctuary(Target::LowestHp)),
            card(EnemyAdjacent, Retreat),
        ],
        Class::Rogue => vec![
            card(Always, Shadowstep(Target::Ranged)),
            card(EnemyAdjacent, Backstab(Target::Nearest)),
            card(MyHpBelow(Threshold::P25), Retreat),
            card(EnemyAdjacent, Attack(Target::Ranged)),
        ],
        Class::Berserker => vec![
            card(EnemyAdjacent, Frenzy),
            card(Outnumbered, Cleave),
            card(EnemyAdjacent, Cleave),
            card(MyHpBelow(Threshold::P25), Guard),
        ],
    }
}

/// How many level 2 and level 3 units the AI fields in a round.
fn level_mix(round: u32, units: usize) -> (usize, usize) {
    let threes = (round.saturating_sub(9) / 2) as usize;
    let threes = threes.min(units);
    let twos = ((round / 2) as usize).min(units - threes);
    (twos, threes)
}

/// Builds the AI's board for a round, in enemy-side coordinates (the right half).
pub fn enemy_board(round: u32, rng: &mut Rng) -> Vec<(Pos, UnitSpec)> {
    let units = board_size(round);
    let mut classes = Class::ALL;
    rng.shuffle(&mut classes);
    let (twos, threes) = level_mix(round, units);

    // Front column faces the player; melee units go there first, ranged at the back.
    let front = PLAYER_COLUMNS;
    let back = BOARD_WIDTH - 1;
    let rows = [2, 1, 3, 0, 4];
    let melee_slots: Vec<Pos> =
        rows.iter().flat_map(|&y| [Pos::new(front, y), Pos::new(front + 1, y)]).collect();
    let ranged_slots: Vec<Pos> =
        rows.iter().flat_map(|&y| [Pos::new(back, y), Pos::new(back - 1, y)]).collect();
    let mut used: Vec<Pos> = Vec::new();

    let mut board = Vec::new();
    for (i, &class) in classes.iter().take(units).enumerate() {
        let level = if i < threes {
            Level::Three
        } else if i < threes + twos {
            Level::Two
        } else {
            Level::One
        };
        let mut nodes = preset_program(class);
        nodes.truncate(level.max_nodes());
        let slots = if class.is_ranged() { &ranged_slots } else { &melee_slots };
        let pos =
            *slots.iter().find(|p| !used.contains(p)).expect("enemy half has room for six units");
        used.push(pos);
        debug_assert!(pos.x >= PLAYER_COLUMNS && pos.x < BOARD_WIDTH && pos.y < BOARD_HEIGHT);
        board.push((pos, UnitSpec::new(class, level, Program::new(nodes))));
    }
    board
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_are_valid_at_every_level() {
        for class in Class::ALL {
            for level in [Level::One, Level::Two, Level::Three] {
                let mut nodes = preset_program(class);
                nodes.truncate(level.max_nodes());
                assert_eq!(
                    Program::new(nodes).validate(class, level),
                    Ok(()),
                    "{class:?} {level:?}"
                );
            }
        }
    }

    #[test]
    fn boards_grow_and_level_up() {
        let mut rng = Rng::new(3);
        let early = enemy_board(1, &mut rng);
        let late = enemy_board(15, &mut rng);
        assert_eq!(early.len(), 3);
        assert!(early.iter().all(|(_, s)| s.level == Level::One));
        assert_eq!(late.len(), 6);
        assert!(late.iter().any(|(_, s)| s.level == Level::Three));
    }

    #[test]
    fn enemy_units_stay_on_their_half_and_never_overlap() {
        let mut rng = Rng::new(9);
        for round in 1..=15 {
            let board = enemy_board(round, &mut rng);
            for (i, (p, _)) in board.iter().enumerate() {
                assert!(p.x >= PLAYER_COLUMNS && p.x < BOARD_WIDTH);
                assert!(board[i + 1..].iter().all(|(q, _)| q != p));
            }
        }
    }
}

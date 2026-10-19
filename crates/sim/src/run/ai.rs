//! AI opponents: enemy boards that get stronger each round.
//!
//! The AI levels up with the round, fields that many units, rolls rarities with
//! the same shop odds as the player, and builds towards one or two class
//! synergies. Each character has a preset program; a unit gets as many of its
//! cards as its level allows.

use super::economy::{
    BOARD_HEIGHT, BOARD_WIDTH, MAX_LEVEL, PLAYER_COLUMNS, START_LEVEL, shop_odds,
};
use crate::{
    Ability, Action, Character, Class, Condition::*, Level, Node, Pos, Program, Rarity, Rng,
    Target, Threshold, UnitSpec,
};

/// The condition a preset program uses for a signature ability.
fn signature_card(ability: Ability) -> Node {
    use Ability as A;
    let condition = match ability {
        A::Taunt | A::Kite | A::FrostNova | A::Blink | A::Frenzy | A::Cleave | A::Rampage => {
            EnemyAdjacent
        }
        A::ShieldWall | A::MassHeal => AllyHpBelow(Threshold::P50),
        A::Heal => AllyHpBelow(Threshold::P75),
        A::Sanctuary => AllyHpBelow(Threshold::P25),
        A::SecondWind => MyHpBelow(Threshold::P50),
        A::AimedShot
        | A::Volley
        | A::Firebolt
        | A::Fireball
        | A::Backstab
        | A::Shadowstep
        | A::Assassinate => Always,
    };
    Node::new(condition, Action::for_ability(ability))
}

/// The AI's full program for a character, best cards first. Uses only blocks
/// available from the start, plus the character's signatures.
pub fn preset_program(character: Character) -> Vec<Node> {
    let mut nodes: Vec<Node> = character.abilities().iter().map(|&a| signature_card(a)).collect();
    // Defensive Mages blink before casting.
    nodes.sort_by_key(|n| n.condition != EnemyAdjacent);
    nodes.push(Node::new(MyHpBelow(Threshold::P25), Action::Guard));
    if character.class().is_ranged() {
        nodes.push(Node::new(Always, Action::Attack(Target::Weakest)));
    }
    nodes
}

/// The AI's player level in a round.
pub fn ai_level(round: u32) -> u32 {
    (START_LEVEL + round / 3).min(MAX_LEVEL)
}

/// How many level 2 and level 3 units the AI fields in a round.
fn star_mix(round: u32, units: usize) -> (usize, usize) {
    let threes = ((round.saturating_sub(9) / 2) as usize).min(units);
    let twos = ((round / 2) as usize).min(units - threes);
    (twos, threes)
}

fn roll_rarity(level: u32, rng: &mut Rng) -> Rarity {
    let odds = shop_odds(level);
    let mut roll = rng.below(100);
    for rarity in Rarity::ALL {
        if roll < odds[rarity.index()] {
            return rarity;
        }
        roll -= odds[rarity.index()];
    }
    Rarity::Common
}

/// Picks the AI's characters: up to 3 of a focus class, 2 of a second, then
/// anything, rolling rarities with the shop odds. Characters never repeat.
fn pick_characters(level: u32, units: usize, rng: &mut Rng) -> Vec<Character> {
    let mut classes = Class::ALL;
    rng.shuffle(&mut classes);
    let wanted: Vec<Class> = [classes[0], classes[0], classes[0], classes[1], classes[1]]
        .into_iter()
        .chain(classes[2..].iter().copied())
        .collect();

    let mut picked: Vec<Character> = Vec::new();
    for &class in &wanted {
        if picked.len() == units {
            break;
        }
        let rarity = roll_rarity(level, rng);
        let of_class =
            |r: Rarity| Character::ALL.into_iter().find(|c| c.class() == class && c.rarity() == r);
        // Prefer the rolled rarity, then cheaper ones, then pricier ones.
        let order = match rarity {
            Rarity::Common => [Rarity::Common, Rarity::Rare, Rarity::Epic],
            Rarity::Rare => [Rarity::Rare, Rarity::Common, Rarity::Epic],
            Rarity::Epic => [Rarity::Epic, Rarity::Rare, Rarity::Common],
        };
        if let Some(c) = order.into_iter().filter_map(of_class).find(|c| !picked.contains(c)) {
            picked.push(c);
        }
    }
    picked
}

/// Builds the AI's board for a round, in enemy-side coordinates (the right half).
pub fn enemy_board(round: u32, rng: &mut Rng) -> Vec<(Pos, UnitSpec)> {
    let level = ai_level(round);
    let characters = pick_characters(level, level as usize, rng);
    let (twos, threes) = star_mix(round, characters.len());

    // Front columns face the player; melee units go there, ranged at the back.
    let rows = [2, 1, 3, 0, 4];
    let front = PLAYER_COLUMNS;
    let back = BOARD_WIDTH - 1;
    let melee: Vec<Pos> =
        (front..=back).flat_map(|x| rows.iter().map(move |&y| Pos::new(x, y))).collect();
    let ranged: Vec<Pos> =
        (front..=back).rev().flat_map(|x| rows.iter().map(move |&y| Pos::new(x, y))).collect();
    let mut used: Vec<Pos> = Vec::new();

    let mut board = Vec::new();
    for (i, &character) in characters.iter().enumerate() {
        let star = if i < threes {
            Level::Three
        } else if i < threes + twos {
            Level::Two
        } else {
            Level::One
        };
        let mut nodes = preset_program(character);
        nodes.truncate(star.max_nodes());
        let slots = if character.class().is_ranged() { &ranged } else { &melee };
        let pos = *slots.iter().find(|p| !used.contains(p)).expect("enemy half has room");
        used.push(pos);
        debug_assert!(pos.x >= PLAYER_COLUMNS && pos.y < BOARD_HEIGHT);
        board.push((pos, UnitSpec::new(character, star, Program::new(nodes))));
    }
    board
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Synergies, Unlocks};

    #[test]
    fn presets_are_valid_from_the_start_at_every_level() {
        for character in Character::ALL {
            for star in [Level::One, Level::Two, Level::Three] {
                let mut nodes = preset_program(character);
                nodes.truncate(star.max_nodes());
                let unlocks =
                    Unlocks { player_level: START_LEVEL, synergies: Synergies::default() };
                assert_eq!(
                    Program::new(nodes).validate(character, star, &unlocks),
                    Ok(()),
                    "{character:?} {star:?}"
                );
            }
        }
    }

    #[test]
    fn boards_grow_level_up_and_chase_a_synergy() {
        let mut rng = Rng::new(3);
        let early = enemy_board(1, &mut rng);
        let late = enemy_board(15, &mut rng);
        assert_eq!(early.len(), 3);
        assert!(early.iter().all(|(_, s)| s.level == Level::One));
        assert_eq!(late.len(), MAX_LEVEL as usize);
        assert!(late.iter().any(|(_, s)| s.level == Level::Three));
        let syn = Synergies::from_characters(late.iter().map(|(_, s)| s.character));
        assert!(syn.active_count() >= 1);
    }

    #[test]
    fn enemy_units_stay_on_their_half_never_overlap_or_repeat() {
        let mut rng = Rng::new(9);
        for round in 1..=15 {
            let board = enemy_board(round, &mut rng);
            for (i, (p, s)) in board.iter().enumerate() {
                assert!(p.x >= PLAYER_COLUMNS && p.x < BOARD_WIDTH);
                assert!(board[i + 1..].iter().all(|(q, t)| q != p && t.character != s.character));
            }
        }
    }
}

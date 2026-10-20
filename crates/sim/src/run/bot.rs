//! Computer players.
//!
//! A [`Bot`] is only a source of [`Command`]s: it reads its own [`Player`] (and
//! the round number) and sends commands through [`Lobby::apply`], the same path
//! the human's clicks take. It can't do anything a human couldn't, so bot boards
//! are always real, legal boards: they buy copies to merge, pay for XP and
//! reroll from the shared pool like everyone else.
//!
//! The strategy is deliberately simple and readable: pick up to two focus
//! classes, buy pairs, merges and focus units, level up on a schedule, keep some
//! gold for interest, and give every unit a preset program.

use super::economy::*;
use super::{Command, Lobby, Location, OwnedUnit, Player, PlayerId, RunError};
use crate::{
    Ability, Action, Character, Class, Condition::*, Item, Node, Program, Rng, Target, Threshold,
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

/// A bot's full program for a character, best cards first. Uses only blocks
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

/// The level a bot aims for in a round.
pub fn target_level(round: u32) -> u32 {
    (START_LEVEL + round / 3).min(MAX_LEVEL)
}

/// Rough strength of an owned unit, for choosing who plays.
fn power(unit: &OwnedUnit) -> u32 {
    unit.character.rarity().stat_percent() * unit.level.scale_percent() / 100
}

/// Most rerolls a bot makes in one planning phase.
const MAX_REROLLS: u32 = 4;

#[derive(Debug, Clone)]
pub struct Bot {
    /// Class preference, to break ties when choosing focus classes. Gives each
    /// bot a personality without any randomness during play.
    taste: [Class; 6],
}

impl Bot {
    pub fn new(seed: u64) -> Self {
        let mut taste = Class::ALL;
        Rng::new(seed).shuffle(&mut taste);
        Self { taste }
    }

    /// Plays one planning phase.
    pub fn plan(&mut self, lobby: &mut Lobby, id: PlayerId) {
        let round = lobby.round();
        let reserve = self.reserve(lobby.player(id), round);

        self.shop(lobby, id, reserve);
        while lobby.player(id).level() < target_level(round)
            && lobby.player(id).gold() >= XP_COST + reserve
        {
            if lobby.apply(id, Command::BuyXp).is_err() {
                break;
            }
        }
        if round >= 6 {
            for _ in 0..MAX_REROLLS {
                if lobby.player(id).gold() < reserve + REROLL_COST + 4 {
                    break;
                }
                let _ = lobby.apply(id, Command::Reroll);
                self.shop(lobby, id, reserve);
            }
        }
        self.arrange(lobby, id);
        self.trim_bench(lobby, id);
        self.program(lobby, id);
        self.equip(lobby, id);
    }

    /// Puts stashed items on board units with a free slot: defensive items on
    /// the toughest melee unit, range on a ranged one, anything else on the
    /// strongest unit. Simple on purpose; it goes through the same commands as
    /// a human, so it can't do anything a human couldn't.
    fn equip(&self, lobby: &mut Lobby, id: PlayerId) {
        // Each successful equip shifts the stash, so look again every time.
        for _ in 0..16 {
            let player = lobby.player(id);
            let Some(&item) = player.stash().first() else { return };
            let free = |u: &OwnedUnit| u.items.iter().any(Option::is_none);
            let mut board: Vec<(Location, &OwnedUnit)> = player
                .board()
                .iter()
                .filter(|(_, u)| free(u))
                .map(|(p, u)| (Location::Board(*p), u))
                .collect();
            if board.is_empty() {
                return;
            }
            board.sort_by_key(|(_, u)| std::cmp::Reverse(power(u)));
            let melee = |u: &OwnedUnit| !u.class().is_ranged();
            let defensive = matches!(
                item,
                Item::LeatherJerkin
                    | Item::HealingDraught
                    | Item::SpikedBuckler
                    | Item::PhoenixFeather
                    | Item::Aegis
            );
            let pick = if defensive {
                board.iter().find(|(_, u)| melee(u))
            } else if item == Item::Spyglass {
                board.iter().find(|(_, u)| !melee(u))
            } else {
                None
            }
            .or(board.first());
            let Some(&(loc, _)) = pick else { return };
            if lobby.apply(id, Command::Equip { item: 0, to: loc, slot: None }).is_err() {
                return;
            }
        }
    }

    /// Gold to keep for interest. Bots spend everything when they're in danger.
    fn reserve(&self, player: &Player, round: u32) -> u32 {
        if player.hp() < 35 {
            return 0;
        }
        match round {
            0..=4 => 0,
            5..=8 => 10,
            _ => 20,
        }
    }

    /// The two classes this bot is building towards: most characters owned,
    /// ties broken by taste.
    fn focus(&self, player: &Player) -> [Class; 2] {
        let mut owned: Vec<Character> = player.all_units().map(|u| u.character).collect();
        owned.sort();
        owned.dedup();
        let mut ranked = self.taste;
        // `sort_by_key` is stable, so equal counts keep their taste order.
        ranked.sort_by_key(|&c| std::cmp::Reverse(owned.iter().filter(|o| o.class() == c).count()));
        [ranked[0], ranked[1]]
    }

    /// How much the bot wants a shop offer (0 = not at all).
    fn want(&self, player: &Player, character: Character) -> u32 {
        let [first, second] = self.focus(player);
        let mut score = 0;
        if player.buy_would_merge(character) {
            score += 100;
        }
        if player.copies_owned(character) > 0 {
            score += 60;
        }
        if character.class() == first {
            score += 30;
        } else if character.class() == second {
            score += 20;
        }
        let on_board = player.board().iter().any(|(_, u)| u.character == character);
        if player.board().len() < player.board_limit() && !on_board {
            score += 25;
        }
        if score > 0 {
            score += 5 * character.rarity().cost();
        }
        score
    }

    /// Buys every offer worth having that the bot can afford.
    fn shop(&self, lobby: &mut Lobby, id: PlayerId, reserve: u32) {
        loop {
            let player = lobby.player(id);
            let best = player
                .shop()
                .iter()
                .enumerate()
                .filter_map(|(slot, offer)| offer.map(|c| (slot, c, self.want(player, c))))
                .filter(|&(_, c, want)| {
                    let cost = c.rarity().cost();
                    // Pairs and merges may dig into half the reserve.
                    let keep = if want >= 60 { reserve / 2 } else { reserve };
                    want >= 25 && player.gold() >= cost + keep
                })
                .max_by_key(|&(slot, _, want)| (want, std::cmp::Reverse(slot)));
            let Some((slot, character, want)) = best else { return };
            match lobby.apply(id, Command::Buy(slot)) {
                Ok(_) => {}
                Err(RunError::BenchFull) if want >= 60 => {
                    if !self.sell_weakest_single(lobby, id, character) {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    }

    /// Sells the weakest bench unit that isn't part of a pair. Returns false
    /// if there's nothing to sell.
    fn sell_weakest_single(&self, lobby: &mut Lobby, id: PlayerId, keep: Character) -> bool {
        let player = lobby.player(id);
        let victim = player
            .bench()
            .iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|u| (i, u)))
            .filter(|(_, u)| u.character != keep && player.copies_owned(u.character) == 1)
            .min_by_key(|(_, u)| power(u));
        match victim {
            Some((i, _)) => lobby.apply(id, Command::Sell(Location::Bench(i))).is_ok(),
            None => false,
        }
    }

    /// Fills the board from the bench, then swaps in stronger bench units.
    fn arrange(&self, lobby: &mut Lobby, id: PlayerId) {
        for _ in 0..MAX_LEVEL * 2 {
            let player = lobby.player(id);
            let [first, second] = self.focus(player);
            let value = |u: &OwnedUnit| {
                let focus = u.class() == first || u.class() == second;
                power(u) + if focus { 40 } else { 0 }
            };
            let on_board = |c: Character| player.board().iter().any(|(_, u)| u.character == c);
            let candidate = player
                .bench()
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.as_ref().map(|u| (i, u)))
                .filter(|(_, u)| !on_board(u.character))
                .max_by_key(|&(i, u)| (value(u), std::cmp::Reverse(i)));
            let Some((i, unit)) = candidate else { return };

            let to = if player.board().len() < player.board_limit() {
                match player.suggested_spot(unit.character) {
                    Some(p) => p,
                    None => return,
                }
            } else {
                let weakest = player.board().iter().min_by_key(|(p, u)| (value(u), p.x, p.y));
                match weakest {
                    Some((p, w)) if value(unit) > value(w) => *p,
                    _ => return,
                }
            };
            let from = Location::Bench(i);
            if lobby.apply(id, Command::Move { from, to: Location::Board(to) }).is_err() {
                return;
            }
        }
    }

    /// Sells bench units the bot has no use for, keeping a few spares.
    fn trim_bench(&self, lobby: &mut Lobby, id: PlayerId) {
        const KEEP: usize = 5;
        loop {
            let player = lobby.player(id);
            if player.bench().iter().flatten().count() <= KEEP {
                return;
            }
            let [first, second] = self.focus(player);
            let useless = player
                .bench()
                .iter()
                .enumerate()
                .filter_map(|(i, s)| s.as_ref().map(|u| (i, u)))
                .filter(|(_, u)| {
                    player.copies_owned(u.character) == u.level.copies()
                        && u.class() != first
                        && u.class() != second
                })
                .min_by_key(|(_, u)| power(u));
            let Some((i, _)) = useless else { return };
            if lobby.apply(id, Command::Sell(Location::Bench(i))).is_err() {
                return;
            }
        }
    }

    /// Gives every board unit its preset program, cut to its card slots.
    fn program(&self, lobby: &mut Lobby, id: PlayerId) {
        let player = lobby.player(id);
        let synergies = player.synergies();
        let wanted: Vec<(Location, Program)> = player
            .board()
            .iter()
            .filter_map(|(p, u)| {
                let mut nodes = preset_program(u.character);
                nodes.truncate(synergies.card_slots(u.character, u.level));
                let program = Program::new(nodes);
                (program != u.program).then_some((Location::Board(*p), program))
            })
            .collect();
        for (loc, program) in wanted {
            let _ = lobby.apply(id, Command::SetProgram(loc, program));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Synergies, Unlocks};

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
}

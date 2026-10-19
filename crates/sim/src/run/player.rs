//! One player's state in a lobby: health, gold, level, shop, bench and board.
//!
//! Humans and bots are the same [`Player`]; the only difference is where their
//! commands come from. Everything here can be read by anyone (that's how other
//! players' boards will be viewed), but it only changes through
//! [`Lobby::apply`](super::Lobby::apply) and the lobby's round resolution.

use super::economy::*;
use super::pool::Pool;
use super::{
    BattleSetup, Location, OwnedId, OwnedUnit, PlayerId, RoundReport, RunError, RunStatus,
};
use crate::synergy::Synergies;
use crate::{Character, Level, Pos, Program, Rarity, Rng, Team, UnitSpec, Unlocks};

#[derive(Debug, Clone)]
pub struct Player {
    id: PlayerId,
    name: &'static str,
    /// Drives this player's shop rolls.
    rng: Rng,
    hp: u32,
    gold: u32,
    level: u32,
    xp: u32,
    streak: i32,
    status: RunStatus,
    /// Final placement (1 = winner), once known.
    placement: Option<u32>,
    shop: [Option<Character>; SHOP_SIZE],
    bench: Vec<Option<OwnedUnit>>,
    board: Vec<(Pos, OwnedUnit)>,
    next_id: u32,
    history: Vec<(BattleSetup, RoundReport)>,
}

impl Player {
    /// A fresh player with a full shop drawn from `pool`.
    pub fn new(id: PlayerId, name: &'static str, seed: u64, pool: &mut Pool) -> Self {
        let mut player = Self {
            id,
            name,
            rng: Rng::new(seed),
            hp: START_HP,
            gold: START_GOLD,
            level: START_LEVEL,
            xp: 0,
            streak: 0,
            status: RunStatus::Planning,
            placement: None,
            shop: [None; SHOP_SIZE],
            bench: vec![None; BENCH_SIZE],
            board: Vec::new(),
            next_id: 0,
            history: Vec::new(),
        };
        player.refresh_shop(pool);
        player
    }

    // ----- Reading state -------------------------------------------------

    pub fn id(&self) -> PlayerId {
        self.id
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn hp(&self) -> u32 {
        self.hp
    }
    pub fn gold(&self) -> u32 {
        self.gold
    }
    /// The player's level, which is also the board size.
    pub fn level(&self) -> u32 {
        self.level
    }
    pub fn xp(&self) -> u32 {
        self.xp
    }
    /// XP needed for the next level, or `None` at the maximum.
    pub fn xp_needed(&self) -> Option<u32> {
        xp_to_next(self.level)
    }
    /// Positive for a win streak, negative for a loss streak.
    pub fn streak(&self) -> i32 {
        self.streak
    }
    pub fn status(&self) -> RunStatus {
        self.status
    }
    pub fn is_alive(&self) -> bool {
        self.status == RunStatus::Planning
    }
    /// Final placement (1 = winner), once the player is out or has won.
    pub fn placement(&self) -> Option<u32> {
        self.placement
    }
    pub fn shop(&self) -> &[Option<Character>; SHOP_SIZE] {
        &self.shop
    }
    pub fn bench(&self) -> &[Option<OwnedUnit>] {
        &self.bench
    }
    pub fn board(&self) -> &[(Pos, OwnedUnit)] {
        &self.board
    }
    pub fn board_limit(&self) -> usize {
        self.level as usize
    }
    /// Every round this player has fought, oldest first.
    pub fn history(&self) -> &[(BattleSetup, RoundReport)] {
        &self.history
    }
    /// Interest the player would earn if the round ended now.
    pub fn interest_preview(&self) -> u32 {
        interest(self.gold)
    }
    /// Synergies from the units on the board.
    pub fn synergies(&self) -> Synergies {
        Synergies::from_characters(self.board.iter().map(|(_, u)| u.character))
    }
    /// What the player can currently put in programs.
    pub fn unlocks(&self) -> Unlocks {
        Unlocks { player_level: self.level, synergies: self.synergies() }
    }

    pub fn unit_at(&self, loc: Location) -> Option<&OwnedUnit> {
        match loc {
            Location::Bench(i) => self.bench.get(i).and_then(Option::as_ref),
            Location::Board(p) => self.board.iter().find(|(q, _)| *q == p).map(|(_, u)| u),
        }
    }

    pub fn locate(&self, id: OwnedId) -> Option<Location> {
        if let Some((p, _)) = self.board.iter().find(|(_, u)| u.id == id) {
            return Some(Location::Board(*p));
        }
        self.bench.iter().position(|s| s.as_ref().is_some_and(|u| u.id == id)).map(Location::Bench)
    }

    /// Level 1 copies owned of a character, across board and bench.
    pub fn copies_owned(&self, character: Character) -> u32 {
        self.all_units().filter(|u| u.character == character).map(|u| u.level.copies()).sum()
    }

    /// True if buying this character now would complete a merge.
    pub fn buy_would_merge(&self, character: Character) -> bool {
        self.all_units().filter(|u| u.character == character && u.level == Level::One).count() == 2
    }

    /// True if placing this character would activate (or raise) a synergy.
    pub fn would_raise_synergy(&self, character: Character) -> bool {
        if self.board.iter().any(|(_, u)| u.character == character) {
            return false;
        }
        let now = self.synergies().count(character.class());
        now == 1 || now == 2
    }

    /// Where a character would be placed automatically: the first free cell,
    /// melee from the front column back, ranged from the back column forward.
    pub fn suggested_spot(&self, character: Character) -> Option<Pos> {
        let rows = [2, 1, 3, 0, 4];
        let columns: Vec<i32> = if character.class().is_ranged() {
            (0..PLAYER_COLUMNS).collect()
        } else {
            (0..PLAYER_COLUMNS).rev().collect()
        };
        columns
            .iter()
            .flat_map(|&x| rows.iter().map(move |&y| Pos::new(x, y)))
            .find(|&p| self.unit_at(Location::Board(p)).is_none())
    }

    /// The board as battle units, in a stable order (spawn order decides turn order).
    pub fn board_specs(&self) -> Vec<(Pos, UnitSpec)> {
        let mut specs: Vec<(Pos, UnitSpec)> =
            self.board.iter().map(|(p, u)| (*p, u.spec())).collect();
        specs.sort_by_key(|(p, _)| (p.x, p.y));
        specs
    }

    pub fn all_units(&self) -> impl Iterator<Item = &OwnedUnit> {
        self.board.iter().map(|(_, u)| u).chain(self.bench.iter().flatten())
    }

    // ----- Commands (called through the lobby) ---------------------------

    /// Buys a shop offer. The unit lands on the bench, merges if it completes a
    /// set of three, and moves onto the board if there's room for it there.
    pub(super) fn buy(&mut self, slot: usize) -> Result<OwnedId, RunError> {
        let character = self.shop.get(slot).copied().flatten().ok_or(RunError::NoOffer(slot))?;
        let cost = character.rarity().cost();
        self.spend_check(cost)?;
        let free = self.bench.iter().position(Option::is_none);
        if free.is_none() && !self.buy_would_merge(character) {
            return Err(RunError::BenchFull);
        }

        self.gold -= cost;
        self.shop[slot] = None;
        let id = self.new_id();
        let unit = OwnedUnit { id, character, level: Level::One, program: Program::default() };
        match free {
            Some(i) => self.bench[i] = Some(unit),
            // A full bench is fine when this copy merges straight away: park it in
            // a temporary extra slot, which the merge then consumes.
            None => self.bench.push(Some(unit)),
        }
        let result = self.merge_all().unwrap_or(id);
        self.bench.truncate(BENCH_SIZE);
        self.auto_place(result);
        Ok(result)
    }

    /// Moves a bench unit onto the board if there's room and its character
    /// isn't already there.
    fn auto_place(&mut self, id: OwnedId) {
        let Some(from @ Location::Bench(_)) = self.locate(id) else { return };
        let character = self.unit_at(from).expect("just located").character;
        if self.board.len() >= self.board_limit()
            || self.board.iter().any(|(_, u)| u.character == character)
        {
            return;
        }
        if let Some(p) = self.suggested_spot(character) {
            let unit = self.take(from).expect("located");
            self.put(Location::Board(p), unit);
        }
    }

    pub(super) fn reroll(&mut self, pool: &mut Pool) -> Result<(), RunError> {
        self.spend_check(REROLL_COST)?;
        self.gold -= REROLL_COST;
        self.refresh_shop(pool);
        Ok(())
    }

    /// Buys XP. Returns the levels reached, if any.
    pub(super) fn buy_xp(&mut self) -> Result<Vec<u32>, RunError> {
        if self.level >= MAX_LEVEL {
            return Err(RunError::MaxLevel);
        }
        self.spend_check(XP_COST)?;
        self.gold -= XP_COST;
        Ok(self.gain_xp(XP_PER_BUY))
    }

    fn gain_xp(&mut self, amount: u32) -> Vec<u32> {
        let mut reached = Vec::new();
        self.xp += amount;
        while let Some(needed) = xp_to_next(self.level) {
            if self.xp < needed {
                break;
            }
            self.xp -= needed;
            self.level += 1;
            reached.push(self.level);
        }
        if self.level >= MAX_LEVEL {
            self.xp = 0;
        }
        reached
    }

    /// Sells a unit for the full price of the copies it's made of.
    pub(super) fn sell(&mut self, loc: Location, pool: &mut Pool) -> Result<u32, RunError> {
        let unit = self.take(loc).ok_or(RunError::Empty(loc))?;
        pool.give_back(unit.character, unit.level.copies());
        let refund = unit.sell_value();
        self.gold += refund;
        Ok(refund)
    }

    /// Returns unbought offers to the pool and draws a fresh shop: each slot
    /// rolls a rarity with this level's odds, then a copy of that rarity.
    pub(super) fn refresh_shop(&mut self, pool: &mut Pool) {
        self.return_shop(pool);
        let odds = shop_odds(self.level);
        for slot in 0..SHOP_SIZE {
            let mut roll = self.rng.below(100);
            let mut rarity = Rarity::Common;
            for r in Rarity::ALL {
                if roll < odds[r.index()] {
                    rarity = r;
                    break;
                }
                roll -= odds[r.index()];
            }
            // Fall back to another rarity if this one has run dry.
            let order = match rarity {
                Rarity::Common => [Rarity::Common, Rarity::Rare, Rarity::Epic],
                Rarity::Rare => [Rarity::Rare, Rarity::Common, Rarity::Epic],
                Rarity::Epic => [Rarity::Epic, Rarity::Rare, Rarity::Common],
            };
            self.shop[slot] = order.into_iter().find_map(|r| pool.draw(r, &mut self.rng));
        }
    }

    fn return_shop(&mut self, pool: &mut Pool) {
        for offer in self.shop.iter_mut() {
            if let Some(c) = offer.take() {
                pool.give_back(c, 1);
            }
        }
    }

    /// Moves a unit. Moving onto an occupied spot swaps the two units.
    pub(super) fn move_unit(&mut self, from: Location, to: Location) -> Result<(), RunError> {
        if from == to {
            return Ok(());
        }
        let moving = self.unit_at(from).ok_or(RunError::Empty(from))?.clone();
        let displaced = self.unit_at(to).cloned();
        if let Location::Board(p) = to
            && (!(0..PLAYER_COLUMNS).contains(&p.x) || !(0..BOARD_HEIGHT).contains(&p.y))
        {
            return Err(RunError::NotPlayerSide(p));
        }
        if let Location::Bench(i) = to
            && i >= BENCH_SIZE
        {
            return Err(RunError::BenchFull);
        }

        // What the board would hold afterwards, to check the board rules.
        let on_board = |loc: Location| matches!(loc, Location::Board(_));
        let mut after: Vec<Character> = self
            .board
            .iter()
            .filter(|(p, _)| Location::Board(*p) != from && Location::Board(*p) != to)
            .map(|(_, u)| u.character)
            .collect();
        if on_board(to) {
            after.push(moving.character);
        }
        if let (Some(d), true) = (&displaced, on_board(from)) {
            after.push(d.character);
        }
        if after.len() > self.board_limit() && after.len() > self.board.len() {
            return Err(RunError::BoardFull { max: self.board_limit() });
        }
        let mut seen = Vec::new();
        for c in after {
            if seen.contains(&c) {
                return Err(RunError::AlreadyOnBoard(c));
            }
            seen.push(c);
        }

        let a = self.take(from).expect("checked above");
        let b = self.take(to);
        self.put(to, a);
        if let Some(b) = b {
            self.put(from, b);
        }
        Ok(())
    }

    /// Replaces a unit's program, after checking its card slots and that every
    /// block is unlocked.
    pub(super) fn set_program(&mut self, loc: Location, program: Program) -> Result<(), RunError> {
        let unlocks = self.unlocks();
        let unit = self.unit_at(loc).ok_or(RunError::Empty(loc))?;
        program.validate(unit.character, unit.level, &unlocks)?;
        self.unit_mut(loc).expect("checked").program = program;
        Ok(())
    }

    // ----- Round resolution (called by the lobby) ------------------------

    /// Applies a round's result: damage, streak, gold, XP. Returns the report,
    /// which is also stored in the history with the battle's setup.
    pub(super) fn settle(&mut self, outcome: RoundOutcome, setup: BattleSetup) -> RoundReport {
        let RoundOutcome { round, opponent, side, result, ticks, survivors, damage, reward } =
            outcome;
        self.hp = self.hp.saturating_sub(damage);
        // PvE rounds leave streaks alone.
        if !opponent.is_pve() {
            self.streak = match result {
                Some(true) => self.streak.max(0) + 1,
                Some(false) => self.streak.min(0) - 1,
                None => 0,
            };
        }
        let banked = self.gold;
        let income = Income {
            base: BASE_INCOME,
            win: reward,
            interest: interest(banked),
            streak: streak_bonus(self.streak.unsigned_abs()),
        };
        self.gold += income.total();
        let level_ups =
            if self.level < MAX_LEVEL { self.gain_xp(XP_PER_ROUND) } else { Vec::new() };
        let report = RoundReport {
            round,
            opponent,
            side,
            result,
            ticks,
            player_survivors: survivors,
            damage,
            hp_after: self.hp,
            banked,
            income,
            gold_after: self.gold,
            streak: self.streak,
            level_ups,
            placement: None,
        };
        self.history.push((setup, report.clone()));
        report
    }

    /// Knocks the player out: every copy they hold goes back to the pool.
    pub(super) fn eliminate(&mut self, placement: u32, pool: &mut Pool) {
        self.status = RunStatus::Defeat;
        self.placement = Some(placement);
        self.return_shop(pool);
        for unit in
            self.board.drain(..).map(|(_, u)| u).chain(self.bench.iter_mut().flat_map(Option::take))
        {
            pool.give_back(unit.character, unit.level.copies());
        }
        if let Some((_, report)) = self.history.last_mut() {
            report.placement = Some(placement);
        }
    }

    pub(super) fn win(&mut self) {
        self.status = RunStatus::Victory;
        self.placement = Some(1);
        if let Some((_, report)) = self.history.last_mut() {
            report.placement = Some(1);
        }
    }

    // ----- Internals -----------------------------------------------------

    fn spend_check(&self, cost: u32) -> Result<(), RunError> {
        if self.gold >= cost {
            Ok(())
        } else {
            Err(RunError::NotEnoughGold { need: cost, have: self.gold })
        }
    }

    fn new_id(&mut self) -> OwnedId {
        self.next_id += 1;
        OwnedId(self.next_id)
    }

    fn unit_mut(&mut self, loc: Location) -> Option<&mut OwnedUnit> {
        match loc {
            Location::Bench(i) => self.bench.get_mut(i).and_then(Option::as_mut),
            Location::Board(p) => self.board.iter_mut().find(|(q, _)| *q == p).map(|(_, u)| u),
        }
    }

    fn take(&mut self, loc: Location) -> Option<OwnedUnit> {
        match loc {
            Location::Bench(i) => self.bench.get_mut(i).and_then(Option::take),
            Location::Board(p) => {
                let i = self.board.iter().position(|(q, _)| *q == p)?;
                Some(self.board.remove(i).1)
            }
        }
    }

    fn put(&mut self, loc: Location, unit: OwnedUnit) {
        match loc {
            Location::Bench(i) => self.bench[i] = Some(unit),
            Location::Board(p) => self.board.push((p, unit)),
        }
    }

    /// Merges every set of three same-character, same-level units, cascading
    /// (three level 2s made by a merge immediately become a level 3).
    /// Returns the id of the last unit produced, if any merge happened.
    fn merge_all(&mut self) -> Option<OwnedId> {
        let mut produced = None;
        'again: loop {
            for character in Character::ALL {
                for level in [Level::One, Level::Two] {
                    let mut locs: Vec<Location> = self
                        .board
                        .iter()
                        .filter(|(_, u)| u.character == character && u.level == level)
                        .map(|(p, _)| Location::Board(*p))
                        .collect();
                    locs.extend(self.bench.iter().enumerate().filter_map(|(i, s)| {
                        s.as_ref()
                            .filter(|u| u.character == character && u.level == level)
                            .map(|_| Location::Bench(i))
                    }));
                    if locs.len() < 3 {
                        continue;
                    }
                    // Board copies come first, so the board unit (and its program)
                    // is the one that survives.
                    let target = locs[0];
                    for &loc in &locs[1..3] {
                        self.take(loc);
                    }
                    let on_board = matches!(target, Location::Board(_));
                    let unit = self.unit_mut(target).expect("target exists");
                    unit.level = level.next().expect("only levels 1 and 2 merge");
                    if !on_board {
                        unit.program = Program::default();
                    }
                    produced = Some(unit.id);
                    continue 'again;
                }
            }
            return produced;
        }
    }

    // ----- Test helpers --------------------------------------------------

    #[cfg(test)]
    pub(super) fn set_gold(&mut self, gold: u32) {
        self.gold = gold;
    }

    #[cfg(test)]
    pub(super) fn set_shop(&mut self, character: Character) {
        self.shop = [Some(character); SHOP_SIZE];
    }

    #[cfg(test)]
    pub(super) fn set_level(&mut self, level: u32, xp: u32) {
        self.level = level;
        self.xp = xp;
    }

    #[cfg(test)]
    pub(super) fn set_hp(&mut self, hp: u32) {
        self.hp = hp;
    }
}

/// Everything [`Player::settle`] needs to know about a finished round.
pub(super) struct RoundOutcome {
    pub round: u32,
    pub opponent: super::Opponent,
    /// The side this player fought on.
    pub side: Team,
    /// `Some(true)` won, `Some(false)` lost, `None` drawn.
    pub result: Option<bool>,
    pub ticks: u32,
    pub survivors: usize,
    pub damage: u32,
    /// Gold for winning (win bonus, or a PvE reward).
    pub reward: u32,
}

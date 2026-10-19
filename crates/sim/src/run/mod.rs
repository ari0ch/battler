//! A whole run: rounds of planning and battles against AI boards.
//!
//! The game crate drives a run through the commands here (buy, sell, reroll,
//! buy XP, move, set a program, start a battle, resolve it) and only displays
//! the state. All randomness comes from the run's seed, so a run is reproducible.

pub mod ai;
pub mod economy;

#[cfg(test)]
mod tests;

use std::fmt;

use crate::program::ProgramError;
use crate::synergy::Synergies;
use crate::{
    Battle, Character, Class, Level, Outcome, Pos, Program, Rarity, Rng, Team, UnitSpec, Unlocks,
};
use economy::*;

/// Identifies one owned unit for the whole run, even as it moves around.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OwnedId(pub u32);

/// A unit the player owns, on the board or the bench.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedUnit {
    pub id: OwnedId,
    pub character: Character,
    pub level: Level,
    pub program: Program,
}

impl OwnedUnit {
    pub fn class(&self) -> Class {
        self.character.class()
    }

    pub fn spec(&self) -> UnitSpec {
        UnitSpec::new(self.character, self.level, self.program.clone())
    }

    /// Gold returned when sold: the price of every copy inside.
    pub fn sell_value(&self) -> u32 {
        self.level.copies() * self.character.rarity().cost()
    }
}

/// Where a unit sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Location {
    Bench(usize),
    Board(Pos),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Planning between battles (also the state while a battle is being watched).
    Planning,
    Victory,
    Defeat,
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunError {
    RunOver,
    NotEnoughGold {
        need: u32,
        have: u32,
    },
    NoOffer(usize),
    BenchFull,
    BoardFull {
        max: usize,
    },
    /// Only one copy of each character may be on the board.
    AlreadyOnBoard(Character),
    NotPlayerSide(Pos),
    Empty(Location),
    MaxLevel,
    InvalidProgram(ProgramError),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::RunOver => write!(f, "the run is over"),
            RunError::NotEnoughGold { need, have } => write!(f, "need {need} gold, have {have}"),
            RunError::NoOffer(i) => write!(f, "shop slot {} is empty", i + 1),
            RunError::BenchFull => write!(f, "the bench is full"),
            RunError::BoardFull { max } => {
                write!(f, "the board holds {max} units at this level - buy XP to grow it")
            }
            RunError::AlreadyOnBoard(c) => write!(f, "{} is already on the board", c.name()),
            RunError::NotPlayerSide(_) => write!(f, "units can only go on your side"),
            RunError::Empty(_) => write!(f, "there's no unit there"),
            RunError::MaxLevel => write!(f, "already at the maximum level"),
            RunError::InvalidProgram(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RunError {}

impl From<ProgramError> for RunError {
    fn from(e: ProgramError) -> Self {
        RunError::InvalidProgram(e)
    }
}

/// Everything needed to (re)create one battle: both boards and the seed.
/// Stored per round, so any battle can be replayed exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleSetup {
    pub round: u32,
    pub seed: u64,
    pub player: Vec<(Pos, UnitSpec)>,
    pub enemy: Vec<(Pos, UnitSpec)>,
}

impl BattleSetup {
    pub fn build(&self) -> Battle {
        let mut battle = Battle::new(BOARD_WIDTH, BOARD_HEIGHT, MAX_TICKS, self.seed);
        let units = self
            .player
            .iter()
            .map(|u| (Team::Player, u))
            .chain(self.enemy.iter().map(|u| (Team::Enemy, u)));
        for (team, (pos, spec)) in units {
            battle.spawn(team, *pos, spec.clone()).expect("setups only hold valid units");
        }
        battle
    }
}

/// What happened in a round, for the result screen and the run summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundReport {
    pub round: u32,
    pub outcome: Outcome,
    pub ticks: u32,
    pub player_survivors: usize,
    pub damage: u32,
    pub hp_after: u32,
    pub banked: u32,
    pub income: Income,
    pub gold_after: u32,
    /// Positive for a win streak, negative for a loss streak.
    pub streak: i32,
    /// Player levels reached thanks to this round's XP.
    pub level_ups: Vec<u32>,
    pub status: RunStatus,
}

#[derive(Debug, Clone)]
pub struct Run {
    seed: u64,
    rng: Rng,
    round: u32,
    hp: u32,
    gold: u32,
    level: u32,
    xp: u32,
    streak: i32,
    status: RunStatus,
    pool: [u32; Character::COUNT],
    shop: [Option<Character>; SHOP_SIZE],
    bench: Vec<Option<OwnedUnit>>,
    board: Vec<(Pos, OwnedUnit)>,
    next_id: u32,
    history: Vec<(BattleSetup, RoundReport)>,
    pending: Option<BattleSetup>,
}

impl Run {
    pub fn new(seed: u64) -> Self {
        let mut pool = [0; Character::COUNT];
        for c in Character::ALL {
            pool[c.index()] = c.rarity().pool_copies();
        }
        let mut run = Self {
            seed,
            rng: Rng::new(seed),
            round: 1,
            hp: START_HP,
            gold: START_GOLD,
            level: START_LEVEL,
            xp: 0,
            streak: 0,
            status: RunStatus::Planning,
            pool,
            shop: [None; SHOP_SIZE],
            bench: vec![None; BENCH_SIZE],
            board: Vec::new(),
            next_id: 0,
            history: Vec::new(),
            pending: None,
        };
        run.refresh_shop();
        run
    }

    // ----- Reading state -------------------------------------------------

    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn round(&self) -> u32 {
        self.round
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
    pub fn history(&self) -> &[(BattleSetup, RoundReport)] {
        &self.history
    }
    /// Copies of a character left in the shared pool.
    pub fn pool_remaining(&self, character: Character) -> u32 {
        self.pool[character.index()]
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
        let class = character.class();
        let now = self.synergies().count(class);
        now == 1 || now == 2
    }

    fn all_units(&self) -> impl Iterator<Item = &OwnedUnit> {
        self.board.iter().map(|(_, u)| u).chain(self.bench.iter().flatten())
    }

    // ----- Shop and XP ---------------------------------------------------

    /// Buys a shop offer. The unit lands on the bench, merges if it completes a
    /// set of three, and moves onto the board if there's room for it there.
    pub fn buy(&mut self, slot: usize) -> Result<OwnedId, RunError> {
        self.check_planning()?;
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
    /// isn't already there: melee to the front, ranged to the back.
    fn auto_place(&mut self, id: OwnedId) {
        let Some(from @ Location::Bench(_)) = self.locate(id) else { return };
        let character = self.unit_at(from).expect("just located").character;
        if self.board.len() >= self.board_limit()
            || self.board.iter().any(|(_, u)| u.character == character)
        {
            return;
        }
        let rows = [2, 1, 3, 0, 4];
        let columns: Vec<i32> = if character.class().is_ranged() {
            (0..PLAYER_COLUMNS).collect()
        } else {
            (0..PLAYER_COLUMNS).rev().collect()
        };
        let spot = columns
            .iter()
            .flat_map(|&x| rows.iter().map(move |&y| Pos::new(x, y)))
            .find(|&p| self.unit_at(Location::Board(p)).is_none());
        if let Some(p) = spot {
            let unit = self.take(from).expect("located");
            self.put(Location::Board(p), unit);
        }
    }

    pub fn reroll(&mut self) -> Result<(), RunError> {
        self.check_planning()?;
        self.spend_check(REROLL_COST)?;
        self.gold -= REROLL_COST;
        self.refresh_shop();
        Ok(())
    }

    /// Buys XP. Returns the levels reached, if any.
    pub fn buy_xp(&mut self) -> Result<Vec<u32>, RunError> {
        self.check_planning()?;
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
    pub fn sell(&mut self, loc: Location) -> Result<u32, RunError> {
        self.check_planning()?;
        let unit = self.take(loc).ok_or(RunError::Empty(loc))?;
        self.pool[unit.character.index()] += unit.level.copies();
        let refund = unit.sell_value();
        self.gold += refund;
        Ok(refund)
    }

    /// Returns unbought offers to the pool and draws a fresh shop: each slot
    /// rolls a rarity with this level's odds, then a copy of that rarity.
    fn refresh_shop(&mut self) {
        for offer in self.shop.iter_mut() {
            if let Some(c) = offer.take() {
                self.pool[c.index()] += 1;
            }
        }
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
            for r in order {
                if let Some(c) = self.draw_of_rarity(r) {
                    self.shop[slot] = Some(c);
                    break;
                }
            }
        }
    }

    /// Draws one copy of the given rarity, weighted by copies left.
    fn draw_of_rarity(&mut self, rarity: Rarity) -> Option<Character> {
        let candidates = Character::ALL.iter().filter(|c| c.rarity() == rarity);
        let total: u32 = candidates.clone().map(|c| self.pool[c.index()]).sum();
        if total == 0 {
            return None;
        }
        let mut pick = self.rng.below(total);
        for &c in candidates {
            let left = self.pool[c.index()];
            if pick < left {
                self.pool[c.index()] -= 1;
                return Some(c);
            }
            pick -= left;
        }
        None
    }

    // ----- Placement and programs ---------------------------------------

    /// Moves a unit. Moving onto an occupied spot swaps the two units.
    pub fn move_unit(&mut self, from: Location, to: Location) -> Result<(), RunError> {
        self.check_planning()?;
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
    pub fn set_program(&mut self, loc: Location, program: Program) -> Result<(), RunError> {
        self.check_planning()?;
        let unlocks = self.unlocks();
        let unit = self.unit_at(loc).ok_or(RunError::Empty(loc))?;
        program.validate(unit.character, unit.level, &unlocks)?;
        self.unit_mut(loc).expect("checked").program = program;
        Ok(())
    }

    // ----- Battles -------------------------------------------------------

    /// Locks in the boards for this round's battle. The caller plays it (step by
    /// step to animate, or all at once) and then calls [`Run::resolve`].
    pub fn start_battle(&mut self) -> Result<BattleSetup, RunError> {
        self.check_planning()?;
        if let Some(setup) = &self.pending {
            return Ok(setup.clone());
        }
        let seed = self.rng.next_u64();
        let enemy = ai::enemy_board(self.round, &mut self.rng);
        let mut player: Vec<(Pos, UnitSpec)> =
            self.board.iter().map(|(p, u)| (*p, u.spec())).collect();
        // Stable order: spawn order decides unit ids, and so turn order.
        player.sort_by_key(|(p, _)| (p.x, p.y));
        let setup = BattleSetup { round: self.round, seed, player, enemy };
        self.pending = Some(setup.clone());
        Ok(setup)
    }

    /// Applies a finished battle: damage, gold, XP, streak, next round.
    ///
    /// # Panics
    /// If no battle was started, or the battle isn't finished.
    pub fn resolve(&mut self, battle: &Battle) -> RoundReport {
        let setup = self.pending.take().expect("resolve() without start_battle()");
        assert_ne!(battle.outcome(), Outcome::Ongoing, "resolve() on an unfinished battle");

        let outcome = battle.outcome();
        let alive = |team| battle.units().iter().filter(move |u| u.is_alive() && u.team == team);
        let enemy_levels: u32 = alive(Team::Enemy).map(|u| u.level.number()).sum();
        let player_survivors = alive(Team::Player).count();

        let won = outcome == Outcome::Won(Team::Player);
        let lost = outcome == Outcome::Won(Team::Enemy);
        let damage = if lost { loss_damage(enemy_levels) } else { 0 };
        self.hp = self.hp.saturating_sub(damage);
        self.streak = match (won, lost) {
            (true, _) => self.streak.max(0) + 1,
            (_, true) => self.streak.min(0) - 1,
            _ => 0,
        };

        let banked = self.gold;
        let income = Income {
            base: BASE_INCOME,
            win: if won { WIN_BONUS } else { 0 },
            interest: interest(banked),
            streak: streak_bonus(self.streak.unsigned_abs()),
        };
        self.gold += income.total();
        let level_ups =
            if self.level < MAX_LEVEL { self.gain_xp(XP_PER_ROUND) } else { Vec::new() };

        self.status = if self.hp == 0 {
            RunStatus::Defeat
        } else if self.round >= FINAL_ROUND {
            RunStatus::Victory
        } else {
            RunStatus::Planning
        };
        let report = RoundReport {
            round: self.round,
            outcome,
            ticks: battle.tick(),
            player_survivors,
            damage,
            hp_after: self.hp,
            banked,
            income,
            gold_after: self.gold,
            streak: self.streak,
            level_ups,
            status: self.status,
        };
        self.history.push((setup, report.clone()));
        if self.status == RunStatus::Planning {
            self.round += 1;
            self.refresh_shop();
        }
        report
    }

    // ----- Internals -----------------------------------------------------

    fn check_planning(&self) -> Result<(), RunError> {
        if self.status == RunStatus::Planning { Ok(()) } else { Err(RunError::RunOver) }
    }

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
}

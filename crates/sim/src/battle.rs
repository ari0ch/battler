use std::fmt;

use crate::program::ProgramError;
use crate::synergy::Synergies;
use crate::unit::ItemsSpent;
use crate::{
    Ability, CardRef, Class, Item, Passive, Pos, Rng, Signal, Statuses, Team, Unit, UnitId,
    UnitSpec,
};

/// Chance (in percent) that any single hit is a critical strike.
pub const CRIT_CHANCE_PERCENT: u32 = 15;
/// Critical strikes deal this percentage of normal damage.
pub const CRIT_DAMAGE_PERCENT: u32 = 150;

/// Something that happened during a tick. The full event log doubles as the
/// replay: the renderer and the replay timeline only ever read events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Which card a unit ran this tick. Always precedes that card's effects.
    CardFired {
        unit: UnitId,
        card: CardRef,
    },
    Moved {
        unit: UnitId,
        from: Pos,
        to: Pos,
    },
    Teleported {
        unit: UnitId,
        from: Pos,
        to: Pos,
    },
    Attacked {
        attacker: UnitId,
        target: UnitId,
        damage: u32,
        crit: bool,
    },
    Healed {
        healer: UnitId,
        target: UnitId,
        amount: u32,
    },
    Guarded {
        unit: UnitId,
    },
    StatusApplied {
        unit: UnitId,
        status: StatusKind,
        until: u32,
    },
    SignalSent {
        unit: UnitId,
        signal: Signal,
    },
    AbilityUsed {
        unit: UnitId,
        ability: Ability,
    },
    /// A frozen unit lost its turn.
    TurnSkipped {
        unit: UnitId,
    },
    Died {
        unit: UnitId,
    },
    /// Brought back by Resurrection.
    Revived {
        unit: UnitId,
        hp: u32,
    },
    /// Survived a killing blow (Undying).
    CheatedDeath {
        unit: UnitId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Shielded,
    Immune,
    Frenzied,
    Frozen,
    Marked,
    Taunted { by: UnitId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Won(Team),
    /// Both sides wiped out, or the tick limit was reached.
    Draw,
}

/// Why a unit couldn't be added to a battle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnError {
    OutOfBounds(Pos),
    Occupied(Pos),
    InvalidProgram(ProgramError),
}

impl fmt::Display for SpawnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpawnError::OutOfBounds(p) => write!(f, "position ({}, {}) is off the grid", p.x, p.y),
            SpawnError::Occupied(p) => write!(f, "position ({}, {}) is already taken", p.x, p.y),
            SpawnError::InvalidProgram(e) => write!(f, "invalid program: {e}"),
        }
    }
}

impl std::error::Error for SpawnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SpawnError::InvalidProgram(e) => Some(e),
            _ => None,
        }
    }
}

// `From` lets `?` convert a `ProgramError` into a `SpawnError` automatically.
impl From<ProgramError> for SpawnError {
    fn from(e: ProgramError) -> Self {
        SpawnError::InvalidProgram(e)
    }
}

#[derive(Debug, Clone)]
pub struct Battle {
    pub(crate) width: i32,
    pub(crate) height: i32,
    max_ticks: u32,
    pub(crate) tick: u32,
    next_id: u32,
    /// Kept in ascending id order; dead units stay so ids remain valid.
    pub(crate) units: Vec<Unit>,
    log: Vec<(u32, Event)>,
    outcome: Outcome,
    pub(crate) rng: Rng,
    pub(crate) crit_override: Option<u32>,
    /// Signals sent this tick, per team (bit set), delivered next tick.
    pub(crate) signals_pending: [u8; 2],
    /// Signals teammates can see this tick.
    pub(crate) signals_active: [u8; 2],
    /// Each team's synergies, fixed when the battle starts.
    pub(crate) synergies: [Synergies; 2],
    /// The enemy each team's Rangers hit most recently (Rangers synergy target).
    pub(crate) ranger_mark: [Option<UnitId>; 2],
    /// Whether each team's Resurrection has been used.
    pub(crate) resurrection_used: [bool; 2],
    /// Set while an Arcane Surge ability resolves: its hits crit.
    /// The unit the running card's condition spotted: the action's `it`.
    /// Set only while that card's action runs.
    pub(crate) it: Option<usize>,
    pub(crate) force_crit: bool,
    started: bool,
}

impl Battle {
    /// Creates an empty battle. The `seed` drives all randomness (crits), so the
    /// same units, programs and seed always replay identically.
    pub fn new(width: i32, height: i32, max_ticks: u32, seed: u64) -> Self {
        Self {
            width,
            height,
            max_ticks,
            tick: 0,
            next_id: 0,
            units: Vec::new(),
            log: Vec::new(),
            outcome: Outcome::Ongoing,
            rng: Rng::new(seed),
            crit_override: None,
            signals_pending: [0; 2],
            signals_active: [0; 2],
            synergies: [Synergies::default(); 2],
            ranger_mark: [None; 2],
            resurrection_used: [false; 2],
            force_crit: false,
            it: None,
            started: false,
        }
    }

    /// Adds a unit before the battle starts.
    ///
    /// Only basic program checks happen here (card count and signatures); unlock
    /// rules are the run's business. Cards whose synergy isn't active in this
    /// battle, or beyond the unit's card slots, are simply skipped.
    pub fn spawn(&mut self, team: Team, pos: Pos, spec: UnitSpec) -> Result<UnitId, SpawnError> {
        if !self.in_bounds(pos) {
            return Err(SpawnError::OutOfBounds(pos));
        }
        if self.is_occupied(pos) {
            return Err(SpawnError::Occupied(pos));
        }
        spec.program.validate_basic(spec.kind)?;

        let mut stats = spec.level.scale(spec.kind.base_stats());
        let boost = 100 + spec.boost_percent;
        stats.max_hp = stats.max_hp * boost / 100;
        stats.attack = stats.attack * boost / 100;
        // Stat items.
        let has = |item| crate::item::has(&spec.items, item);
        let mut crit_chance = CRIT_CHANCE_PERCENT;
        if has(Item::IronSword) {
            stats.attack = stats.attack * 115 / 100;
        }
        if has(Item::LeatherJerkin) {
            stats.max_hp = stats.max_hp * 120 / 100;
        }
        if has(Item::LuckyCoin) {
            crit_chance += 10;
        }
        if has(Item::Spyglass) {
            stats.range += 1;
        }
        let id = UnitId(self.next_id);
        self.next_id += 1;
        self.units.push(Unit {
            id,
            team,
            kind: spec.kind,
            class: spec.kind.class(),
            level: spec.level,
            stats,
            pos,
            hp: stats.max_hp,
            program: spec.program,
            statuses: Statuses::default(),
            crit_chance,
            items: spec.items,
            items_spent: ItemsSpent::default(),
            ready_at: [0; Ability::COUNT],
            abilities_used: 0,
            cheated_death: false,
            #[cfg(test)]
            inert: false,
        });
        Ok(id)
    }

    /// Overrides every unit's crit chance (default [`CRIT_CHANCE_PERCENT`]).
    /// Setting it to 0 removes all randomness from the battle.
    pub fn set_crit_chance(&mut self, percent: u32) {
        self.crit_override = Some(percent.min(100));
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn tick(&self) -> u32 {
        self.tick
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn outcome(&self) -> Outcome {
        self.outcome
    }

    /// A team's synergies (computed when the battle starts; before that, from
    /// the units spawned so far).
    pub fn synergies(&self, team: Team) -> Synergies {
        if self.started { self.synergies[team.index()] } else { self.compute_synergies(team) }
    }

    /// Every event so far, tagged with the tick it happened on.
    pub fn log(&self) -> &[(u32, Event)] {
        &self.log
    }

    fn compute_synergies(&self, team: Team) -> Synergies {
        Synergies::from_members(
            self.units.iter().filter(|u| u.team == team).filter_map(|u| {
                u.kind.character().map(|c| (c, crate::item::banner_class(&u.items)))
            }),
        )
    }

    /// Fixes synergies and applies battle-start effects.
    fn begin(&mut self) {
        self.started = true;
        for team in [Team::Player, Team::Enemy] {
            self.synergies[team.index()] = self.compute_synergies(team);
        }
        for unit in &mut self.units {
            let syn = self.synergies[unit.team.index()];
            if unit.class == Class::Ranger && syn.full(Class::Ranger) {
                unit.stats.range += 1;
            }
            if unit.class == Class::Rogue && syn.full(Class::Rogue) {
                unit.crit_chance *= 2;
            }
            if let Some(c) = self.crit_override {
                unit.crit_chance = c;
            }
            if unit.has_passive(Passive::Vanish) {
                unit.statuses.untargetable_until = 2;
            }
            if unit.has_item(Item::Aegis) {
                unit.statuses.immune_until = 2;
            }
        }
    }

    /// Advances one tick and returns the events it produced.
    /// Does nothing once the battle is over.
    pub fn step(&mut self) -> Vec<Event> {
        if self.outcome != Outcome::Ongoing {
            return Vec::new();
        }
        if !self.started {
            self.begin();
        }
        self.tick += 1;
        // Signals sent last tick become visible now.
        self.signals_active = std::mem::take(&mut self.signals_pending);

        let mut events = Vec::new();
        self.signal_heals(&mut events);
        for i in 0..self.units.len() {
            if self.units[i].is_alive() {
                self.take_turn(i, &mut events);
            }
        }

        self.outcome = self.evaluate_outcome();
        let tick = self.tick;
        self.log.extend(events.iter().cloned().map(|e| (tick, e)));
        events
    }

    /// Clerics synergy tier 3: receiving a signal heals 10% of max HP.
    fn signal_heals(&mut self, events: &mut Vec<Event>) {
        for team in [Team::Player, Team::Enemy] {
            let t = team.index();
            if self.signals_active[t] == 0 || !self.synergies[t].full(Class::Cleric) {
                continue;
            }
            for unit in self.units.iter_mut().filter(|u| u.team == team && u.is_alive()) {
                let amount = (unit.stats.max_hp / 10).min(unit.stats.max_hp - unit.hp);
                if amount > 0 {
                    unit.hp += amount;
                    events.push(Event::Healed { healer: unit.id, target: unit.id, amount });
                }
            }
        }
    }

    /// Steps until the battle ends and returns the result.
    pub fn run_to_end(&mut self) -> Outcome {
        while self.outcome == Outcome::Ongoing {
            self.step();
        }
        self.outcome
    }

    pub(crate) fn in_bounds(&self, p: Pos) -> bool {
        (0..self.width).contains(&p.x) && (0..self.height).contains(&p.y)
    }

    pub(crate) fn is_occupied(&self, p: Pos) -> bool {
        self.units.iter().any(|u| u.is_alive() && u.pos == p)
    }

    pub(crate) fn is_free(&self, p: Pos) -> bool {
        self.in_bounds(p) && !self.is_occupied(p)
    }

    fn evaluate_outcome(&self) -> Outcome {
        let alive = |team| self.units.iter().any(|u| u.is_alive() && u.team == team);
        match (alive(Team::Player), alive(Team::Enemy)) {
            (true, true) if self.tick >= self.max_ticks => Outcome::Draw,
            (true, true) => Outcome::Ongoing,
            (true, false) => Outcome::Won(Team::Player),
            (false, true) => Outcome::Won(Team::Enemy),
            (false, false) => Outcome::Draw,
        }
    }
}

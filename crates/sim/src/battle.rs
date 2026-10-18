use crate::{Instruction, Pos, Program, Team, Unit, UnitId, UnitStats};

/// Something that happened during a tick. The full event log doubles as the
/// replay: the renderer and the replay timeline only ever read events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Moved { unit: UnitId, from: Pos, to: Pos },
    Attacked { attacker: UnitId, target: UnitId, damage: u32 },
    Guarded { unit: UnitId },
    Died { unit: UnitId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ongoing,
    Won(Team),
    /// Both sides wiped out, or the tick limit was reached.
    Draw,
}

#[derive(Debug, Clone)]
pub struct Battle {
    width: i32,
    height: i32,
    max_ticks: u32,
    tick: u32,
    next_id: u32,
    /// Kept in ascending id order; dead units stay so ids remain valid.
    units: Vec<Unit>,
    log: Vec<(u32, Event)>,
    outcome: Outcome,
}

impl Battle {
    pub fn new(width: i32, height: i32, max_ticks: u32) -> Self {
        Self {
            width,
            height,
            max_ticks,
            tick: 0,
            next_id: 0,
            units: Vec::new(),
            log: Vec::new(),
            outcome: Outcome::Ongoing,
        }
    }

    /// Adds a unit before the battle starts. Returns its id.
    ///
    /// # Panics
    /// If the cell is out of bounds or already occupied.
    pub fn spawn(&mut self, team: Team, pos: Pos, stats: UnitStats, program: Program) -> UnitId {
        assert!(self.in_bounds(pos), "spawn position {pos:?} is out of bounds");
        assert!(!self.is_occupied(pos), "spawn position {pos:?} is occupied");
        let id = UnitId(self.next_id);
        self.next_id += 1;
        self.units.push(Unit {
            id,
            team,
            pos,
            stats,
            hp: stats.max_hp,
            program,
            pc: 0,
            guarding: false,
        });
        id
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

    /// Every event so far, tagged with the tick it happened on.
    pub fn log(&self) -> &[(u32, Event)] {
        &self.log
    }

    /// Advances one tick and returns the events it produced.
    /// Does nothing once the battle is over.
    pub fn step(&mut self) -> Vec<Event> {
        if self.outcome != Outcome::Ongoing {
            return Vec::new();
        }
        self.tick += 1;
        let mut events = Vec::new();

        for i in 0..self.units.len() {
            if !self.units[i].is_alive() {
                continue;
            }
            let unit = &mut self.units[i];
            unit.guarding = false;
            let instruction = unit.program.fetch(unit.pc);
            unit.pc = (unit.pc + 1) % unit.program.len().max(1);
            self.execute(i, instruction, &mut events);
        }

        self.outcome = self.evaluate_outcome();
        self.log.extend(events.iter().cloned().map(|e| (self.tick, e)));
        events
    }

    /// Steps until the battle ends and returns the result.
    pub fn run_to_end(&mut self) -> Outcome {
        while self.outcome == Outcome::Ongoing {
            self.step();
        }
        self.outcome
    }

    fn execute(&mut self, i: usize, instruction: Instruction, events: &mut Vec<Event>) {
        match instruction {
            Instruction::Wait => {}
            Instruction::Advance => self.advance(i, events),
            Instruction::Attack => self.attack(i, events),
            Instruction::Guard => {
                self.units[i].guarding = true;
                events.push(Event::Guarded { unit: self.units[i].id });
            }
        }
    }

    fn advance(&mut self, i: usize, events: &mut Vec<Event>) {
        let Some(target) = self.nearest_enemy(i) else { return };
        let from = self.units[i].pos;
        let target_pos = self.units[target].pos;
        if from.is_adjacent(target_pos) {
            return;
        }
        let step =
            from.steps_toward(target_pos).find(|&p| self.in_bounds(p) && !self.is_occupied(p));
        if let Some(to) = step {
            self.units[i].pos = to;
            events.push(Event::Moved { unit: self.units[i].id, from, to });
        }
    }

    fn attack(&mut self, i: usize, events: &mut Vec<Event>) {
        let attacker = &self.units[i];
        let target = self
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| {
                u.is_alive() && u.team != attacker.team && u.pos.is_adjacent(attacker.pos)
            })
            .min_by_key(|(_, u)| u.id)
            .map(|(j, _)| j);
        let Some(j) = target else { return };

        let base = self.units[i].stats.attack;
        let damage = if self.units[j].guarding { base.div_ceil(2) } else { base };
        let defender = &mut self.units[j];
        defender.hp = defender.hp.saturating_sub(damage);
        events.push(Event::Attacked {
            attacker: self.units[i].id,
            target: self.units[j].id,
            damage,
        });
        if !self.units[j].is_alive() {
            events.push(Event::Died { unit: self.units[j].id });
        }
    }

    /// Index of the closest living enemy; ties go to the lowest id.
    fn nearest_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.is_alive() && u.team != me.team)
            .min_by_key(|(_, u)| (u.pos.distance(me.pos), u.id))
            .map(|(j, _)| j)
    }

    fn in_bounds(&self, p: Pos) -> bool {
        (0..self.width).contains(&p.x) && (0..self.height).contains(&p.y)
    }

    fn is_occupied(&self, p: Pos) -> bool {
        self.units.iter().any(|u| u.is_alive() && u.pos == p)
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

#[cfg(test)]
mod tests {
    use super::*;
    use Instruction::*;

    const KNIGHT: UnitStats = UnitStats { max_hp: 10, attack: 3 };
    const GOBLIN: UnitStats = UnitStats { max_hp: 6, attack: 2 };

    fn skirmish() -> Battle {
        let mut b = Battle::new(8, 4, 200);
        b.spawn(Team::Player, Pos::new(0, 1), KNIGHT, Program::new([Advance, Attack]));
        b.spawn(Team::Player, Pos::new(0, 2), KNIGHT, Program::new([Advance, Guard]));
        b.spawn(Team::Enemy, Pos::new(7, 1), GOBLIN, Program::new([Advance, Attack]));
        b.spawn(Team::Enemy, Pos::new(7, 2), GOBLIN, Program::new([Advance, Attack, Attack]));
        b
    }

    #[test]
    fn identical_battles_produce_identical_logs() {
        let mut a = skirmish();
        let mut b = skirmish();
        assert_eq!(a.run_to_end(), b.run_to_end());
        assert_eq!(a.log(), b.log());
        assert!(!a.log().is_empty());
    }

    #[test]
    fn knight_beats_goblin_one_on_one() {
        let mut b = Battle::new(6, 1, 100);
        b.spawn(Team::Player, Pos::new(0, 0), KNIGHT, Program::new([Advance, Attack]));
        b.spawn(Team::Enemy, Pos::new(5, 0), GOBLIN, Program::new([Advance, Attack]));
        assert_eq!(b.run_to_end(), Outcome::Won(Team::Player));
    }

    #[test]
    fn guarding_halves_damage_rounding_up() {
        let mut b = Battle::new(2, 1, 10);
        let target = b.spawn(Team::Player, Pos::new(0, 0), KNIGHT, Program::new([Guard]));
        b.spawn(Team::Enemy, Pos::new(1, 0), GOBLIN, Program::new([Attack]));
        let events = b.step();
        assert!(events.contains(&Event::Attacked { attacker: UnitId(1), target, damage: 1 }));
    }

    #[test]
    fn units_do_not_walk_through_each_other() {
        let mut b = Battle::new(3, 1, 10);
        b.spawn(Team::Player, Pos::new(0, 0), KNIGHT, Program::new([Advance]));
        b.spawn(Team::Player, Pos::new(1, 0), KNIGHT, Program::new([Wait]));
        b.spawn(Team::Enemy, Pos::new(2, 0), GOBLIN, Program::new([Wait]));
        let events = b.step();
        assert!(events.iter().all(|e| !matches!(e, Event::Moved { .. })));
    }

    #[test]
    fn stalemate_ends_in_a_draw_at_the_tick_limit() {
        let mut b = Battle::new(4, 1, 5);
        b.spawn(Team::Player, Pos::new(0, 0), KNIGHT, Program::new([Wait]));
        b.spawn(Team::Enemy, Pos::new(3, 0), GOBLIN, Program::new([Wait]));
        assert_eq!(b.run_to_end(), Outcome::Draw);
        assert_eq!(b.tick(), 5);
    }
}

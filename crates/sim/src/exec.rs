//! The interpreter: evaluating rule cards and carrying out actions.
//!
//! This is a second `impl Battle` block. Rust lets a type's methods be split
//! across modules of the same crate, which keeps `battle.rs` focused on state.
//!
//! Every `try_*` action function follows one rule: **check first, then mutate.**
//! If an action can't be carried out it returns `false` without changing any
//! state or emitting events, so the interpreter can fall through to the next card.

use std::cmp::Reverse;

use crate::battle::{CRIT_DAMAGE_PERCENT, StatusKind};
use crate::{Action, Battle, CardRef, Condition, Event, Pos, Target};

/// Who an action may target.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Enemies,
    /// Allies, optionally including the acting unit itself.
    Allies {
        include_self: bool,
    },
}

impl Battle {
    /// Runs one unit's turn: the first card whose condition holds and whose
    /// action succeeds, otherwise the locked default rule.
    pub(crate) fn take_turn(&mut self, i: usize, events: &mut Vec<Event>) {
        self.units[i].statuses.guarding = false;
        let id = self.units[i].id;

        for n in 0..self.units[i].program.len() {
            // `Node` is `Copy`, so this copies it out instead of holding a borrow
            // of `self.units` while the action below mutates the battle.
            let node = self.units[i].program.nodes()[n];
            if !self.condition_holds(i, node.condition) {
                continue;
            }
            let start = events.len();
            if self.try_action(i, node.action, events) {
                events.insert(start, Event::CardFired { unit: id, card: CardRef::Node(n) });
                return;
            }
        }

        let start = events.len();
        if self.try_default_rule(i, events) {
            events.insert(start, Event::CardFired { unit: id, card: CardRef::Default });
        }
    }

    fn condition_holds(&self, i: usize, condition: Condition) -> bool {
        let me = &self.units[i];
        match condition {
            Condition::Always => true,
            Condition::EnemyAdjacent => self.enemies_of(i).any(|j| self.dist(i, j) == 1),
            Condition::EnemyInRange => {
                self.enemies_of(i).any(|j| self.dist(i, j) <= me.stats.range)
            }
            Condition::MyHpBelow(t) => me.hp_percent() < t.percent(),
            Condition::AllyHpBelow(t) => {
                self.allies_of(i, false).any(|j| self.units[j].hp_percent() < t.percent())
            }
            Condition::Outnumbered => {
                let enemies = self.enemies_of(i).filter(|&j| self.dist(i, j) <= 2).count();
                let allies = self.allies_of(i, true).filter(|&j| self.dist(i, j) <= 2).count();
                enemies > allies
            }
            Condition::AbilityReady(ability) => me.ability_ready(ability, self.tick),
            Condition::EveryNTicks(n) => n > 0 && self.tick.is_multiple_of(u32::from(n)),
            Condition::SignalReceived(signal) => {
                self.signals_active[me.team.index()] & signal.bit() != 0
            }
        }
    }

    fn try_action(&mut self, i: usize, action: Action, events: &mut Vec<Event>) -> bool {
        if let Some(ability) = action.ability()
            && !self.units[i].ability_ready(ability, self.tick)
        {
            return false;
        }

        let done = match action {
            Action::Advance(t) => self.try_advance(i, t, events),
            Action::Attack(t) => self.try_attack(i, t, events),
            Action::Guard => {
                self.units[i].statuses.guarding = true;
                events.push(Event::Guarded { unit: self.units[i].id });
                true
            }
            Action::Retreat => self.try_retreat(i, events),
            Action::Hold => true,
            Action::ProtectAlly(t) => self.try_protect_ally(i, t, events),
            Action::SendSignal(signal) => {
                let team = self.units[i].team.index();
                self.signals_pending[team] |= signal.bit();
                events.push(Event::SignalSent { unit: self.units[i].id, signal });
                true
            }
            Action::Taunt => self.try_taunt(i, events),
            Action::ShieldWall => self.shield_wall(i, events),
            Action::AimedShot(t) => self.try_aimed_shot(i, t, events),
            Action::Kite(t) => self.try_kite(i, t, events),
            Action::Fireball(t) => self.try_fireball(i, t, events),
            Action::Blink => self.try_blink(i, events),
            Action::Heal(t) => self.try_heal(i, t, events),
            Action::Sanctuary(t) => self.try_sanctuary(i, t, events),
            Action::Backstab(t) => self.try_backstab(i, t, events),
            Action::Shadowstep(t) => self.try_shadowstep(i, t, events),
            Action::Frenzy => self.frenzy(i, events),
            Action::Cleave => self.try_cleave(i, events),
        };

        if done && let Some(ability) = action.ability() {
            let unit = &mut self.units[i];
            unit.ready_at[ability.index()] = self.tick + ability.cooldown();
            events.push(Event::AbilityUsed { unit: unit.id, ability });
        }
        done
    }

    /// The locked final rule: attack the nearest enemy, advancing if out of range.
    fn try_default_rule(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        self.try_attack(i, Target::Nearest, events) || self.try_advance(i, Target::Nearest, events)
    }

    // ----- Shared actions -------------------------------------------------

    fn try_advance(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) <= self.units[i].stats.range {
            return false;
        }
        let dest = self.units[j].pos;
        self.try_step_toward(i, dest, events)
    }

    fn try_attack(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > self.units[i].stats.range {
            return false;
        }
        let attack = self.units[i].stats.attack;
        self.strike(i, j, attack, events);
        // Frenzy: a second strike, if the target survived the first.
        if self.units[i].statuses.frenzied_until >= self.tick && self.units[j].is_alive() {
            self.strike(i, j, attack, events);
        }
        true
    }

    fn try_retreat(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        // Retreat ignores taunts: it is about the actual nearest threat.
        let Some(threat) = self.nearest(i, Side::Enemies) else { return false };
        let threat_pos = self.units[threat].pos;
        let here = self.units[i].pos;
        let current = here.distance(threat_pos);
        let best = here
            .neighbours()
            .into_iter()
            .filter(|&p| self.is_free(p) && p.distance(threat_pos) > current)
            .max_by_key(|&p| (p.distance(threat_pos), Reverse(p)));
        match best {
            Some(to) => {
                self.move_unit(i, to, events);
                true
            }
            None => false,
        }
    }

    fn try_protect_ally(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Allies { include_self: false }) else {
            return false;
        };
        if self.dist(i, j) <= 1 {
            return false;
        }
        let dest = self.units[j].pos;
        self.try_step_toward(i, dest, events)
    }

    // ----- Knight ----------------------------------------------------------

    fn try_taunt(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let victims: Vec<usize> = self.enemies_of(i).filter(|&j| self.dist(i, j) <= 2).collect();
        if victims.is_empty() {
            return false;
        }
        let by = self.units[i].id;
        let until = self.tick + 3;
        for j in victims {
            self.units[j].statuses.taunted_by = Some((by, until));
            let unit = self.units[j].id;
            events.push(Event::StatusApplied { unit, status: StatusKind::Taunted { by }, until });
        }
        true
    }

    fn shield_wall(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let mut shielded = vec![i];
        shielded.extend(self.allies_of(i, false).filter(|&j| self.dist(i, j) == 1));
        let until = self.tick + 2;
        for j in shielded {
            self.units[j].statuses.shielded_until = until;
            let unit = self.units[j].id;
            events.push(Event::StatusApplied { unit, status: StatusKind::Shielded, until });
        }
        true
    }

    // ----- Ranger ----------------------------------------------------------

    fn try_aimed_shot(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > self.units[i].stats.range + 1 {
            return false;
        }
        let damage = self.units[i].stats.attack * 2;
        self.strike(i, j, damage, events);
        true
    }

    fn try_kite(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > self.units[i].stats.range {
            return false;
        }
        let attack = self.units[i].stats.attack;
        self.strike(i, j, attack, events);
        // The step back is a bonus; Kite still counts if there's nowhere to go.
        self.try_retreat(i, events);
        true
    }

    // ----- Mage ------------------------------------------------------------

    fn try_fireball(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > self.units[i].stats.range {
            return false;
        }
        let centre = self.units[j].pos;
        // Collect splash victims before the main hit, in case it kills the target.
        let splash: Vec<usize> = self
            .enemies_of(i)
            .filter(|&k| k != j && self.units[k].pos.is_adjacent(centre))
            .collect();
        let attack = self.units[i].stats.attack;
        self.strike(i, j, attack, events);
        for k in splash {
            self.strike(i, k, attack / 2, events);
        }
        true
    }

    fn try_blink(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let here = self.units[i].pos;
        let safety = |p: Pos| self.enemies_of(i).map(|j| p.distance(self.units[j].pos)).min();
        let Some(current) = safety(here) else { return false };

        let mut best: Option<(u32, Pos)> = None;
        for y in 0..self.height {
            for x in 0..self.width {
                let p = Pos::new(x, y);
                if p == here || here.distance(p) > 3 || !self.is_free(p) {
                    continue;
                }
                let score = safety(p).unwrap_or(0);
                // Strictly greater keeps the first cell found on ties (stable order).
                if score > current && best.is_none_or(|(s, _)| score > s) {
                    best = Some((score, p));
                }
            }
        }
        match best {
            Some((_, to)) => {
                self.teleport(i, to, events);
                true
            }
            None => false,
        }
    }

    // ----- Cleric ----------------------------------------------------------

    fn try_heal(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Allies { include_self: true }) else {
            return false;
        };
        let target_unit = &self.units[j];
        if self.dist(i, j) > self.units[i].stats.range + 1
            || target_unit.hp >= target_unit.stats.max_hp
        {
            return false;
        }
        let amount =
            (self.units[i].stats.attack * 4).min(target_unit.stats.max_hp - target_unit.hp);
        self.units[j].hp += amount;
        events.push(Event::Healed { healer: self.units[i].id, target: self.units[j].id, amount });
        true
    }

    fn try_sanctuary(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Allies { include_self: true }) else {
            return false;
        };
        if self.dist(i, j) > self.units[i].stats.range + 1 {
            return false;
        }
        let until = self.tick + 2;
        self.units[j].statuses.immune_until = until;
        let unit = self.units[j].id;
        events.push(Event::StatusApplied { unit, status: StatusKind::Immune, until });
        true
    }

    // ----- Rogue -----------------------------------------------------------

    fn try_backstab(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        let busy = self.allies_of(i, false).any(|k| self.dist(k, j) == 1);
        if self.dist(i, j) != 1 || !busy {
            return false;
        }
        let damage = self.units[i].stats.attack * 3;
        self.strike(i, j, damage, events);
        true
    }

    fn try_shadowstep(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > 4 {
            return false;
        }
        let here = self.units[i].pos;
        // "Behind" the target: the free cell next to it that is furthest from us.
        let spot = self.units[j]
            .pos
            .neighbours()
            .into_iter()
            .filter(|&p| self.is_free(p))
            .max_by_key(|&p| (p.distance(here), Reverse(p)));
        match spot {
            Some(to) => {
                self.teleport(i, to, events);
                true
            }
            None => false,
        }
    }

    // ----- Berserker -------------------------------------------------------

    fn frenzy(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let until = self.tick + 4;
        self.units[i].statuses.frenzied_until = until;
        let unit = self.units[i].id;
        events.push(Event::StatusApplied { unit, status: StatusKind::Frenzied, until });
        true
    }

    fn try_cleave(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let victims: Vec<usize> = self.enemies_of(i).filter(|&j| self.dist(i, j) == 1).collect();
        if victims.is_empty() {
            return false;
        }
        let attack = self.units[i].stats.attack;
        for j in victims {
            self.strike(i, j, attack, events);
        }
        true
    }

    // ----- Targeting -------------------------------------------------------

    /// Picks a target for unit `i`. A taunted unit always picks its taunter
    /// when choosing among enemies, whatever the card's dropdown says.
    fn select(&self, i: usize, target: Target, side: Side) -> Option<usize> {
        if side == Side::Enemies
            && let Some((by, until)) = self.units[i].statuses.taunted_by
        {
            let taunter = self.units.iter().position(|u| u.id == by && u.is_alive());
            if until >= self.tick && taunter.is_some() {
                return taunter;
            }
        }

        let candidates = || self.side_of(i, side);
        let near = |j: usize| (self.dist(i, j), self.units[j].id);
        match target {
            Target::Nearest => candidates().min_by_key(|&j| near(j)),
            Target::LowestHp => match side {
                Side::Enemies => candidates().min_by_key(|&j| (self.units[j].hp, near(j))),
                Side::Allies { .. } => {
                    candidates().min_by_key(|&j| (self.units[j].hp_percent(), near(j)))
                }
            },
            Target::HighestHp => candidates().min_by_key(|&j| (Reverse(self.units[j].hp), near(j))),
            Target::Strongest => {
                candidates().min_by_key(|&j| (Reverse(self.units[j].stats.attack), near(j)))
            }
            Target::Ranged => {
                candidates().filter(|&j| self.units[j].is_ranged()).min_by_key(|&j| near(j))
            }
        }
    }

    /// The truly nearest unit on a side, ignoring taunts.
    fn nearest(&self, i: usize, side: Side) -> Option<usize> {
        self.side_of(i, side).min_by_key(|&j| (self.dist(i, j), self.units[j].id))
    }

    fn side_of(&self, i: usize, side: Side) -> impl Iterator<Item = usize> + '_ {
        let team = self.units[i].team;
        self.units.iter().enumerate().filter_map(move |(j, u)| {
            let wanted = match side {
                Side::Enemies => u.team != team,
                Side::Allies { include_self } => u.team == team && (include_self || j != i),
            };
            (wanted && u.is_alive()).then_some(j)
        })
    }

    fn enemies_of(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        self.side_of(i, Side::Enemies)
    }

    fn allies_of(&self, i: usize, include_self: bool) -> impl Iterator<Item = usize> + '_ {
        self.side_of(i, Side::Allies { include_self })
    }

    fn dist(&self, i: usize, j: usize) -> u32 {
        self.units[i].pos.distance(self.units[j].pos)
    }

    // ----- Effects ---------------------------------------------------------

    /// Deals `base` damage from `i` to `j`, applying crits and defences.
    fn strike(&mut self, i: usize, j: usize, base: u32, events: &mut Vec<Event>) {
        // Always roll, even against an immune target, so the RNG sequence
        // doesn't depend on statuses in surprising ways.
        let crit = self.rng.chance(self.crit_chance);
        let mut damage = if crit { base * CRIT_DAMAGE_PERCENT / 100 } else { base };

        let defender = &self.units[j].statuses;
        if defender.immune_until >= self.tick {
            damage = 0;
        } else if defender.guarding || defender.shielded_until >= self.tick {
            damage = damage.div_ceil(2);
        }

        let was_alive = self.units[j].is_alive();
        self.units[j].hp = self.units[j].hp.saturating_sub(damage);
        let (attacker, target) = (self.units[i].id, self.units[j].id);
        events.push(Event::Attacked { attacker, target, damage, crit });
        if was_alive && !self.units[j].is_alive() {
            events.push(Event::Died { unit: target });
        }
    }

    fn try_step_toward(&mut self, i: usize, dest: Pos, events: &mut Vec<Event>) -> bool {
        let here = self.units[i].pos;
        match here.steps_toward(dest).find(|&p| self.is_free(p)) {
            Some(to) => {
                self.move_unit(i, to, events);
                true
            }
            None => false,
        }
    }

    fn move_unit(&mut self, i: usize, to: Pos, events: &mut Vec<Event>) {
        let from = std::mem::replace(&mut self.units[i].pos, to);
        events.push(Event::Moved { unit: self.units[i].id, from, to });
    }

    fn teleport(&mut self, i: usize, to: Pos, events: &mut Vec<Event>) {
        let from = std::mem::replace(&mut self.units[i].pos, to);
        events.push(Event::Teleported { unit: self.units[i].id, from, to });
    }
}

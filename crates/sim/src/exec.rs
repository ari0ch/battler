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
use crate::program::{Requirement, WEAK_PERCENT};
use crate::{
    Action, Battle, CardRef, Class, Condition, Event, MAX_CARDS, Node, Passive, Pos, Target,
};

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
    /// Runs one unit's turn: the first usable card whose condition holds and whose
    /// action succeeds, otherwise the locked default rule.
    pub(crate) fn take_turn(&mut self, i: usize, events: &mut Vec<Event>) {
        #[cfg(test)]
        if self.units[i].inert {
            return;
        }
        let unit = &mut self.units[i];
        unit.statuses.guarding = false;
        let id = unit.id;
        // A freeze takes the unit's first turn *after* the tick it was frozen on,
        // so it costs exactly one turn whatever the turn order.
        if unit.statuses.frozen_turns > 0 && self.tick > unit.statuses.frozen_on {
            unit.statuses.frozen_turns -= 1;
            events.push(Event::TurnSkipped { unit: id });
            return;
        }

        let slots = self.card_slots(i);
        for n in 0..slots {
            if self.run_card(i, n, events) {
                // Berserkers tier 3, Fury: the next card also runs if it can.
                let fury = self.units[i].class == Class::Berserker
                    && self.synergies[self.units[i].team.index()].full(Class::Berserker);
                if fury && n + 1 < slots {
                    self.run_card(i, n + 1, events);
                }
                return;
            }
        }

        let start = events.len();
        if self.try_default_rule(i, events) {
            events.insert(start, Event::CardFired { unit: id, card: CardRef::Default });
        }
    }

    /// Cards this unit may run: its program, capped by its level's slots (plus synergy).
    fn card_slots(&self, i: usize) -> usize {
        let u = &self.units[i];
        // Monsters run their whole fixed program.
        let max = match u.kind.character() {
            Some(c) => self.synergies[u.team.index()].card_slots(c, u.level),
            None => MAX_CARDS,
        };
        u.program.len().min(max)
    }

    /// Tries card `n`: returns true (after logging it) if it fired.
    fn run_card(&mut self, i: usize, n: usize, events: &mut Vec<Event>) -> bool {
        // `Node` is `Copy`, so this copies it out instead of holding a borrow
        // of `self.units` while the action below mutates the battle.
        let Some(&node) = self.units[i].program.nodes().get(n) else { return false };
        if !self.card_usable(i, &node) || !self.condition_holds(i, node.condition) {
            return false;
        }
        let start = events.len();
        if self.try_action(i, node.action, events) {
            let unit = self.units[i].id;
            events.insert(start, Event::CardFired { unit, card: CardRef::Node(n) });
            true
        } else {
            false
        }
    }

    /// A card is skipped if it uses a synergy block whose synergy isn't active.
    fn card_usable(&self, i: usize, node: &Node) -> bool {
        let u = &self.units[i];
        let syn = &self.synergies[u.team.index()];
        node.requirements().all(|req| match req {
            Requirement::Synergy(class, scope) => {
                u.kind.character().is_some_and(|c| syn.allows(class, scope, c))
            }
            Requirement::Signature(a) => u.kind.has_ability(a),
            Requirement::Start | Requirement::PlayerLevel(_) => true,
        })
    }

    fn condition_holds(&self, i: usize, condition: Condition) -> bool {
        let me = &self.units[i];
        let tick = self.tick;
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
            Condition::EnemyWeak => self.enemies_of(i).any(|j| {
                self.dist(i, j) <= me.stats.range && self.units[j].hp_percent() < WEAK_PERCENT
            }),
            Condition::WasHit => me.recently_hit(tick),
            Condition::AllyAttacked => self
                .allies_of(i, false)
                .any(|j| self.dist(i, j) == 1 && self.units[j].recently_hit(tick)),
            Condition::EnemiesGrouped => self.enemies_of(i).any(|j| {
                self.dist(i, j) <= me.stats.range
                    && self.enemies_of(i).any(|k| k != j && self.dist(j, k) == 1)
            }),
            Condition::EnemyIsolated => self
                .enemies_of(i)
                .any(|j| !self.enemies_of(i).any(|k| k != j && self.dist(j, k) <= 2)),
            Condition::Surrounded => {
                self.enemies_of(i).filter(|&j| self.dist(i, j) == 1).count() >= 2
            }
            Condition::SignalReceived(signal) => {
                self.signals_active[me.team.index()] & signal.bit() != 0
            }
        }
    }

    fn try_action(&mut self, i: usize, action: Action, events: &mut Vec<Event>) -> bool {
        let ability = action.ability();
        if let Some(a) = ability
            && !self.units[i].ability_ready(a, self.tick)
        {
            return false;
        }
        // Arcane Surge: every third ability crits. Decided before the action so
        // its hits can use it; undone below if the action doesn't happen.
        if ability.is_some() && self.units[i].has_passive(Passive::ArcaneSurge) {
            self.force_crit = (self.units[i].abilities_used + 1).is_multiple_of(3);
        }

        let done = match action {
            Action::Attack(t) => self.try_attack(i, t, events),
            Action::Guard => {
                self.units[i].statuses.guarding = true;
                events.push(Event::Guarded { unit: self.units[i].id });
                true
            }
            Action::Retreat => self.try_retreat(i, events),
            Action::ProtectAlly(t) => self.try_protect_ally(i, t, events),
            Action::SendSignal(signal) => {
                let team = self.units[i].team.index();
                self.signals_pending[team] |= signal.bit();
                events.push(Event::SignalSent { unit: self.units[i].id, signal });
                true
            }
            Action::Taunt => self.try_taunt(i, events),
            Action::ShieldWall => self.shield_wall(i, events),
            Action::SecondWind => self.try_second_wind(i, events),
            Action::AimedShot(t) => self.try_aimed_shot(i, t, events),
            Action::Kite(t) => self.try_kite(i, t, events),
            Action::Volley => self.try_volley(i, events),
            Action::Firebolt(t) => self.try_firebolt(i, t, events),
            Action::FrostNova => self.try_frost_nova(i, events),
            Action::Fireball(t) => self.try_fireball(i, t, events),
            Action::Blink => self.try_blink(i, events),
            Action::Heal(t) => self.try_heal(i, t, events),
            Action::Sanctuary(t) => self.try_sanctuary(i, t, events),
            Action::MassHeal => self.try_mass_heal(i, events),
            Action::Backstab(t) => self.try_backstab(i, t, events),
            Action::Shadowstep(t) => self.try_shadowstep(i, t, events),
            Action::Assassinate => self.try_assassinate(i, events),
            Action::Frenzy => self.frenzy(i, events),
            Action::Cleave => self.try_cleave(i, events),
            Action::Rampage => self.try_rampage(i, events),
        };
        self.force_crit = false;

        if done && let Some(a) = ability {
            let mage_bonus = self.units[i].class == Class::Mage
                && self.synergies[self.units[i].team.index()].full(Class::Mage);
            let cooldown =
                if mage_bonus { a.cooldown().saturating_sub(2).max(1) } else { a.cooldown() };
            let unit = &mut self.units[i];
            unit.ready_at[a.index()] = self.tick + cooldown;
            unit.abilities_used += 1;
            events.push(Event::AbilityUsed { unit: unit.id, ability: a });
        }
        done
    }

    /// The locked final rule: attack the nearest enemy, advancing if out of range.
    fn try_default_rule(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        self.try_attack(i, Target::Nearest, events) || self.try_advance(i, events)
    }

    // ----- Shared actions -------------------------------------------------

    fn try_advance(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, Target::Nearest, Side::Enemies) else { return false };
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

    // ----- Knights ---------------------------------------------------------

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

    fn try_second_wind(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let u = &self.units[i];
        if u.hp >= u.stats.max_hp {
            return false;
        }
        let amount = u.stats.max_hp * 30 / 100;
        self.heal(i, i, amount, events);
        true
    }

    // ----- Rangers ---------------------------------------------------------

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

    fn try_volley(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let range = self.units[i].stats.range;
        let victims: Vec<usize> = self
            .enemies_of(i)
            .filter(|&j| self.dist(i, j) <= range && self.targetable(j))
            .collect();
        if victims.is_empty() {
            return false;
        }
        let damage = self.units[i].stats.attack / 2;
        for j in victims {
            self.strike(i, j, damage, events);
        }
        true
    }

    // ----- Mages -----------------------------------------------------------

    fn try_firebolt(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Enemies) else { return false };
        if self.dist(i, j) > self.units[i].stats.range {
            return false;
        }
        let damage = self.units[i].stats.attack * 5 / 2;
        self.strike(i, j, damage, events);
        true
    }

    fn try_frost_nova(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let victims: Vec<usize> = self.enemies_of(i).filter(|&j| self.dist(i, j) == 1).collect();
        if victims.is_empty() {
            return false;
        }
        let damage = self.units[i].stats.attack / 2;
        for j in victims {
            self.strike(i, j, damage, events);
            if self.units[j].is_alive() {
                self.units[j].statuses.frozen_turns = 1;
                self.units[j].statuses.frozen_on = self.tick;
                let (unit, until) = (self.units[j].id, self.tick + 1);
                events.push(Event::StatusApplied { unit, status: StatusKind::Frozen, until });
            }
        }
        true
    }

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

    // ----- Clerics ---------------------------------------------------------

    fn try_heal(&mut self, i: usize, target: Target, events: &mut Vec<Event>) -> bool {
        let Some(j) = self.select(i, target, Side::Allies { include_self: true }) else {
            return false;
        };
        let t = &self.units[j];
        if self.dist(i, j) > self.units[i].stats.range + 1 || t.hp >= t.stats.max_hp {
            return false;
        }
        let amount = self.units[i].stats.attack * 4;
        self.heal(i, j, amount, events);
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
        if self.units[i].has_passive(Passive::Blessing) {
            let amount = self.units[i].stats.attack * 3;
            self.heal(i, j, amount, events);
        }
        true
    }

    fn try_mass_heal(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let hurt: Vec<usize> = self
            .allies_of(i, true)
            .filter(|&j| self.units[j].hp < self.units[j].stats.max_hp)
            .collect();
        if hurt.is_empty() {
            return false;
        }
        let amount = self.units[i].stats.attack * 2;
        for j in hurt {
            self.heal(i, j, amount, events);
        }
        true
    }

    // ----- Rogues ----------------------------------------------------------

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
        match self.far_side_of(i, j) {
            Some(to) => {
                self.teleport(i, to, events);
                true
            }
            None => false,
        }
    }

    fn try_assassinate(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        let victim = self
            .enemies_of(i)
            .filter(|&j| {
                self.targetable(j) && self.dist(i, j) <= 3 && self.units[j].hp_percent() < 25
            })
            .min_by_key(|&j| (self.units[j].hp, self.dist(i, j), self.units[j].id));
        let Some(j) = victim else { return false };
        if self.dist(i, j) > 1 {
            let Some(to) = self.far_side_of(i, j) else { return false };
            self.teleport(i, to, events);
        }
        let damage = self.units[i].stats.attack * 3;
        self.strike(i, j, damage, events);
        true
    }

    /// The free cell next to `j` that is furthest from `i` ("behind" the target).
    fn far_side_of(&self, i: usize, j: usize) -> Option<Pos> {
        let here = self.units[i].pos;
        self.units[j]
            .pos
            .neighbours()
            .into_iter()
            .filter(|&p| self.is_free(p))
            .max_by_key(|&p| (p.distance(here), Reverse(p)))
    }

    // ----- Berserkers ------------------------------------------------------

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

    fn try_rampage(&mut self, i: usize, events: &mut Vec<Event>) -> bool {
        if !self.enemies_of(i).any(|j| self.dist(i, j) == 1) {
            return false;
        }
        self.frenzy(i, events);
        self.try_cleave(i, events)
    }

    // ----- Targeting -------------------------------------------------------

    /// Can enemies pick this unit as a target? (Vanish makes it untargetable.)
    fn targetable(&self, j: usize) -> bool {
        self.units[j].statuses.untargetable_until < self.tick
    }

    /// Picks a target for unit `i`. A taunted unit always picks its taunter
    /// when choosing among enemies, whatever the card's dropdown says.
    fn select(&self, i: usize, target: Target, side: Side) -> Option<usize> {
        if side == Side::Enemies
            && let Some((by, until)) = self.units[i].statuses.taunted_by
        {
            let taunter = self.units.iter().position(|u| u.id == by && u.is_alive());
            if until >= self.tick && taunter.is_some_and(|t| self.targetable(t)) {
                return taunter;
            }
        }

        let candidates =
            || self.side_of(i, side).filter(move |&j| side != Side::Enemies || self.targetable(j));
        let near = |j: usize| (self.dist(i, j), self.units[j].id);
        match target {
            Target::Nearest => candidates().min_by_key(|&j| near(j)),
            Target::Weakest => match side {
                Side::Enemies => candidates().min_by_key(|&j| (self.units[j].hp, near(j))),
                Side::Allies { .. } => {
                    candidates().min_by_key(|&j| (self.units[j].hp_percent(), near(j)))
                }
            },
            Target::Strongest => {
                candidates().min_by_key(|&j| (Reverse(self.units[j].stats.attack), near(j)))
            }
            Target::Ranged => {
                candidates().filter(|&j| self.units[j].is_ranged()).min_by_key(|&j| near(j))
            }
            Target::Marked => {
                let mark = self.ranger_mark[self.units[i].team.index()]?;
                candidates().find(|&j| self.units[j].id == mark)
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

    /// Deals `base` damage from `i` to `j`, applying crits, passives and defences.
    fn strike(&mut self, i: usize, j: usize, base: u32, events: &mut Vec<Event>) {
        // Always roll, even against an immune target, so the RNG sequence
        // doesn't depend on statuses in surprising ways.
        let rolled = self.rng.chance(self.units[i].crit_chance);
        let crit = rolled || self.force_crit;
        let mut damage = if crit { base * CRIT_DAMAGE_PERCENT / 100 } else { base };

        // Attacker and target bonuses (percentages multiply).
        let pct = |d: u32, p: u32| d * p / 100;
        let target = &self.units[j];
        if self.units[i].has_passive(Passive::Opportunist) && target.hp_percent() < 50 {
            damage = pct(damage, 130);
        }
        if target.statuses.hunters_marked {
            damage = pct(damage, 115);
        }
        if target.statuses.frozen_turns > 0 {
            // Only Mirelle freezes, so a frozen target is always Chilled.
            damage = pct(damage, 120);
        }

        // Defences.
        let s = &target.statuses;
        if s.immune_until >= self.tick {
            damage = 0;
        } else {
            if s.guarding || s.shielded_until >= self.tick {
                damage = damage.div_ceil(2);
            }
            if target.has_passive(Passive::LastStand) && target.hp_percent() < 25 {
                damage = damage.div_ceil(2);
            }
            if target.has_passive(Passive::Bulwark) {
                let allies = self.allies_of(j, false).filter(|&k| self.dist(j, k) == 1).count();
                damage = pct(damage, 100 - 10 * allies.min(3) as u32);
            }
        }

        let (attacker, victim) = (self.units[i].id, self.units[j].id);
        let was_alive = self.units[j].is_alive();
        let lethal = damage >= self.units[j].hp;
        let undying =
            lethal && self.units[j].has_passive(Passive::Undying) && !self.units[j].cheated_death;
        let dealt = if undying { self.units[j].hp - 1 } else { damage.min(self.units[j].hp) };

        let t = &mut self.units[j];
        t.hp -= dealt;
        if damage > 0 {
            t.statuses.damaged_on = self.tick;
        }
        events.push(Event::Attacked { attacker, target: victim, damage, crit });

        if undying {
            self.units[j].cheated_death = true;
            events.push(Event::CheatedDeath { unit: victim });
        } else if was_alive && !self.units[j].is_alive() {
            events.push(Event::Died { unit: victim });
            self.try_resurrect(j, events);
        }

        // After-hit effects for the attacker.
        if self.units[i].class == Class::Ranger {
            self.ranger_mark[self.units[i].team.index()] = Some(victim);
        }
        if self.units[i].has_passive(Passive::HuntersMark)
            && self.units[j].is_alive()
            && !self.units[j].statuses.hunters_marked
        {
            self.units[j].statuses.hunters_marked = true;
            events.push(Event::StatusApplied {
                unit: victim,
                status: StatusKind::Marked,
                until: u32::MAX,
            });
        }
        if self.units[i].has_passive(Passive::Bloodlust) && self.units[i].hp_percent() < 50 {
            let amount = dealt / 5;
            if amount > 0 {
                self.heal(i, i, amount, events);
            }
        }
    }

    /// Resurrection: the first of a team's units to fall comes back at 30%,
    /// if that team has a living Seraphine (who isn't the one falling).
    fn try_resurrect(&mut self, j: usize, events: &mut Vec<Event>) {
        let team = self.units[j].team;
        if self.resurrection_used[team.index()] {
            return;
        }
        let has_seraphine = self.units.iter().enumerate().any(|(k, u)| {
            k != j && u.team == team && u.is_alive() && u.has_passive(Passive::Resurrection)
        });
        if !has_seraphine {
            return;
        }
        self.resurrection_used[team.index()] = true;
        let u = &mut self.units[j];
        u.hp = (u.stats.max_hp * 30 / 100).max(1);
        u.statuses.frozen_turns = 0;
        events.push(Event::Revived { unit: u.id, hp: u.hp });
    }

    /// Restores HP (capped at max) and logs it.
    fn heal(&mut self, healer: usize, target: usize, amount: u32, events: &mut Vec<Event>) {
        let t = &mut self.units[target];
        let amount = amount.min(t.stats.max_hp - t.hp);
        if amount == 0 {
            return;
        }
        t.hp += amount;
        let (healer, target) = (self.units[healer].id, self.units[target].id);
        events.push(Event::Healed { healer, target, amount });
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

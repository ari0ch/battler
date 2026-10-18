//! Card wording, the block palette per class, and "click to cycle" settings.
//! Text is plain ASCII because Bevy's built-in font only covers ASCII.

use battler_sim::{
    Ability, Action, Class, Condition, Level, Node, Program, Signal, Target, Threshold,
};

use crate::state::DraftCard;
use crate::ui::Block;

pub fn target_label(t: Target) -> &'static str {
    match t {
        Target::Nearest => "nearest",
        Target::LowestHp => "lowest HP",
        Target::HighestHp => "highest HP",
        Target::Strongest => "strongest",
        Target::Ranged => "ranged",
    }
}

fn signal_label(s: Signal) -> &'static str {
    match s {
        Signal::Red => "red",
        Signal::Blue => "blue",
    }
}

pub fn ability_label(a: Ability) -> &'static str {
    match a {
        Ability::Taunt => "Taunt",
        Ability::ShieldWall => "Shield Wall",
        Ability::AimedShot => "Aimed Shot",
        Ability::Kite => "Kite",
        Ability::Fireball => "Fireball",
        Ability::Blink => "Blink",
        Ability::Heal => "Heal",
        Ability::Sanctuary => "Sanctuary",
        Ability::Backstab => "Backstab",
        Ability::Shadowstep => "Shadowstep",
        Ability::Frenzy => "Frenzy",
        Ability::Cleave => "Cleave",
    }
}

pub fn cond_label(c: Condition) -> String {
    match c {
        Condition::Always => "always".into(),
        Condition::EnemyAdjacent => "enemy adjacent".into(),
        Condition::EnemyInRange => "enemy in range".into(),
        Condition::MyHpBelow(t) => format!("my HP < {}%", t.percent()),
        Condition::AllyHpBelow(t) => format!("ally HP < {}%", t.percent()),
        Condition::Outnumbered => "outnumbered".into(),
        Condition::AbilityReady(a) => format!("{} ready", ability_label(a)),
        Condition::EveryNTicks(n) => format!("every {n} ticks"),
        Condition::SignalReceived(s) => format!("signal: {}", signal_label(s)),
    }
}

/// The action's name, without its target.
pub fn action_name(a: Action) -> String {
    if let Some(ability) = a.ability() {
        return ability_label(ability).into();
    }
    match a {
        Action::Advance(_) => "Advance".into(),
        Action::Attack(_) => "Attack".into(),
        Action::Guard => "Guard".into(),
        Action::Retreat => "Retreat".into(),
        Action::Hold => "Hold".into(),
        Action::ProtectAlly(_) => "Protect Ally".into(),
        Action::SendSignal(s) => format!("Signal {}", signal_label(s)),
        _ => "?".into(),
    }
}

pub fn action_label(a: Action) -> String {
    match a.target() {
        Some(t) => format!("{} > {}", action_name(a), target_label(t)),
        None => action_name(a),
    }
}

pub fn card_label(n: &Node) -> String {
    format!("{} -> {}", cond_label(n.condition), action_label(n.action))
}

pub fn block_label(b: Block) -> String {
    match b {
        Block::Cond(c) => cond_label(c),
        Block::Act(a) => action_label(a),
    }
}

/// Short description shown for signature abilities.
pub fn ability_help(a: Ability) -> &'static str {
    match a {
        Ability::Taunt => "Enemies within 2 cells must target this unit for 3 ticks.",
        Ability::ShieldWall => "This unit and adjacent allies take half damage for 2 ticks.",
        Ability::AimedShot => "Double damage, range +1.",
        Ability::Kite => "Attack, then step away from the nearest enemy.",
        Ability::Fireball => "Hits the target, plus half damage to enemies next to it.",
        Ability::Blink => "Teleport up to 3 cells to the spot furthest from enemies.",
        Ability::Heal => "Restore 4x attack in HP to an ally in range +1.",
        Ability::Sanctuary => "An ally in range +1 is immune to damage for 2 ticks.",
        Ability::Backstab => "Triple damage on an adjacent enemy next to one of your allies.",
        Ability::Shadowstep => "Teleport next to an enemy within 4 cells, on its far side.",
        Ability::Frenzy => "For 4 ticks, basic attacks strike twice.",
        Ability::Cleave => "Hit every adjacent enemy.",
    }
}

/// How far an ability reaches, for highlighting cells on the board (Manhattan radius).
pub fn ability_reach(a: Ability, class: Class) -> u32 {
    let range = class.base_stats().range;
    match a {
        Ability::Taunt => 2,
        Ability::ShieldWall | Ability::Backstab | Ability::Cleave => 1,
        Ability::AimedShot | Ability::Heal | Ability::Sanctuary => range + 1,
        Ability::Blink => 3,
        Ability::Shadowstep => 4,
        Ability::Kite | Ability::Fireball => range,
        Ability::Frenzy => 0,
    }
}

pub fn shared_conditions(class: Class) -> Vec<Condition> {
    vec![
        Condition::Always,
        Condition::EnemyAdjacent,
        Condition::EnemyInRange,
        Condition::MyHpBelow(Threshold::P50),
        Condition::AllyHpBelow(Threshold::P50),
        Condition::Outnumbered,
        Condition::AbilityReady(class.abilities()[0]),
        Condition::EveryNTicks(3),
        Condition::SignalReceived(Signal::Red),
    ]
}

pub fn shared_actions() -> Vec<Action> {
    vec![
        Action::Advance(Target::Nearest),
        Action::Attack(Target::Nearest),
        Action::Guard,
        Action::Retreat,
        Action::Hold,
        Action::ProtectAlly(Target::LowestHp),
        Action::SendSignal(Signal::Red),
    ]
}

/// The class's two signature actions with sensible default targets.
pub fn signature_actions(class: Class) -> Vec<Action> {
    use Action::*;
    match class {
        Class::Knight => vec![Taunt, ShieldWall],
        Class::Ranger => vec![AimedShot(Target::LowestHp), Kite(Target::Nearest)],
        Class::Mage => vec![Fireball(Target::Nearest), Blink],
        Class::Cleric => vec![Heal(Target::LowestHp), Sanctuary(Target::LowestHp)],
        Class::Rogue => vec![Backstab(Target::Nearest), Shadowstep(Target::Ranged)],
        Class::Berserker => vec![Frenzy, Cleave],
    }
}

fn next_threshold(t: Threshold) -> Threshold {
    match t {
        Threshold::P25 => Threshold::P50,
        Threshold::P50 => Threshold::P75,
        Threshold::P75 => Threshold::P25,
    }
}

fn other_signal(s: Signal) -> Signal {
    match s {
        Signal::Red => Signal::Blue,
        Signal::Blue => Signal::Red,
    }
}

/// The next setting for a condition when its slot is clicked.
pub fn cycle_condition(c: Condition, class: Class) -> Condition {
    match c {
        Condition::MyHpBelow(t) => Condition::MyHpBelow(next_threshold(t)),
        Condition::AllyHpBelow(t) => Condition::AllyHpBelow(next_threshold(t)),
        Condition::EveryNTicks(n) => Condition::EveryNTicks(if n >= 5 { 2 } else { n + 1 }),
        Condition::SignalReceived(s) => Condition::SignalReceived(other_signal(s)),
        Condition::AbilityReady(a) => {
            let [first, second] = class.abilities();
            Condition::AbilityReady(if a == first { second } else { first })
        }
        other => other,
    }
}

fn with_target(a: Action, t: Target) -> Action {
    use Action::*;
    match a {
        Advance(_) => Advance(t),
        Attack(_) => Attack(t),
        ProtectAlly(_) => ProtectAlly(t),
        AimedShot(_) => AimedShot(t),
        Kite(_) => Kite(t),
        Fireball(_) => Fireball(t),
        Heal(_) => Heal(t),
        Sanctuary(_) => Sanctuary(t),
        Backstab(_) => Backstab(t),
        Shadowstep(_) => Shadowstep(t),
        other => other,
    }
}

/// The next valid setting for an action when its slot is clicked: the next
/// target this class may use, or the other signal colour.
pub fn cycle_action(a: Action, class: Class) -> Action {
    if let Action::SendSignal(s) = a {
        return Action::SendSignal(other_signal(s));
    }
    let Some(current) = a.target() else { return a };
    let order =
        [Target::Nearest, Target::LowestHp, Target::HighestHp, Target::Strongest, Target::Ranged];
    let start = order.iter().position(|&t| t == current).unwrap_or(0);
    for step in 1..order.len() {
        let candidate = with_target(a, order[(start + step) % order.len()]);
        let probe = Program::new([Node::new(Condition::Always, candidate)]);
        if probe.validate(class, Level::One).is_ok() {
            return candidate;
        }
    }
    a
}

/// Converts the editor's draft into a program. Cards without an action are left
/// out; a card without a condition means "always".
pub fn draft_to_program(draft: &[DraftCard]) -> Program {
    Program::new(
        draft
            .iter()
            .filter_map(|c| c.act.map(|a| Node::new(c.cond.unwrap_or(Condition::Always), a)))
            .collect::<Vec<_>>(),
    )
}

pub fn program_to_draft(program: &Program) -> Vec<DraftCard> {
    program
        .nodes()
        .iter()
        .map(|n| DraftCard { cond: Some(n.condition), act: Some(n.action) })
        .collect()
}

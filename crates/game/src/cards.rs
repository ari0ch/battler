//! Words and help text for blocks, characters and synergies, the palette for a
//! character, "click to cycle" settings, and tooltip builders.
//! Text is plain ASCII because Bevy's built-in font only covers ASCII.

use battler_sim::run::{Lobby, Opponent, RoundKind};
use battler_sim::{
    Ability, Action, Character, Class, Condition, Level, Node, Program, Rarity, Requirement, Scope,
    Signal, Synergies, Target, Threshold, UnitKind, Unlocks, palette_actions, palette_conditions,
};
use bevy::prelude::Color;

use crate::state::DraftCard;
use crate::ui::{Block, Tooltip, color};

/// Two letters for unit tiles until there's art.
/// Two letters for a unit tile. Takes a character or a monster.
pub fn initials(kind: impl Into<UnitKind>) -> String {
    kind.into().short_name().chars().take(2).collect()
}

/// Who a round is against, for headers and the result screen.
pub fn opponent_label(lobby: &Lobby, opponent: Opponent) -> String {
    match opponent {
        Opponent::Monsters(wave) => wave.name().to_string(),
        Opponent::Player(id) => lobby.player(id).name().to_string(),
        Opponent::Ghost(id) => format!("{}'s ghost", lobby.player(id).name()),
    }
}

/// What a round will be, before it's fought.
pub fn round_label(kind: RoundKind) -> String {
    match kind {
        RoundKind::Pve(wave) if wave.is_boss() => format!("Boss: {}", wave.name()),
        RoundKind::Pve(wave) => format!("Monsters: {}", wave.name()),
        RoundKind::Versus => "Versus".to_string(),
    }
}

pub fn rarity_color(r: Rarity) -> Color {
    match r {
        Rarity::Common => color::COMMON,
        Rarity::Rare => color::RARE,
        Rarity::Epic => color::EPIC,
    }
}

pub fn target_label(t: Target) -> &'static str {
    match t {
        Target::Nearest => "nearest",
        Target::Weakest => "weakest",
        Target::Strongest => "strongest",
        Target::Ranged => "ranged",
        Target::Marked => "marked",
    }
}

fn signal_label(s: Signal) -> &'static str {
    match s {
        Signal::Red => "red",
        Signal::Blue => "blue",
    }
}

pub fn ability_label(a: Ability) -> &'static str {
    use Ability::*;
    match a {
        Taunt => "Taunt",
        ShieldWall => "Shield Wall",
        SecondWind => "Second Wind",
        AimedShot => "Aimed Shot",
        Kite => "Kite",
        Volley => "Volley",
        Firebolt => "Firebolt",
        FrostNova => "Frost Nova",
        Fireball => "Fireball",
        Blink => "Blink",
        Heal => "Heal",
        Sanctuary => "Sanctuary",
        MassHeal => "Mass Heal",
        Backstab => "Backstab",
        Shadowstep => "Shadowstep",
        Assassinate => "Assassinate",
        Frenzy => "Frenzy",
        Cleave => "Cleave",
        Rampage => "Rampage",
    }
}

pub fn ability_help(a: Ability) -> &'static str {
    use Ability::*;
    match a {
        Taunt => "Enemies within 2 cells must target this unit for 3 ticks.",
        ShieldWall => "This unit and adjacent allies take half damage for 2 ticks.",
        SecondWind => "Heal yourself for 30% of max HP. Falls through at full HP.",
        AimedShot => "Double damage, range +1.",
        Kite => "Attack, then step away from the nearest enemy.",
        Volley => "Half damage to every enemy in range.",
        Firebolt => "A 2.5x damage bolt at a target in range.",
        FrostNova => "Half damage to adjacent enemies; each loses its next turn.",
        Fireball => "Hits the target, plus half damage to enemies next to it.",
        Blink => "Teleport up to 3 cells to the spot furthest from enemies.",
        Heal => "Restore 4x attack in HP to an ally in range +1. Falls through if unhurt.",
        Sanctuary => "An ally in range +1 is immune to damage for 2 ticks.",
        MassHeal => "Restore 2x attack in HP to every ally. Falls through if nobody is hurt.",
        Backstab => "Triple damage on an adjacent enemy that is next to one of your allies.",
        Shadowstep => "Teleport next to an enemy within 4 cells, on its far side.",
        Assassinate => "Teleport to an enemy below 25% HP within 3 cells and strike for 3x.",
        Frenzy => "For 4 ticks, basic attacks strike twice.",
        Cleave => "Hit every adjacent enemy.",
        Rampage => "Frenzy and Cleave at once. Needs an adjacent enemy.",
    }
}

pub fn cond_label(c: Condition) -> String {
    use Condition::*;
    match c {
        Always => "always".into(),
        EnemyAdjacent => "enemy adjacent".into(),
        EnemyInRange => "enemy in range".into(),
        MyHpBelow(t) => format!("my HP < {}%", t.percent()),
        AllyHpBelow(t) => format!("ally HP < {}%", t.percent()),
        Outnumbered => "outnumbered".into(),
        EnemyWeak => "enemy weak".into(),
        WasHit => "I was hit".into(),
        AllyAttacked => "ally attacked".into(),
        EnemiesGrouped => "enemies grouped".into(),
        EnemyIsolated => "enemy isolated".into(),
        Surrounded => "surrounded".into(),
        SignalReceived(s) => format!("signal: {}", signal_label(s)),
    }
}

pub fn cond_help(c: Condition) -> String {
    use Condition::*;
    match c {
        Always => "Always true. An empty condition slot means always.".into(),
        EnemyAdjacent => "An enemy is next to this unit.".into(),
        EnemyInRange => "An enemy is within this unit's attack range.".into(),
        MyHpBelow(t) => format!("This unit is below {}% HP. Click to change.", t.percent()),
        AllyHpBelow(t) => format!("Another ally is below {}% HP. Click to change.", t.percent()),
        Outnumbered => "More enemies than allies within 2 cells.".into(),
        EnemyWeak => format!("An enemy in range is below {}% HP.", battler_sim::WEAK_PERCENT),
        WasHit => "This unit took damage since its last turn.".into(),
        AllyAttacked => "An adjacent ally took damage since its last turn.".into(),
        EnemiesGrouped => "An enemy in range has another enemy next to it.".into(),
        EnemyIsolated => "Some enemy has none of its allies within 2 cells.".into(),
        Surrounded => "Two or more enemies are adjacent.".into(),
        SignalReceived(_) => {
            "A teammate sent this signal last tick. Click to change colour.".into()
        }
    }
}

/// The action's name, without its target.
pub fn action_name(a: Action) -> String {
    if let Some(ability) = a.ability() {
        return ability_label(ability).into();
    }
    match a {
        Action::Attack(_) => "Attack".into(),
        Action::Guard => "Guard".into(),
        Action::Retreat => "Retreat".into(),
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

pub fn action_help(a: Action) -> String {
    let base = if let Some(ability) = a.ability() {
        format!("{} Cooldown {} ticks.", ability_help(ability), ability.cooldown())
    } else {
        match a {
            Action::Attack(_) => "Basic attack on the target, if in range.".into(),
            Action::Guard => "Take half damage until this unit's next turn.".into(),
            Action::Retreat => "Step away from the nearest enemy.".into(),
            Action::ProtectAlly(_) => "Move next to the targeted ally.".into(),
            Action::SendSignal(_) => {
                "Teammates see this signal next tick. Click to change colour.".into()
            }
            _ => String::new(),
        }
    };
    match a.target() {
        Some(t) => format!("{base} Target: {} (click to change).", target_label(t)),
        None => base,
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

pub fn block_requirement(b: Block) -> Requirement {
    match b {
        Block::Cond(c) => c.requirement(),
        Block::Act(a) => a.requirement(),
    }
}

fn requirement_sentence(req: Requirement) -> Option<String> {
    match req {
        Requirement::PlayerLevel(n) => Some(format!("Unlocks at player level {n}.")),
        Requirement::Synergy(class, Scope::Team) => {
            Some(format!("Unlocked by 2 {} on the board, for the whole team.", class.plural()))
        }
        Requirement::Synergy(class, Scope::ClassOnly) => Some(format!(
            "Unlocked by 2 {} on the board. Only {} can use it.",
            class.plural(),
            class.plural()
        )),
        Requirement::Start | Requirement::Signature(_) => None,
    }
}

/// What a class synergy does at a tier (2 or 3).
pub fn synergy_effect(class: Class, tier: u8) -> &'static str {
    match (class, tier) {
        (Class::Knight, 2) => "Unlocks 'ally attacked' (Knights only).",
        (Class::Knight, _) => "+1 card slot for Knights.",
        (Class::Ranger, 2) => "Unlocks the 'marked' target for the team.",
        (Class::Ranger, _) => "+1 range for Rangers.",
        (Class::Mage, 2) => "Unlocks 'enemies grouped' (Mages only).",
        (Class::Mage, _) => "Mage signature cooldowns -2.",
        (Class::Cleric, 2) => "Unlocks Signal blocks for the team.",
        (Class::Cleric, _) => "Receiving a signal heals 10% max HP.",
        (Class::Rogue, 2) => "Unlocks 'enemy isolated' (Rogues only).",
        (Class::Rogue, _) => "Rogue crit chance doubles.",
        (Class::Berserker, 2) => "Unlocks 'surrounded' (Berserkers only).",
        (Class::Berserker, _) => "Fury: after a card fires, the next card also runs.",
    }
}

/// How far an ability reaches, for highlighting cells on the board (Manhattan radius).
pub fn ability_reach(a: Ability, character: Character) -> u32 {
    let range = character.base_stats().range;
    use Ability::*;
    match a {
        Taunt => 2,
        ShieldWall | Backstab | Cleave | FrostNova | Rampage => 1,
        AimedShot | Heal | Sanctuary => range + 1,
        Blink | Assassinate => 3,
        Shadowstep => 4,
        Kite | Fireball | Firebolt | Volley => range,
        SecondWind | Frenzy | MassHeal => 0,
    }
}

/// The palette for a character: shared conditions, then its signatures and the
/// shared actions. Locked blocks are included (shown greyed out).
pub fn palette(character: Character) -> (Vec<Block>, Vec<Block>) {
    let conds = palette_conditions().into_iter().map(Block::Cond).collect();
    let acts = character
        .abilities()
        .iter()
        .map(|&a| Block::Act(Action::for_ability(a)))
        .chain(palette_actions().into_iter().map(Block::Act))
        .collect();
    (conds, acts)
}

/// Labels of the blocks and targets that unlock at a player level.
pub fn unlocks_at_level(level: u32) -> Vec<String> {
    let at = |req: Requirement| req == Requirement::PlayerLevel(level);
    let mut out: Vec<String> =
        palette_conditions().into_iter().filter(|c| at(c.requirement())).map(cond_label).collect();
    out.extend(palette_actions().into_iter().filter(|a| at(a.requirement())).map(action_name));
    out.extend(
        Target::ALL
            .into_iter()
            .filter(|t| at(t.requirement()))
            .map(|t| format!("target: {}", target_label(t))),
    );
    out
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
pub fn cycle_condition(c: Condition) -> Condition {
    match c {
        Condition::MyHpBelow(t) => Condition::MyHpBelow(next_threshold(t)),
        Condition::AllyHpBelow(t) => Condition::AllyHpBelow(next_threshold(t)),
        Condition::SignalReceived(s) => Condition::SignalReceived(other_signal(s)),
        other => other,
    }
}

/// The next target this character may use, or the other signal colour.
pub fn cycle_action(a: Action, unlocks: &Unlocks, character: Character) -> Action {
    if let Action::SendSignal(s) = a {
        return Action::SendSignal(other_signal(s));
    }
    let Some(current) = a.target() else { return a };
    let all = Target::ALL;
    let start = all.iter().position(|&t| t == current).unwrap_or(0);
    for step in 1..all.len() {
        let t = all[(start + step) % all.len()];
        let usable = unlocks.allows(t.requirement(), character)
            && !(a.targets_allies() && t == Target::Marked);
        if usable {
            return a.with_target(t);
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
        .map(|n| DraftCard {
            cond: (n.condition != Condition::Always).then_some(n.condition),
            act: Some(n.action),
        })
        .collect()
}

// ----- Tooltips -----------------------------------------------------------

pub fn unit_tooltip(c: Character, level: Option<Level>, note: Option<String>) -> Tooltip {
    let level = level.unwrap_or(Level::One);
    let stats = level.scale(c.base_stats());
    let mut lines = vec![
        (
            format!(
                "{} {} - {} gold - level {}",
                c.rarity().name(),
                c.class().name(),
                c.rarity().cost(),
                level.number()
            ),
            rarity_color(c.rarity()),
        ),
        (
            format!("HP {}   Attack {}   Range {}", stats.max_hp, stats.attack, stats.range),
            color::TEXT,
        ),
    ];
    for &a in c.abilities() {
        lines.push((
            format!("{} (signature, cd {})", ability_label(a), a.cooldown()),
            color::SIG_TEXT,
        ));
        lines.push((ability_help(a).to_string(), color::MUTED));
    }
    if let Some(p) = c.passive() {
        lines.push((format!("{} (passive)", p.name()), color::GOLD_LIGHT));
        lines.push((p.description().to_string(), color::MUTED));
    }
    if let Some(note) = note {
        lines.push((note, color::GOOD));
    }
    Tooltip { title: c.name().to_string(), accent: rarity_color(c.rarity()), lines }
}

pub fn block_tooltip(b: Block, unlocks: &Unlocks, character: Character) -> Tooltip {
    let (help, accent) = match b {
        Block::Cond(c) => (cond_help(c), color::COND_LINE),
        Block::Act(a) if a.ability().is_some() => (action_help(a), color::SIG_LINE),
        Block::Act(a) => (action_help(a), color::ACT_LINE),
    };
    let mut lines = vec![(help, color::TEXT)];
    let req = block_requirement(b);
    if let Some(s) = requirement_sentence(req) {
        let ink = if unlocks.allows(req, character) { color::MUTED } else { color::BAD };
        lines.push((s, ink));
    }
    if let Block::Act(a) = b
        && let Some(t) = a.target()
        && let Some(s) = requirement_sentence(t.requirement())
    {
        lines.push((format!("Target '{}': {s}", target_label(t)), color::MUTED));
    }
    lines.push(("Drag or click into a card.".to_string(), color::FAINT));
    Tooltip { title: block_label(b), accent, lines }
}

pub fn synergy_tooltip(class: Class, synergies: &Synergies) -> Tooltip {
    let count = synergies.count(class);
    let tier_line = |tier: u8| {
        let ink = if count >= tier { color::GOLD_LIGHT } else { color::FAINT };
        (format!("{tier}: {}", synergy_effect(class, tier)), ink)
    };
    let names: Vec<&str> =
        Character::ALL.iter().filter(|c| c.class() == class).map(|c| c.short_name()).collect();
    Tooltip {
        title: format!("{} - {count} on the board", class.plural()),
        accent: if count >= 2 { color::GOLD } else { color::LINE_STRONG },
        lines: vec![
            tier_line(2),
            tier_line(3),
            (names.join(", "), color::MUTED),
            ("Only units on the board count, once per character.".to_string(), color::FAINT),
        ],
    }
}

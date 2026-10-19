//! The timed planning phase, all on one screen: header, synergy tracker, the
//! next opponent, the player's half of the board and the bench, the selected
//! unit's program (see `editor.rs`), the lobby standings, and the shop.
//!
//! The board is drawn rotated from sim coordinates, as in battle: lanes (`y`)
//! run across the screen and the front row (`x = SIDE_DEPTH - 1`) is at the
//! top, facing the enemy. Only the player's half is shown while planning.

use battler_sim::run::economy::{BENCH_SIZE, BOARD_LANES, REROLL_COST, SIDE_DEPTH, XP_COST};
use battler_sim::run::{Lobby, Location, OwnedUnit, Player, RoundKind};
use battler_sim::{Class, Level, Pos, Wave};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::cards;
use crate::editor::{self, HoveredAbility};
use crate::result::ordinal;
use crate::sprites::{Sprites, sprite_node};
use crate::state::{GameRun, LobbyExt, ME, Planning, Rev, RunPhase};
use crate::ui::{
    self, ActionQueue, Clickable, DragSource, DropTarget, PRIMARY, SECONDARY, UiAction, color,
    column, row, spawn_button, text,
};

pub struct PlanningPlugin;

impl Plugin for PlanningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RunPhase::Planning), enter).add_systems(
            Update,
            (hotkeys, rebuild, tick_timers, update_cells)
                .chain()
                .run_if(in_state(RunPhase::Planning)),
        );
    }
}

pub const CELL: f32 = 72.0;
pub const GAP: f32 = 4.0;
/// The board's width on screen: one cell per lane.
const BOARD_PX: f32 = CELL * BOARD_LANES as f32 + GAP * (BOARD_LANES as f32 - 1.0);
/// Board tiles fit a 2x sprite (64 px) inside a 2 px border.
const TILE: f32 = 68.0;
const BENCH_TILE: f32 = 50.0;

/// Seconds allowed for planning this round.
pub fn planning_seconds(run: &Player) -> f32 {
    let units = run.board().len() + run.bench().iter().flatten().count();
    if units >= 5 { 75.0 } else { 60.0 }
}

fn enter(run: Res<GameRun>, mut planning: ResMut<Planning>, mut rev: ResMut<Rev>) {
    planning.time_left = planning_seconds(run.0.me());
    planning.toast = None;
    if planning.selected.is_some_and(|id| run.0.me().locate(id).is_none()) {
        planning.selected = None;
    }
    editor::load_draft(run.0.me(), &mut planning);
    rev.bump();
}

#[derive(Component)]
struct PlanningRoot;
#[derive(Component)]
struct TimerText;
#[derive(Component)]
struct TimerBar;
#[derive(Component)]
struct ToastText;
#[derive(Component)]
struct NoticeBox;

/// A player-side board cell, coloured each frame (hover, reach highlight).
#[derive(Component)]
struct CellView(Pos);

/// D rerolls, F buys XP, E sells the unit under the pointer.
fn hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<ActionQueue>,
    units: Query<(&DragSource, &Hovered)>,
) {
    if keys.just_pressed(KeyCode::KeyD) {
        queue.0.push(UiAction::Reroll);
    }
    if keys.just_pressed(KeyCode::KeyF) {
        queue.0.push(UiAction::BuyXp);
    }
    if keys.just_pressed(KeyCode::KeyE) {
        let hovered = units.iter().find_map(|(src, h)| match src {
            DragSource::Unit(loc) if h.get() => Some(*loc),
            _ => None,
        });
        if let Some(loc) = hovered {
            queue.0.push(UiAction::SellAt(loc));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn tick_timers(
    time: Res<Time<Real>>,
    run: Res<GameRun>,
    mut planning: ResMut<Planning>,
    mut queue: ResMut<ActionQueue>,
    mut timer_text: Query<&mut Text, (With<TimerText>, Without<ToastText>)>,
    mut timer_bar: Query<&mut Node, (With<TimerBar>, Without<NoticeBox>)>,
    mut toast_text: Query<&mut Text, (With<ToastText>, Without<TimerText>)>,
    mut notice: Query<&mut Node, (With<NoticeBox>, Without<TimerBar>)>,
) {
    let dt = time.delta_secs();
    let before = planning.time_left;
    planning.time_left = (planning.time_left - dt).max(0.0);
    if before > 0.0 && planning.time_left <= 0.0 {
        queue.0.push(UiAction::Ready);
    }
    let secs = planning.time_left.ceil() as u32;
    for mut t in &mut timer_text {
        **t = format!("{}:{:02}", secs / 60, secs % 60);
    }
    let total = planning_seconds(run.0.me());
    for mut n in &mut timer_bar {
        n.width = percent(100.0 * planning.time_left / total);
    }

    if let Some((_, left)) = planning.toast.as_mut() {
        *left -= dt;
    }
    if planning.toast.as_ref().is_some_and(|(_, left)| *left <= 0.0) {
        planning.toast = None;
    }
    let message = planning.toast.as_ref().map(|(m, _)| m.clone()).unwrap_or_default();
    for mut t in &mut toast_text {
        if **t != message {
            **t = message.clone();
        }
    }

    if let Some((_, left)) = planning.notice.as_mut() {
        *left -= dt;
    }
    if planning.notice.as_ref().is_some_and(|(_, left)| *left <= 0.0) {
        planning.notice = None;
        for mut n in &mut notice {
            n.display = Display::None;
        }
    }
}

/// Colours player-side cells: hover feedback, plus the reach of a hovered
/// signature block around the selected unit.
fn update_cells(
    run: Res<GameRun>,
    planning: Res<Planning>,
    hovered_ability: Res<HoveredAbility>,
    mut cells: Query<(&CellView, &Hovered, &mut BackgroundColor, &mut BorderColor)>,
) {
    let selected = editor::selected_unit(run.0.me(), &planning);
    let reach = match (hovered_ability.0, selected) {
        (Some(a), Some((Location::Board(origin), unit))) => {
            Some((origin, cards::ability_reach(a, unit.character)))
        }
        _ => None,
    };
    for (cell, hovered, mut bg, mut border) in &mut cells {
        let in_reach = reach.is_some_and(|(o, r)| {
            let d = o.distance(cell.0);
            d > 0 && d <= r
        });
        let (fill, line) = if in_reach {
            (color::REACH, color::REACH_LINE)
        } else if hovered.get() {
            (color::PANEL_2, color::LINE_STRONG)
        } else {
            (color::CELL, color::LINE)
        };
        bg.0 = fill;
        border.set_all(line);
    }
}

fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    run: Res<GameRun>,
    planning: Res<Planning>,
    sprites: Res<Sprites>,
    roots: Query<Entity, With<PlanningRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let lobby = &run.0;
    let run = lobby.me();
    let stage = ui::spawn_stage(
        &mut commands,
        (PlanningRoot, DespawnOnExit(RunPhase::Planning)),
        color::BG,
    );
    commands.entity(stage).with_children(|stage| {
        header(stage, lobby, &planning);
        stage
            .spawn(Node {
                flex_grow: 1.0,
                min_height: px(0),
                padding: UiRect::new(px(20), px(20), px(14), px(12)),
                align_items: AlignItems::Stretch,
                ..row(14.0)
            })
            .with_children(|body| {
                synergy_column(body, run);
                body.spawn(Node { width: px(BOARD_PX), flex_shrink: 0.0, ..column(8.0) })
                    .with_children(|mid| {
                        next_opponent(mid, lobby);
                        board_section(mid, run, &planning, &sprites);
                        bench_section(mid, run, &planning, &sprites);
                    });
                editor::spawn_program_panel(body, run, &planning, &sprites);
                standings(body, lobby);
            });
        shop(stage, run, &sprites);
        notice(stage, &planning);
    });
}

fn header(stage: &mut ChildSpawnerCommands, lobby: &Lobby, planning: &Planning) {
    let run = lobby.me();
    stage
        .spawn((
            Node {
                height: px(60),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(20)),
                border: UiRect::bottom(px(1)),
                ..row(24.0)
            },
            BackgroundColor(color::BAR),
            BorderColor::all(color::LINE),
        ))
        .with_children(|h| {
            h.spawn(Node { width: px(110), ..column(2.0) }).with_children(|c| {
                c.spawn(text(format!("Round {}", lobby.round()), 19.0, color::TEXT));
                let (kind, ink) = match lobby.round_kind() {
                    RoundKind::Pve(w) if w.is_boss() => ("Boss round", color::BAD),
                    RoundKind::Pve(_) => ("Monster round", color::GOOD),
                    RoundKind::Versus => ("Versus", color::MUTED),
                };
                c.spawn(text(kind, 12.0, ink));
            });
            h.spawn(Node { width: px(360), ..row(12.0) }).with_children(|lv| {
                lv.spawn(Node { width: px(170), ..column(5.0) }).with_children(|c| {
                    let xp = match run.xp_needed() {
                        Some(n) => format!("Level {} - XP {} / {n}", run.level(), run.xp()),
                        None => format!("Level {} - max", run.level()),
                    };
                    c.spawn(text(xp, 13.0, color::TEXT));
                    let fill =
                        run.xp_needed().map_or(100.0, |n| 100.0 * run.xp() as f32 / n as f32);
                    c.spawn((
                        Node {
                            height: px(6),
                            border_radius: BorderRadius::all(px(3)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(color::LINE),
                    ))
                    .with_child((
                        Node { width: percent(fill), height: px(6), ..default() },
                        BackgroundColor(color::XP),
                    ));
                });
                spawn_button(
                    lv,
                    &format!("Buy XP {XP_COST} g (F)"),
                    UiAction::BuyXp,
                    SECONDARY,
                    px(170),
                    px(40),
                );
            });
            h.spawn(Node { flex_grow: 1.0, align_items: AlignItems::Center, ..column(5.0) })
                .with_children(|c| {
                    let secs = planning.time_left.ceil() as u32;
                    c.spawn((
                        text(format!("{}:{:02}", secs / 60, secs % 60), 22.0, color::GOLD_LIGHT),
                        TimerText,
                    ));
                    c.spawn((
                        Node {
                            width: px(260),
                            height: px(5),
                            border_radius: BorderRadius::all(px(3)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(color::LINE),
                    ))
                    .with_child((
                        Node { width: percent(100), height: px(5), ..default() },
                        BackgroundColor(color::GOLD),
                        TimerBar,
                    ));
                });
            h.spawn(Node { width: px(400), justify_content: JustifyContent::FlexEnd, ..row(20.0) })
                .with_children(|r| {
                    stat(r, "Health", format!("{} / 100", run.hp()), color::TEXT);
                    stat(
                        r,
                        &format!("Gold - +{} interest", run.interest_preview()),
                        format!("{} g", run.gold()),
                        color::GOLD_LIGHT,
                    );
                    let (label, c) = match run.streak() {
                        s if s >= 2 => (format!("{s} wins"), color::GOOD),
                        s if s <= -2 => (format!("{} losses", -s), color::BAD),
                        _ => ("none".to_string(), color::MUTED),
                    };
                    stat(r, "Streak", label, c);
                });
        });
    // The toast line floats just under the header.
    stage
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(66),
                left: px(0),
                right: px(0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(50),
        ))
        .with_child((text("", 15.0, color::BAD), ToastText));
}

fn stat(parent: &mut ChildSpawnerCommands, label: &str, value: String, value_color: Color) {
    parent.spawn(Node { align_items: AlignItems::FlexEnd, ..column(3.0) }).with_children(|c| {
        c.spawn(text(label, 12.0, color::MUTED));
        c.spawn(text(value, 17.0, value_color));
    });
}

/// The level-up notice, under the header.
fn notice(stage: &mut ChildSpawnerCommands, planning: &Planning) {
    let Some((level, _)) = planning.notice else { return };
    let unlocked = cards::unlocks_at_level(level);
    stage
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                // Over the program panel, clear of the board.
                top: px(70),
                right: px(140),
                width: px(420),
                padding: UiRect::axes(px(18), px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(12)),
                ..row(18.0)
            },
            BackgroundColor(color::PANEL),
            BorderColor::all(color::GOLD),
            GlobalZIndex(60),
            NoticeBox,
            Button,
            Hovered::default(),
            Clickable(UiAction::DismissNotice),
        ))
        .with_children(|n| {
            n.spawn(Node { width: px(110), ..column(2.0) }).with_children(|c| {
                c.spawn(text(format!("Level {level}"), 24.0, color::GOLD_LIGHT));
                c.spawn(text(format!("Board size {level}"), 12.0, color::MUTED));
            });
            n.spawn(Node { flex_grow: 1.0, ..column(4.0) }).with_children(|c| {
                if unlocked.is_empty() {
                    c.spawn(text(
                        "Better shop odds: rarer characters appear more often.",
                        13.0,
                        color::TEXT,
                    ));
                } else {
                    c.spawn(text("New blocks unlocked", 12.0, color::MUTED));
                    c.spawn(text(unlocked.join(", "), 14.0, color::TEXT));
                }
                c.spawn(text("Click to dismiss", 11.0, color::FAINT));
            });
        });
}

/// The synergy tracker: each class on the board (and bench-only ones, faded).
fn synergy_column(parent: &mut ChildSpawnerCommands, run: &Player) {
    let synergies = run.synergies();
    let on_bench: Vec<Class> = run.bench().iter().flatten().map(|u| u.class()).collect();
    let mut classes: Vec<Class> = Class::ALL
        .into_iter()
        .filter(|&c| synergies.count(c) > 0 || on_bench.contains(&c))
        .collect();
    classes.sort_by_key(|&c| std::cmp::Reverse(synergies.count(c)));

    parent.spawn(Node { width: px(170), flex_shrink: 0.0, ..column(7.0) }).with_children(|col| {
        col.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(0.0) })
            .with_children(|h| {
                h.spawn(text("Synergies", 12.0, color::MUTED));
                h.spawn(text(
                    format!("{} active", synergies.active_count()),
                    11.0,
                    color::GOLD_LIGHT,
                ));
            });
        if classes.is_empty() {
            col.spawn(text("Field 2 units of a class to unlock its synergy.", 11.0, color::FAINT));
        }
        for class in classes {
            let count = synergies.count(class);
            let active = count >= 2;
            let bench_only = count == 0;
            let (fill, line, name_ink) = if active {
                (color::ACTIVE_FILL, color::ACTIVE_LINE, color::GOLD_LIGHT)
            } else if bench_only {
                (Color::NONE, color::LINE, color::FAINT)
            } else {
                (color::PANEL, color::LINE, color::TEXT)
            };
            col.spawn((
                Node {
                    padding: UiRect::axes(px(9), px(7)),
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(8)),
                    ..column(4.0)
                },
                BackgroundColor(fill),
                BorderColor::all(line),
                Hovered::default(),
                cards::synergy_tooltip(class, &synergies),
            ))
            .with_children(|r| {
                r.spawn(row(8.0)).with_children(|top| {
                    let (badge_fill, badge_ink) = if active {
                        (color::GOLD, color::ON_GOLD)
                    } else {
                        (color::PANEL_2, name_ink)
                    };
                    top.spawn((
                        Node {
                            width: px(22),
                            height: px(22),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(px(6)),
                            ..default()
                        },
                        BackgroundColor(badge_fill),
                        Pickable::IGNORE,
                    ))
                    .with_child(text(count.to_string(), 12.0, badge_ink));
                    top.spawn((Node { flex_grow: 1.0, ..default() }, Pickable::IGNORE))
                        .with_child(text(class.plural(), 13.0, name_ink));
                    top.spawn(text("2 > 3", 11.0, color::FAINT));
                });
                let effect = match count {
                    3.. => Some((cards::synergy_effect(class, 3).to_string(), color::GOLD_LIGHT)),
                    2 => Some((
                        format!(
                            "{} Next: {}",
                            cards::synergy_effect(class, 2),
                            cards::synergy_effect(class, 3)
                        ),
                        color::GOLD_LIGHT,
                    )),
                    1 => {
                        Some((format!("1 more: {}", cards::synergy_effect(class, 2)), color::MUTED))
                    }
                    _ => None,
                };
                if let Some((t, ink)) = effect {
                    r.spawn(text(t, 11.0, ink));
                }
            });
        }
        col.spawn((Node { margin: UiRect::top(Val::Auto), ..default() }, Pickable::IGNORE))
            .with_child(text("Only units on the board count.", 11.0, color::FAINT));
    });
}

/// Small gold squares showing a unit's level.
pub fn spawn_pips(parent: &mut ChildSpawnerCommands, level: Level) {
    parent.spawn((row(3.0), Pickable::IGNORE)).with_children(|p| {
        for _ in 0..level.number() {
            p.spawn((
                Node {
                    width: px(5),
                    height: px(5),
                    border_radius: BorderRadius::all(px(1)),
                    ..default()
                },
                BackgroundColor(color::GOLD),
                Pickable::IGNORE,
            ));
        }
    });
}

/// Dots showing how many of a unit's card slots are programmed.
fn spawn_card_dots(parent: &mut ChildSpawnerCommands, filled: usize, max: usize) {
    parent.spawn((row(2.0), Pickable::IGNORE)).with_children(|p| {
        for i in 0..max {
            let c = if i < filled {
                color::GOOD
            } else if filled == 0 {
                color::BAD
            } else {
                color::LINE_STRONG
            };
            p.spawn((
                Node {
                    width: px(4),
                    height: px(4),
                    border_radius: BorderRadius::all(px(2)),
                    ..default()
                },
                BackgroundColor(c),
                Pickable::IGNORE,
            ));
        }
    });
}

/// A draggable, clickable unit tile with a tooltip: the sprite, with level
/// pips and programmed-card dots along the bottom edge.
fn unit_tile(
    parent: &mut ChildSpawnerCommands,
    run: &Player,
    unit: &OwnedUnit,
    loc: Location,
    size: f32,
    selected: bool,
    sprites: &Sprites,
) {
    let c = unit.character;
    let max = run.synergies().card_slots(c, unit.level);
    // 2x on the board; 1x where tiles are small (the bench).
    let scale = if size >= TILE { 2.0 } else { 1.0 };
    parent
        .spawn((
            Button,
            Hovered::default(),
            Clickable(UiAction::SelectUnit(loc)),
            DragSource::Unit(loc),
            // Let drops "see through" the tile being dragged to the target below.
            Pickable { should_block_lower: false, is_hoverable: true },
            GlobalZIndex::default(),
            Node {
                width: px(size),
                height: px(size),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(9)),
                ..default()
            },
            BackgroundColor(color::BLUE_FILL),
            BorderColor::all(cards::rarity_color(c.rarity())),
            Outline {
                width: px(if selected { 2.0 } else { 0.0 }),
                offset: px(2),
                color: color::GOLD,
            },
            cards::unit_tooltip(
                c,
                Some(unit.level),
                Some(format!("Sells for {} gold (E)", unit.sell_value())),
            ),
        ))
        .with_children(|t| {
            t.spawn(sprite_node(sprites.character(c), scale, false));
            t.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(3),
                    right: px(3),
                    bottom: px(2),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|foot| {
                spawn_pips(foot, unit.level);
                spawn_card_dots(foot, unit.program.len(), max);
            });
        });
}

/// A slim strip above the board: who or what the next battle is against.
fn next_opponent(parent: &mut ChildSpawnerCommands, lobby: &Lobby) {
    parent
        .spawn((
            Node {
                padding: UiRect::axes(px(12), px(7)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..column(3.0)
            },
            BackgroundColor(color::CELL_ENEMY),
            BorderColor::all(color::LINE),
        ))
        .with_children(|strip| match lobby.round_kind() {
            RoundKind::Pve(wave) => wave_preview(strip, wave),
            RoundKind::Versus => {
                strip.spawn(text("Next: versus", 14.0, color::RED_TEXT));
                strip.spawn(text(
                    format!(
                        "{} players left. Your opponent is drawn when the battle starts.",
                        lobby.alive_count()
                    ),
                    11.0,
                    color::MUTED,
                ));
            }
        });
}

fn board_section(
    parent: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    parent.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(0.0) }).with_children(
        |r| {
            r.spawn(text(
                format!("Your side - {} / {} placed", run.board().len(), run.board_limit()),
                12.0,
                color::MUTED,
            ));
            r.spawn(text("Front row (top) faces the enemy", 11.0, color::FAINT));
        },
    );
    parent.spawn(column(GAP)).with_children(|board| player_half(board, run, planning, sprites));
}

/// What the monsters of a PvE round are, before the battle.
fn wave_preview(parent: &mut ChildSpawnerCommands, wave: Wave) {
    let ink = if wave.is_boss() { color::BAD } else { color::GOOD };
    let mut names: Vec<(&str, usize)> = Vec::new();
    for (_, spec) in wave.units() {
        let name = spec.kind.name();
        match names.iter_mut().find(|(n, _)| *n == name) {
            Some((_, count)) => *count += 1,
            None => names.push((name, 1)),
        }
    }
    let units: Vec<String> = names
        .into_iter()
        .map(|(name, count)| if count > 1 { format!("{count} x {name}") } else { name.to_string() })
        .collect();
    let kind = if wave.is_boss() { "Next: boss" } else { "Next: monsters" };
    parent.spawn(row(10.0)).with_children(|r| {
        r.spawn(text(format!("{kind} - {}", wave.name()), 14.0, ink));
        r.spawn(text(units.join(", "), 12.0, color::RED_TEXT));
    });
    let stakes = if wave.is_boss() {
        "Win: gold. Lose: extra damage, rising each round."
    } else {
        "Win: a little gold. Lose: 2 HP."
    };
    parent.spawn(text(format!("{} {stakes}", wave.hint()), 11.0, color::MUTED));
}

/// The lobby standings: a narrow sidebar, every player's health, best first.
fn standings(parent: &mut ChildSpawnerCommands, lobby: &Lobby) {
    let mut players: Vec<&Player> = lobby.players().iter().collect();
    players.sort_by_key(|p| {
        (std::cmp::Reverse(p.is_alive()), std::cmp::Reverse(p.hp()), p.placement())
    });
    parent
        .spawn(ui::panel(Node {
            width: px(104),
            flex_shrink: 0.0,
            padding: UiRect::axes(px(8), px(10)),
            ..column(9.0)
        }))
        .with_children(|col| {
            col.spawn(text("Players", 12.0, color::MUTED));
            for p in players {
                let me = p.id() == ME;
                let alive = p.is_alive();
                let name_ink = if me {
                    color::GOLD_LIGHT
                } else if alive {
                    color::TEXT
                } else {
                    color::FAINT
                };
                col.spawn(column(3.0)).with_children(|entry| {
                    entry
                        .spawn(Node {
                            justify_content: JustifyContent::SpaceBetween,
                            overflow: Overflow::clip(),
                            ..row(4.0)
                        })
                        .with_children(|r| {
                            r.spawn(text(p.name(), 11.0, name_ink));
                            let right = if alive {
                                p.hp().to_string()
                            } else {
                                p.placement().map(ordinal).unwrap_or_default()
                            };
                            r.spawn(text(
                                right,
                                11.0,
                                if alive { color::TEXT } else { color::FAINT },
                            ));
                        });
                    if alive {
                        entry
                            .spawn((
                                Node {
                                    height: px(4),
                                    border_radius: BorderRadius::all(px(2)),
                                    overflow: Overflow::clip(),
                                    ..default()
                                },
                                BackgroundColor(color::LINE),
                            ))
                            .with_child((
                                Node { width: percent(p.hp() as f32), height: px(4), ..default() },
                                BackgroundColor(if me { color::GOLD } else { color::HP_FILL }),
                            ));
                    }
                });
            }
        });
}

/// The player's half of the board: one screen row per depth row, front row
/// first, and one cell per lane across.
fn player_half(
    board: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    for x in (0..SIDE_DEPTH).rev() {
        board.spawn(row(GAP)).with_children(|cells| {
            for y in 0..BOARD_LANES {
                let pos = Pos::new(x, y);
                let cell_node = Node {
                    width: px(CELL),
                    height: px(CELL),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(px(1)),
                    border_radius: BorderRadius::all(px(7)),
                    ..default()
                };
                cells
                    .spawn((
                        Button,
                        Hovered::default(),
                        Clickable(UiAction::ClickCell(pos)),
                        DropTarget::Cell(pos),
                        CellView(pos),
                        cell_node,
                        BackgroundColor(color::CELL),
                        BorderColor::all(color::LINE),
                    ))
                    .with_children(|cell| {
                        let loc = Location::Board(pos);
                        if let Some(unit) = run.unit_at(loc) {
                            let selected = planning.selected == Some(unit.id);
                            unit_tile(cell, run, unit, loc, TILE, selected, sprites);
                        }
                    });
            }
        });
    }
}

fn bench_section(
    parent: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    let used = run.bench().iter().flatten().count();
    parent
        .spawn(Node {
            justify_content: JustifyContent::SpaceBetween,
            margin: UiRect::top(px(4)),
            ..row(0.0)
        })
        .with_children(|r| {
            r.spawn(text(format!("Bench - {used} / {BENCH_SIZE}"), 12.0, color::MUTED));
            r.spawn(text(
                "Ring = rarity - dots = cards programmed - hover for stats",
                11.0,
                color::FAINT,
            ));
        });
    parent.spawn(row(5.0)).with_children(|slots| {
        let slot_w = (BOARD_PX - 5.0 * (BENCH_SIZE as f32 - 1.0)) / BENCH_SIZE as f32;
        for i in 0..BENCH_SIZE {
            slots
                .spawn(ui::button(
                    UiAction::ClickBench(i),
                    ui::ButtonStyle {
                        fill: color::PANEL,
                        hover: color::PANEL_2,
                        border: color::LINE_STRONG,
                    },
                    Node { width: px(slot_w), height: px(58), ..default() },
                ))
                .insert(DropTarget::Bench(i))
                .with_children(|slot| {
                    let loc = Location::Bench(i);
                    if let Some(unit) = run.unit_at(loc) {
                        let selected = planning.selected == Some(unit.id);
                        unit_tile(slot, run, unit, loc, BENCH_TILE, selected, sprites);
                    }
                });
        }
    });
}

fn shop(stage: &mut ChildSpawnerCommands, run: &Player, sprites: &Sprites) {
    stage
        .spawn((
            Node {
                height: px(112),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(20)),
                border: UiRect::top(px(1)),
                ..row(12.0)
            },
            BackgroundColor(color::BAR),
            BorderColor::all(color::LINE),
            // Dropping a unit onto the shop sells it.
            DropTarget::Sell,
        ))
        .with_children(|footer| {
            footer.spawn(Node { flex_grow: 1.0, ..row(10.0) }).with_children(|offers| {
                for (i, offer) in run.shop().iter().enumerate() {
                    let node = Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        height: px(88),
                        padding: UiRect::axes(px(11), px(9)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        overflow: Overflow::clip(),
                        ..default()
                    };
                    let Some(c) = *offer else {
                        offers
                            .spawn((
                                Node {
                                    border: UiRect::all(px(1)),
                                    border_radius: BorderRadius::all(px(10)),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..node
                                },
                                BorderColor::all(color::LINE),
                            ))
                            .with_child(text("Sold", 13.0, color::FAINT));
                        continue;
                    };
                    let merges = run.buy_would_merge(c);
                    let raises = run.would_raise_synergy(c);
                    let owned = run.copies_owned(c);
                    let (note, note_ink) = if merges {
                        (format!("Owned {owned} - merges"), color::GOOD)
                    } else if raises {
                        let next = run.synergies().count(c.class()) + 1;
                        (format!("{next} {} - activates", c.class().plural()), color::GOLD_LIGHT)
                    } else if owned > 0 {
                        (format!("Owned {owned}"), color::MUTED)
                    } else {
                        ("New".to_string(), color::FAINT)
                    };
                    let border = if merges { color::GOOD } else { color::LINE_STRONG };
                    offers
                        .spawn(ui::button(
                            UiAction::Buy(i),
                            ui::ButtonStyle { fill: color::PANEL, hover: color::PANEL_2, border },
                            node,
                        ))
                        .insert(cards::unit_tooltip(c, None, None))
                        .with_children(|card| {
                            card.spawn((row(9.0), Pickable::IGNORE)).with_children(|r| {
                                r.spawn((
                                    Node {
                                        width: px(40),
                                        height: px(40),
                                        flex_shrink: 0.0,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::all(px(2)),
                                        border_radius: BorderRadius::all(px(7)),
                                        ..default()
                                    },
                                    BackgroundColor(color::BLUE_FILL),
                                    BorderColor::all(cards::rarity_color(c.rarity())),
                                    Pickable::IGNORE,
                                ))
                                .with_child(sprite_node(
                                    sprites.character(c),
                                    1.0,
                                    false,
                                ));
                                r.spawn((column(1.0), Pickable::IGNORE)).with_children(|col| {
                                    col.spawn(text(c.name(), 13.0, color::TEXT));
                                    col.spawn(text(
                                        format!("{} {}", c.rarity().name(), c.class().name()),
                                        11.0,
                                        cards::rarity_color(c.rarity()),
                                    ));
                                });
                            });
                            card.spawn((
                                Node {
                                    width: percent(100),
                                    justify_content: JustifyContent::SpaceBetween,
                                    ..row(0.0)
                                },
                                Pickable::IGNORE,
                            ))
                            .with_children(|r| {
                                r.spawn(text(note, 11.0, note_ink));
                                r.spawn(text(
                                    format!("{} g", c.rarity().cost()),
                                    11.0,
                                    color::GOLD_LIGHT,
                                ));
                            });
                        });
                }
            });
            footer.spawn(Node { width: px(140), ..column(8.0) }).with_children(|col| {
                spawn_button(
                    col,
                    &format!("Reroll {REROLL_COST} g (D)"),
                    UiAction::Reroll,
                    SECONDARY,
                    percent(100),
                    px(38),
                );
                spawn_button(col, "Ready", UiAction::Ready, PRIMARY, percent(100), px(44));
            });
        });
}

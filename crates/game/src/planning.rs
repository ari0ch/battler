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
use crate::kit;
use crate::result::ordinal;
use crate::sprites::{Sprites, sprite_node};
use crate::state::{GameRun, LobbyExt, ME, Planning, Rev, RunPhase};
use crate::ui::{
    self, ActionQueue, Clickable, DragSource, DropTarget, PRIMARY, SECONDARY, UiAction, color,
    column, heading, row, text,
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

/// Dark red and green that read on parchment.
const INK_RED: Color = Color::srgb(0.557, 0.165, 0.165);
const INK_GREEN: Color = Color::srgb(0.180, 0.420, 0.180);
const INK_GOLD: Color = Color::srgb(0.541, 0.353, 0.0);

/// One board cell, grid line included.
pub const CELL: f32 = 76.0;
/// Grid line width, drawn on each cell's right and bottom edge.
const LINE: f32 = 2.0;
/// The board's width on screen: one cell per lane.
const BOARD_PX: f32 = CELL * BOARD_LANES as f32;
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
            (color::REACH, color::GRID)
        } else if hovered.get() {
            (color::CELL_HOVER, color::GRID)
        } else {
            (color::CELL, color::GRID)
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
                padding: UiRect::new(px(14), px(14), px(12), px(10)),
                align_items: AlignItems::Stretch,
                ..row(12.0)
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
                height: px(64),
                flex_shrink: 0.0,
                padding: UiRect::new(px(16), px(16), px(0), px(6)),
                ..row(20.0)
            },
            kit::tiled(kit::img::HEADER_WOOD, true, false),
        ))
        .with_children(|h| {
            // The round on a red ribbon.
            h.spawn((
                Node {
                    width: px(184),
                    height: px(46),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..column(0.0)
                },
                kit::sliced(kit::img::BANNER, 6.0),
            ))
            .with_children(|c| {
                c.spawn(heading(format!("ROUND {}", lobby.round()), 18.0, color::TEXT));
                let kind = match lobby.round_kind() {
                    RoundKind::Pve(w) if w.is_boss() => "Boss round",
                    RoundKind::Pve(_) => "Monster round",
                    RoundKind::Versus => "Versus",
                };
                c.spawn(text(kind, 11.0, color::TEXT));
            });
            h.spawn(row(10.0)).with_children(|lv| {
                lv.spawn(kit::icon(kit::img::ICON_CROWN, 32.0));
                lv.spawn(Node { width: px(150), ..column(4.0) }).with_children(|c| {
                    let xp = match run.xp_needed() {
                        Some(n) => format!("Level {} - XP {} / {n}", run.level(), run.xp()),
                        None => format!("Level {} - max", run.level()),
                    };
                    c.spawn(heading(xp, 13.0, color::TEXT));
                    let fill =
                        run.xp_needed().map_or(100.0, |n| 100.0 * run.xp() as f32 / n as f32);
                    kit::spawn_bar(c, kit::img::FILL_XP, fill, percent(100), 16.0, ());
                });
                lv.spawn(ui::button(
                    UiAction::BuyXp,
                    SECONDARY,
                    Node { height: px(44), padding: UiRect::horizontal(px(12)), ..row(6.0) },
                ))
                .with_children(|b| {
                    b.spawn(kit::icon(kit::img::ICON_STAR, 20.0));
                    b.spawn(heading("Buy XP", 15.0, color::TEXT));
                    b.spawn(kit::icon(kit::img::ICON_COIN, 18.0));
                    b.spawn(heading(XP_COST.to_string(), 15.0, color::TEXT));
                    b.spawn(text("F", 12.0, color::MUTED));
                });
            });
            h.spawn(Node { flex_grow: 1.0, align_items: AlignItems::Center, ..column(4.0) })
                .with_children(|c| {
                    let secs = planning.time_left.ceil() as u32;
                    c.spawn(row(8.0)).with_children(|r| {
                        r.spawn(kit::icon(kit::img::ICON_HOURGLASS, 26.0));
                        r.spawn((
                            ui::outlined(
                                format!("{}:{:02}", secs / 60, secs % 60),
                                24.0,
                                color::GOLD_LIGHT,
                            ),
                            TimerText,
                        ));
                    });
                    kit::spawn_bar(c, kit::img::FILL_TIME, 100.0, px(240), 14.0, TimerBar);
                });
            h.spawn(row(18.0)).with_children(|r| {
                stat(r, kit::img::ICON_HEART, run.hp().to_string(), "health", color::TEXT);
                stat(
                    r,
                    kit::img::ICON_COIN,
                    run.gold().to_string(),
                    &format!("+{} interest", run.interest_preview()),
                    color::GOLD_LIGHT,
                );
                let (label, c) = match run.streak() {
                    s if s >= 2 => (format!("{s} wins"), color::GOOD),
                    s if s <= -2 => (format!("{} losses", -s), color::BAD),
                    _ => ("no streak".to_string(), color::MUTED),
                };
                r.spawn(Node { align_items: AlignItems::FlexEnd, ..column(2.0) }).with_children(
                    |c2| {
                        c2.spawn(heading(label, 15.0, c));
                        c2.spawn(text("streak", 11.0, color::MUTED));
                    },
                );
            });
        });
    // The toast line floats just under the header.
    stage
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(70),
                left: px(0),
                right: px(0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(50),
        ))
        .with_child((ui::outlined("", 16.0, color::BAD), ToastText));
}

/// An icon with a big number and a small caption under it.
fn stat(
    parent: &mut ChildSpawnerCommands,
    icon: Handle<Image>,
    value: String,
    caption: &str,
    value_color: Color,
) {
    parent.spawn(row(6.0)).with_children(|r| {
        r.spawn(kit::icon(icon, 28.0));
        r.spawn(column(0.0)).with_children(|c| {
            c.spawn(ui::outlined(value, 20.0, value_color));
            c.spawn(text(caption, 11.0, color::MUTED));
        });
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
                padding: UiRect::axes(px(22), px(18)),
                ..row(18.0)
            },
            kit::sliced(kit::img::PANEL_WOOD, 10.0),
            GlobalZIndex(60),
            NoticeBox,
            Button,
            Hovered::default(),
            Clickable(UiAction::DismissNotice),
        ))
        .with_children(|n| {
            n.spawn(Node { width: px(110), ..column(2.0) }).with_children(|c| {
                c.spawn(ui::outlined(format!("Level {level}"), 24.0, color::GOLD_LIGHT));
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

    parent
        .spawn(ui::panel(Node { width: px(184), flex_shrink: 0.0, ..column(6.0) }))
        .with_children(|col| {
            col.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(0.0) })
                .with_children(|h| {
                    h.spawn(heading("Synergies", 15.0, color::GOLD_LIGHT));
                    h.spawn(text(
                        format!("{} active", synergies.active_count()),
                        11.0,
                        color::GOLD_LIGHT,
                    ));
                });
            if classes.is_empty() {
                col.spawn(text(
                    "Field 2 units of a class to unlock its synergy.",
                    11.0,
                    color::FAINT,
                ));
            }
            for class in classes {
                let count = synergies.count(class);
                let active = count >= 2;
                let bench_only = count == 0;
                let (fill, line, name_ink) = if active {
                    (color::ACTIVE_FILL, color::ACTIVE_LINE, color::GOLD_LIGHT)
                } else if bench_only {
                    (Color::NONE, Color::NONE, color::FAINT)
                } else {
                    (Color::srgba(0.0, 0.0, 0.0, 0.2), Color::NONE, color::TEXT)
                };
                col.spawn((
                    Node {
                        padding: UiRect::axes(px(7), px(6)),
                        border: UiRect::all(px(2)),
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
                            (color::LINE_STRONG, name_ink)
                        };
                        top.spawn((
                            Node {
                                width: px(22),
                                height: px(22),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(badge_fill),
                            Pickable::IGNORE,
                        ))
                        .with_child(heading(
                            count.to_string(),
                            13.0,
                            badge_ink,
                        ));
                        top.spawn(kit::icon(kit::class_icon(class), 24.0));
                        top.spawn((Node { flex_grow: 1.0, ..default() }, Pickable::IGNORE))
                            .with_child(heading(class.plural(), 14.0, name_ink));
                    });
                    let effect = match count {
                        3.. => {
                            Some((cards::synergy_effect(class, 3).to_string(), color::GOLD_LIGHT))
                        }
                        2 => Some((
                            format!(
                                "{} Next: {}",
                                cards::synergy_effect(class, 2),
                                cards::synergy_effect(class, 3)
                            ),
                            color::GOLD_LIGHT,
                        )),
                        1 => Some((
                            format!("1 more: {}", cards::synergy_effect(class, 2)),
                            color::MUTED,
                        )),
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
                ..default()
            },
            // The frame's colour shows rarity: bronze, silver with blue gems, gold.
            kit::sliced(kit::rarity_frame(c.rarity()), 6.0),
            Outline {
                width: px(if selected { 3.0 } else { 0.0 }),
                offset: px(1),
                color: color::GOLD_LIGHT,
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
            Node { padding: UiRect::axes(px(18), px(12)), ..column(3.0) },
            kit::sliced(kit::img::PANEL_PARCHMENT, 10.0),
        ))
        .with_children(|strip| match lobby.round_kind() {
            RoundKind::Pve(wave) => wave_preview(strip, wave),
            RoundKind::Versus => {
                strip.spawn(row(8.0)).with_children(|r| {
                    r.spawn(kit::icon(kit::img::ICON_SWORD, 22.0));
                    r.spawn(heading("Next: versus", 15.0, INK_RED));
                });
                strip.spawn(text(
                    format!(
                        "{} players left. Your opponent is drawn when the battle starts.",
                        lobby.alive_count()
                    ),
                    12.0,
                    color::INK_MUTED,
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
    // One grass field under the whole half, framed in wood; the cells only draw
    // grid lines over it.
    parent
        .spawn((
            Node {
                width: px(BOARD_PX),
                border: UiRect::new(px(LINE), px(0), px(LINE), px(0)),
                ..column(0.0)
            },
            kit::tiled(kit::img::FIELD_GRASS, true, true),
            BorderColor::all(color::GRID),
            Outline { width: px(4), offset: px(0), color: color::LINE_STRONG },
        ))
        .with_children(|board| player_half(board, run, planning, sprites));
}

/// What the monsters of a PvE round are, before the battle.
fn wave_preview(parent: &mut ChildSpawnerCommands, wave: Wave) {
    let ink = if wave.is_boss() { INK_RED } else { INK_GREEN };
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
    let icon = if wave.is_boss() { kit::img::ICON_SKULL } else { kit::img::ICON_SWORD };
    parent.spawn(row(8.0)).with_children(|r| {
        r.spawn(kit::icon(icon, 22.0));
        r.spawn(heading(format!("{kind} - {}", wave.name()), 15.0, ink));
        r.spawn(text(units.join(", "), 12.0, color::INK_MUTED));
    });
    let stakes = if wave.is_boss() {
        "Win: gold. Lose: extra damage, rising each round."
    } else {
        "Win: a little gold. Lose: 2 HP."
    };
    parent.spawn(text(format!("{} {stakes}", wave.hint()), 12.0, color::INK_MUTED));
}

/// The lobby standings: a narrow sidebar, every player's health, best first.
fn standings(parent: &mut ChildSpawnerCommands, lobby: &Lobby) {
    let mut players: Vec<&Player> = lobby.players().iter().collect();
    players.sort_by_key(|p| {
        (std::cmp::Reverse(p.is_alive()), std::cmp::Reverse(p.hp()), p.placement())
    });
    parent
        .spawn(ui::panel(Node {
            width: px(118),
            flex_shrink: 0.0,
            padding: UiRect::axes(px(14), px(16)),
            ..column(8.0)
        }))
        .with_children(|col| {
            col.spawn(heading("Players", 15.0, color::GOLD_LIGHT));
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
                            if !alive {
                                r.spawn(kit::icon(kit::img::ICON_SKULL, 16.0));
                            }
                            r.spawn((Node { flex_grow: 1.0, ..default() }, Pickable::IGNORE))
                                .with_child(heading(p.name(), 12.0, name_ink));
                            let right = if alive {
                                p.hp().to_string()
                            } else {
                                p.placement().map(ordinal).unwrap_or_default()
                            };
                            let ink = if alive { color::TEXT } else { color::FAINT };
                            r.spawn(text(right, 12.0, ink));
                        });
                    if alive {
                        // You are gold; others green, or red when low.
                        let fill = if me {
                            kit::img::FILL_TIME
                        } else if p.hp() < 35 {
                            kit::img::FILL_HP_LOW
                        } else {
                            kit::img::FILL_HP
                        };
                        kit::spawn_bar(entry, fill, p.hp() as f32, percent(100), 12.0, ());
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
        board.spawn(row(0.0)).with_children(|cells| {
            for y in 0..BOARD_LANES {
                let pos = Pos::new(x, y);
                // The first cell of each row is narrower by the board's left border.
                let w = if y == 0 { CELL - LINE } else { CELL };
                let cell_node = Node {
                    width: px(w),
                    height: px(CELL - if x == SIDE_DEPTH - 1 { LINE } else { 0.0 }),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::new(px(0), px(LINE), px(0), px(LINE)),
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
                        BorderColor::all(color::GRID),
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
                    ui::ButtonStyle::flat(Color::NONE, color::CELL_HOVER, Color::NONE),
                    Node { width: px(slot_w), height: px(58), ..default() },
                ))
                // A carved wooden slot; the flat style only adds a hover glow.
                .insert((DropTarget::Bench(i), kit::inset()))
                .remove::<ui::HoverSkin>()
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
                padding: UiRect::new(px(16), px(16), px(8), px(4)),
                ..row(10.0)
            },
            kit::tiled(kit::img::HEADER_WOOD, true, false),
            // Dropping a unit onto the shop sells it.
            DropTarget::Sell,
        ))
        .with_children(|footer| {
            footer.spawn(Node { flex_grow: 1.0, ..row(8.0) }).with_children(|offers| {
                for (i, offer) in run.shop().iter().enumerate() {
                    let node = Node {
                        flex_grow: 1.0,
                        flex_basis: px(0),
                        min_width: px(0),
                        height: px(94),
                        padding: UiRect::axes(px(14), px(12)),
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
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    ..node
                                },
                                kit::inset(),
                            ))
                            .with_child(heading("Sold", 14.0, color::FAINT));
                        continue;
                    };
                    let merges = run.buy_would_merge(c);
                    let raises = run.would_raise_synergy(c);
                    let owned = run.copies_owned(c);
                    let (note, note_ink) = if merges {
                        (format!("Owned {owned} - merges!"), INK_GREEN)
                    } else if raises {
                        let next = run.synergies().count(c.class()) + 1;
                        (format!("{next} {} - activates", c.class().plural()), INK_GOLD)
                    } else if owned > 0 {
                        (format!("Owned {owned}"), color::INK_MUTED)
                    } else {
                        ("New".to_string(), color::INK_FAINT)
                    };
                    offers
                        .spawn(ui::button(UiAction::Buy(i), ui::PARCHMENT, node))
                        .insert(cards::unit_tooltip(c, None, None))
                        .with_children(|card| {
                            card.spawn((row(8.0), Pickable::IGNORE)).with_children(|r| {
                                r.spawn((
                                    Node {
                                        width: px(44),
                                        height: px(44),
                                        flex_shrink: 0.0,
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    kit::sliced(kit::rarity_frame(c.rarity()), 6.0),
                                    Pickable::IGNORE,
                                ))
                                .with_child(sprite_node(
                                    sprites.character(c),
                                    1.0,
                                    false,
                                ));
                                r.spawn((
                                    Node { min_width: px(0), ..column(1.0) },
                                    Pickable::IGNORE,
                                ))
                                .with_children(|col| {
                                    col.spawn(heading(c.short_name(), 15.0, color::INK));
                                    col.spawn(row(4.0)).with_children(|line| {
                                        line.spawn(kit::icon(kit::class_icon(c.class()), 16.0));
                                        line.spawn(text(
                                            format!("{} {}", c.rarity().name(), c.class().name()),
                                            12.0,
                                            rarity_ink(c.rarity()),
                                        ));
                                    });
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
                                r.spawn(heading(note, 12.0, note_ink));
                                r.spawn(row(3.0)).with_children(|price| {
                                    price.spawn(kit::icon(kit::img::ICON_COIN, 18.0));
                                    price.spawn(heading(
                                        c.rarity().cost().to_string(),
                                        15.0,
                                        color::INK,
                                    ));
                                });
                            });
                        });
                }
            });
            footer.spawn(Node { width: px(150), ..column(6.0) }).with_children(|col| {
                col.spawn(ui::button(
                    UiAction::Reroll,
                    SECONDARY,
                    Node { width: percent(100), height: px(40), ..row(6.0) },
                ))
                .with_children(|b| {
                    b.spawn(heading("Reroll", 15.0, color::TEXT));
                    b.spawn(kit::icon(kit::img::ICON_COIN, 18.0));
                    b.spawn(heading(REROLL_COST.to_string(), 15.0, color::TEXT));
                    b.spawn(text("D", 12.0, color::MUTED));
                });
                col.spawn(ui::button(
                    UiAction::Ready,
                    PRIMARY,
                    Node { width: percent(100), height: px(50), ..default() },
                ))
                .with_child(heading("Ready!", 20.0, color::ON_GOLD));
            });
        });
}

/// Rarity colours dark enough to read on parchment.
fn rarity_ink(r: battler_sim::Rarity) -> Color {
    match r {
        battler_sim::Rarity::Common => color::INK_MUTED,
        battler_sim::Rarity::Rare => Color::srgb(0.157, 0.431, 0.588),
        battler_sim::Rarity::Epic => Color::srgb(0.463, 0.212, 0.639),
    }
}

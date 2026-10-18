//! The timed planning phase: header, board, bench, shop and the selected-unit
//! panel. The card editor drawer (in `editor.rs`) replaces the panel and shop
//! while it's open.

use battler_sim::run::economy::{
    BENCH_SIZE, BOARD_HEIGHT, BOARD_WIDTH, PLAYER_COLUMNS, REROLL_COST, UNIT_COST,
};
use battler_sim::run::{Location, OwnedUnit, Run};
use battler_sim::{Class, Level, Pos};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::cards;
use crate::editor::{self, HoveredAbility};
use crate::state::{GameRun, Planning, Rev, RunPhase};
use crate::ui::{
    self, ActionQueue, Clickable, DragSource, DropTarget, PRIMARY, SECONDARY, UiAction, color,
    column, row, spawn_button, text,
};

pub struct PlanningPlugin;

impl Plugin for PlanningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RunPhase::Planning), enter).add_systems(
            Update,
            (rebuild, tick_timer, update_cells).chain().run_if(in_state(RunPhase::Planning)),
        );
    }
}

pub const CELL: f32 = 58.0;
pub const GAP: f32 = 4.0;
const BOARD_PX: f32 = CELL * 8.0 + GAP * 7.0;

/// Seconds allowed for planning this round.
pub fn planning_seconds(run: &Run) -> f32 {
    let units = run.board().len() + run.bench().iter().flatten().count();
    if units >= 5 { 75.0 } else { 60.0 }
}

fn enter(run: Res<GameRun>, mut planning: ResMut<Planning>, mut rev: ResMut<Rev>) {
    planning.time_left = planning_seconds(&run.0);
    planning.editor_open = false;
    planning.toast = None;
    if planning.selected.is_some_and(|id| run.0.locate(id).is_none()) {
        planning.selected = None;
    }
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

/// A player-side board cell, coloured each frame (hover, reach highlight).
#[derive(Component)]
struct CellView(Pos);

fn tick_timer(
    time: Res<Time<Real>>,
    run: Res<GameRun>,
    mut planning: ResMut<Planning>,
    mut queue: ResMut<ActionQueue>,
    mut timer_text: Query<&mut Text, (With<TimerText>, Without<ToastText>)>,
    mut timer_bar: Query<&mut Node, With<TimerBar>>,
    mut toast_text: Query<&mut Text, (With<ToastText>, Without<TimerText>)>,
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
    let total = planning_seconds(&run.0);
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
}

/// Colours player-side cells: hover feedback, plus the reach of a hovered
/// signature block around the selected unit while the editor is open.
fn update_cells(
    run: Res<GameRun>,
    planning: Res<Planning>,
    hovered_ability: Res<HoveredAbility>,
    mut cells: Query<(&CellView, &Hovered, &mut BackgroundColor, &mut BorderColor)>,
) {
    let origin = planning.selected.and_then(|id| run.0.locate(id)).and_then(|loc| match loc {
        Location::Board(p) => Some(p),
        Location::Bench(_) => None,
    });
    let unit_class = planning
        .selected
        .and_then(|id| run.0.locate(id))
        .and_then(|l| run.0.unit_at(l))
        .map(|u| u.class);
    let reach = match (planning.editor_open, hovered_ability.0, origin, unit_class) {
        (true, Some(a), Some(o), Some(c)) => Some((o, cards::ability_reach(a, c))),
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
    roots: Query<Entity, With<PlanningRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let run = &run.0;
    let stage = ui::spawn_stage(
        &mut commands,
        (PlanningRoot, DespawnOnExit(RunPhase::Planning)),
        color::BG,
    );
    commands.entity(stage).with_children(|stage| {
        header(stage, run, &planning);
        stage
            .spawn(Node {
                flex_grow: 1.0,
                flex_direction: FlexDirection::Row,
                min_height: px(0),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: px(14),
                    padding: UiRect::top(px(18)),
                    ..default()
                })
                .with_children(|left| {
                    board_section(left, run, &planning);
                    bench_section(left, run, &planning);
                });
                if planning.editor_open {
                    editor::spawn_drawer(body, run, &planning);
                } else {
                    unit_panel(body, run, &planning);
                }
            });
        if !planning.editor_open {
            shop(stage, run);
        }
    });
}

fn header(stage: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    stage
        .spawn((
            Node {
                height: px(64),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(24)),
                border: UiRect::bottom(px(1)),
                ..row(28.0)
            },
            BackgroundColor(color::BAR),
            BorderColor::all(color::LINE),
        ))
        .with_children(|h| {
            h.spawn(Node { width: px(220), ..column(2.0) }).with_children(|c| {
                c.spawn(text(format!("Round {}", run.round()), 20.0, color::TEXT));
                c.spawn(text(
                    format!("Planning - board size {}", run.board_limit()),
                    13.0,
                    color::MUTED,
                ));
            });
            h.spawn(Node { flex_grow: 1.0, align_items: AlignItems::Center, ..column(6.0) })
                .with_children(|c| {
                    let secs = planning.time_left.ceil() as u32;
                    c.spawn((
                        text(format!("{}:{:02}", secs / 60, secs % 60), 24.0, color::GOLD_LIGHT),
                        TimerText,
                    ));
                    c.spawn((
                        Node {
                            width: px(320),
                            height: px(6),
                            border_radius: BorderRadius::all(px(3)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(color::LINE),
                    ))
                    .with_child((
                        Node { width: percent(100), height: px(6), ..default() },
                        BackgroundColor(color::GOLD),
                        TimerBar,
                    ));
                });
            h.spawn(Node { width: px(380), justify_content: JustifyContent::FlexEnd, ..row(22.0) })
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
        .spawn(Node {
            position_type: PositionType::Absolute,
            top: px(70),
            left: px(0),
            right: px(0),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_child((text("", 15.0, color::BAD), ToastText));
}

fn stat(parent: &mut ChildSpawnerCommands, label: &str, value: String, value_color: Color) {
    parent.spawn(Node { align_items: AlignItems::FlexEnd, ..column(4.0) }).with_children(|c| {
        c.spawn(text(label, 13.0, color::MUTED));
        c.spawn(text(value, 18.0, value_color));
    });
}

pub fn class_letter(class: Class) -> &'static str {
    match class {
        Class::Knight => "K",
        Class::Ranger => "R",
        Class::Mage => "M",
        Class::Cleric => "C",
        Class::Rogue => "Ro",
        Class::Berserker => "B",
    }
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

/// A draggable, clickable unit tile.
fn unit_tile(
    parent: &mut ChildSpawnerCommands,
    unit: &OwnedUnit,
    loc: Location,
    size: f32,
    selected: bool,
) {
    let ring = if selected { color::GOLD } else { color::BLUE };
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
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(3),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(color::BLUE_FILL),
            BorderColor::all(ring),
        ))
        .with_children(|t| {
            t.spawn(text(class_letter(unit.class), 17.0, color::BLUE_TEXT));
            spawn_pips(t, unit.level);
        });
}

fn board_section(parent: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    parent
        .spawn(Node {
            width: px(BOARD_PX),
            justify_content: JustifyContent::SpaceBetween,
            ..row(0.0)
        })
        .with_children(|r| {
            r.spawn(text(
                format!("Your side - {} / {} placed", run.board().len(), run.board_limit()),
                13.0,
                color::MUTED,
            ));
            r.spawn(text("Enemy side - revealed at battle start", 13.0, color::MUTED));
        });
    parent.spawn(column(GAP)).with_children(|board| {
        for y in 0..BOARD_HEIGHT {
            board.spawn(row(GAP)).with_children(|cells| {
                for x in 0..BOARD_WIDTH {
                    let pos = Pos::new(x, y);
                    let cell_node = Node {
                        width: px(CELL),
                        height: px(CELL),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    };
                    if x >= PLAYER_COLUMNS {
                        cells.spawn((
                            cell_node,
                            BackgroundColor(color::CELL_ENEMY),
                            BorderColor::all(color::LINE),
                        ));
                        continue;
                    }
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
                                unit_tile(
                                    cell,
                                    unit,
                                    loc,
                                    48.0,
                                    planning.selected == Some(unit.id),
                                );
                            }
                        });
                }
            });
        }
    });
}

fn bench_section(parent: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    let used = run.bench().iter().flatten().count();
    parent.spawn(Node { width: px(BOARD_PX), ..column(8.0) }).with_children(|b| {
        b.spawn(text(
            format!("Bench - {used} / {BENCH_SIZE} - inactive copies, saved for merging"),
            13.0,
            color::MUTED,
        ));
        b.spawn(row(6.0)).with_children(|slots| {
            let slot_w = (BOARD_PX - 6.0 * (BENCH_SIZE as f32 - 1.0)) / BENCH_SIZE as f32;
            for i in 0..BENCH_SIZE {
                slots
                    .spawn(ui::button(
                        UiAction::ClickBench(i),
                        ui::ButtonStyle {
                            fill: color::PANEL,
                            hover: color::PANEL_2,
                            border: color::LINE_STRONG,
                        },
                        Node { width: px(slot_w), height: px(56), ..default() },
                    ))
                    .insert(DropTarget::Bench(i))
                    .with_children(|slot| {
                        let loc = Location::Bench(i);
                        if let Some(unit) = run.unit_at(loc) {
                            unit_tile(slot, unit, loc, 44.0, planning.selected == Some(unit.id));
                        }
                    });
            }
        });
    });
}

fn unit_panel(parent: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    parent
        .spawn(Node {
            width: px(340),
            padding: UiRect::new(px(0), px(24), px(18), px(0)),
            ..default()
        })
        .with_children(|wrap| {
            wrap.spawn(ui::panel(Node {
                width: percent(100),
                padding: UiRect::all(px(18)),
                ..column(14.0)
            }))
            .with_children(|p| {
                let selected = planning
                    .selected
                    .and_then(|id| run.locate(id))
                    .and_then(|loc| run.unit_at(loc).map(|u| (loc, u)));
                let Some((loc, unit)) = selected else {
                    p.spawn(text("No unit selected", 18.0, color::TEXT));
                    for line in [
                        "Buy units in the shop below.",
                        "Click a unit to select it, then click a cell",
                        "or bench slot to move it - or just drag it.",
                        "Drag a unit onto the shop to sell it.",
                    ] {
                        p.spawn(text(line, 14.0, color::MUTED));
                    }
                    return;
                };
                let stats = unit.level.scale(unit.class.base_stats());
                p.spawn(row(14.0)).with_children(|r| {
                    r.spawn((
                        Node {
                            width: px(52),
                            height: px(52),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(color::BLUE_FILL),
                        BorderColor::all(color::GOLD),
                    ))
                    .with_child(text(
                        class_letter(unit.class),
                        22.0,
                        color::BLUE_TEXT,
                    ));
                    r.spawn(column(3.0)).with_children(|c| {
                        c.spawn(text(unit.class.name(), 18.0, color::TEXT));
                        let place = match loc {
                            Location::Board(_) => "on the board",
                            Location::Bench(_) => "on the bench",
                        };
                        c.spawn(text(
                            format!(
                                "Level {} - {} - {place}",
                                unit.level.number(),
                                unit.class.role()
                            ),
                            13.0,
                            color::MUTED,
                        ));
                    });
                });
                p.spawn(row(24.0)).with_children(|r| {
                    for (label, value) in
                        [("HP", stats.max_hp), ("Attack", stats.attack), ("Range", stats.range)]
                    {
                        r.spawn(column(2.0)).with_children(|c| {
                            c.spawn(text(label, 12.0, color::MUTED));
                            c.spawn(text(value.to_string(), 15.0, color::TEXT));
                        });
                    }
                });
                let max = unit.level.max_nodes();
                p.spawn(column(8.0)).with_children(|c| {
                    c.spawn(text(
                        format!("Cards - {} / {max}", unit.program.len()),
                        13.0,
                        color::MUTED,
                    ));
                    for (i, node) in unit.program.nodes().iter().enumerate() {
                        c.spawn((
                            Node {
                                padding: UiRect::axes(px(10), px(7)),
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BackgroundColor(color::PANEL_2),
                        ))
                        .with_child(text(
                            format!("{} {}", i + 1, cards::card_label(node)),
                            13.0,
                            color::TEXT,
                        ));
                    }
                    for _ in unit.program.len()..max {
                        c.spawn((
                            Node {
                                padding: UiRect::axes(px(10), px(7)),
                                border: UiRect::all(px(1)),
                                border_radius: BorderRadius::all(px(8)),
                                ..default()
                            },
                            BorderColor::all(color::LINE_STRONG),
                        ))
                        .with_child(text(
                            "Empty card slot",
                            13.0,
                            color::FAINT,
                        ));
                    }
                    c.spawn(text(
                        "Locked: attack nearest, advancing if needed",
                        12.0,
                        color::FAINT,
                    ));
                });
                p.spawn(row(10.0)).with_children(|r| {
                    spawn_button(
                        r,
                        "Edit cards",
                        UiAction::OpenEditor,
                        ui::TOGGLED,
                        px(150),
                        px(44),
                    );
                    let refund = unit.level.copies() * UNIT_COST;
                    spawn_button(
                        r,
                        &format!("Sell - {refund} g"),
                        UiAction::Sell,
                        SECONDARY,
                        px(130),
                        px(44),
                    );
                });
                p.spawn(text(
                    format!("Copies owned: {} of 9 for level 3", run.copies_owned(unit.class)),
                    12.0,
                    color::FAINT,
                ));
            });
        });
}

fn shop(stage: &mut ChildSpawnerCommands, run: &Run) {
    stage
        .spawn((
            Node {
                height: px(132),
                flex_shrink: 0.0,
                padding: UiRect::horizontal(px(24)),
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
                        height: px(96),
                        padding: UiRect::axes(px(12), px(10)),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    };
                    let Some(class) = *offer else {
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
                            .with_child(text("Sold", 14.0, color::FAINT));
                        continue;
                    };
                    let merges = run.buy_would_merge(class);
                    let border = if merges { color::GOOD } else { color::LINE_STRONG };
                    offers
                        .spawn(ui::button(
                            UiAction::Buy(i),
                            ui::ButtonStyle { fill: color::PANEL, hover: color::PANEL_2, border },
                            node,
                        ))
                        .with_children(|card| {
                            card.spawn(row(10.0)).with_children(|r| {
                                r.spawn((
                                    Node {
                                        width: px(40),
                                        height: px(40),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        border: UiRect::all(px(2)),
                                        border_radius: BorderRadius::all(px(8)),
                                        ..default()
                                    },
                                    BackgroundColor(color::BLUE_FILL),
                                    BorderColor::all(color::BLUE),
                                    Pickable::IGNORE,
                                ))
                                .with_child(text(
                                    class_letter(class),
                                    16.0,
                                    color::BLUE_TEXT,
                                ));
                                r.spawn((column(2.0), Pickable::IGNORE)).with_children(|c| {
                                    c.spawn(text(class.name(), 15.0, color::TEXT));
                                    c.spawn(text(class.role(), 12.0, color::MUTED));
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
                                let owned = run.copies_owned(class);
                                let (label, c) = if merges {
                                    (format!("Owned {owned} - merges on buy"), color::GOOD)
                                } else if owned > 0 {
                                    (format!("Owned {owned}"), color::MUTED)
                                } else {
                                    ("Not owned".to_string(), color::FAINT)
                                };
                                r.spawn(text(label, 12.0, c));
                                r.spawn(text(format!("{UNIT_COST} g"), 12.0, color::GOLD_LIGHT));
                            });
                        });
                }
            });
            footer.spawn(Node { width: px(150), ..column(10.0) }).with_children(|c| {
                spawn_button(
                    c,
                    &format!("Reroll - {REROLL_COST} g"),
                    UiAction::Reroll,
                    SECONDARY,
                    percent(100),
                    px(44),
                );
                spawn_button(c, "Ready", UiAction::Ready, PRIMARY, percent(100), px(48));
            });
        });
}

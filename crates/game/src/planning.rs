//! The round screen, used for the whole run: planning and battle happen on the
//! same battlefield, and each round's result pops up over the next planning
//! phase while its timer is already running.
//!
//! Layout: a wooden header (round, level, timer or battle controls, health,
//! gold); synergy icons and the bench on the left; the battlefield in the
//! middle with an arrow pointing at the enemy; the players on the right; the
//! program panel lying over the right side (docked to a tab when closed); and
//! the shop along the bottom. Details live in hover tooltips, not on screen.
//!
//! The board is isometric (see `field.rs`): a diamond with your half at the
//! bottom-left and the enemy's at the top-right.

use battler_sim::monster::mirror;
use battler_sim::run::economy::{BENCH_SIZE, BOARD_LANES, REROLL_COST, SIDE_DEPTH, XP_COST};
use battler_sim::run::{Lobby, Location, OwnedUnit, Player, RoundKind};
use battler_sim::{Class, Level, Pos};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::battle::{Playback, battle_controls, spawn_battle_field, spawn_trace_panel};
use crate::cards;
use crate::editor::{self, HoveredAbility};
use crate::field::{self, FIELD_H, FIELD_W, FieldHover};
use crate::kit;
use crate::meters;
use crate::result::{ResultCountdown, ordinal, spawn_result_popup};
use crate::sprites::{SPRITE_PX, Sprites, battle_scale, sprite_node};
use crate::state::{AppState, GameRun, LobbyExt, ME, Planning, Rev, RunPhase, Settings};
use crate::ui::{
    self, ActionQueue, Clickable, DragSource, DropTarget, PRIMARY, SECONDARY, UiAction, color,
    column, heading, row, text,
};

pub struct PlanningPlugin;

impl Plugin for PlanningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(RunPhase::Planning), enter)
            // The screen rebuilds in both phases: the battle plays on it.
            .add_systems(Update, rebuild.run_if(in_state(AppState::InRun)))
            .add_systems(
                Update,
                (hotkeys, tick_timers, update_cells, pulse_rings)
                    .chain()
                    .after(rebuild)
                    .run_if(in_state(RunPhase::Planning)),
            );
    }
}

/// Dark green and gold that read on parchment.
const INK_GREEN: Color = Color::srgb(0.180, 0.420, 0.180);
const INK_GOLD: Color = Color::srgb(0.541, 0.353, 0.0);

/// A bench slot.
const SLOT: f32 = 66.0;

/// The header's and the shop's heights.
const HEADER_H: f32 = 56.0;
const SHOP_H: f32 = 104.0;

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

/// A shop offer that would merge into an upgrade; its gold ring pulses.
#[derive(Component)]
struct UpgradeRing;

const UPGRADE_GOLD: Color = Color::srgb(1.0, 0.86, 0.30);

/// Pulses the upgrade rings in the shop.
fn pulse_rings(time: Res<Time<Real>>, mut rings: Query<&mut Outline, With<UpgradeRing>>) {
    let glow = 0.6 + 0.4 * (time.elapsed_secs() * 5.0).sin();
    for mut ring in &mut rings {
        ring.color = UPGRADE_GOLD.with_alpha(glow);
    }
}

/// A highlight over one cell of your half: shown on hover, or in blue for
/// the reach of a hovered ability. Updated each frame.
#[derive(Component)]
struct CellMark(Pos);

/// D rerolls, F buys XP, E sells the unit under the pointer, Tab opens or
/// docks the program panel.
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
    if keys.just_pressed(KeyCode::Tab) {
        queue.0.push(UiAction::ToggleEditor);
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

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn tick_timers(
    time: Res<Time<Real>>,
    run: Res<GameRun>,
    mut planning: ResMut<Planning>,
    mut queue: ResMut<ActionQueue>,
    mut timer_text: Query<&mut Text, (With<TimerText>, Without<ToastText>)>,
    mut timer_bar: Query<&mut Node, (With<TimerBar>, Without<NoticeBox>)>,
    mut toast_text: Query<&mut Text, (With<ToastText>, Without<TimerText>)>,
    mut notice: Query<&mut Node, (With<NoticeBox>, Without<TimerBar>)>,
    mut countdown: Query<
        &mut Text,
        (With<ResultCountdown>, Without<TimerText>, Without<ToastText>),
    >,
    mut rev: ResMut<Rev>,
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

    // The round result pop-up counts down, then closes itself.
    if let Some((_, left)) = planning.result.as_mut() {
        *left -= dt;
        let secs = left.max(0.0).ceil() as u32;
        for mut t in &mut countdown {
            **t = format!("OK ({secs})");
        }
    }
    if planning.result.as_ref().is_some_and(|(_, left)| *left <= 0.0) {
        planning.result = None;
        rev.bump();
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

/// Highlights cells of your half: the one under the pointer, plus the reach
/// of a hovered signature block around the selected unit.
fn update_cells(
    run: Res<GameRun>,
    planning: Res<Planning>,
    hovered_ability: Res<HoveredAbility>,
    hover: Res<FieldHover>,
    mut marks: Query<(&CellMark, &mut ImageNode, &mut Visibility)>,
) {
    let selected = editor::selected_unit(run.0.me(), &planning);
    let reach = match (hovered_ability.0, selected) {
        (Some(a), Some((Location::Board(origin), unit))) => {
            Some((origin, cards::ability_reach(a, unit.character)))
        }
        _ => None,
    };
    for (mark, mut image, mut vis) in &mut marks {
        let in_reach = reach.is_some_and(|(o, r)| {
            let d = o.distance(mark.0);
            d > 0 && d <= r
        });
        let shown = if in_reach {
            Some(kit::img::TILE_REACH)
        } else if hover.0 == Some(mark.0) {
            Some(kit::img::TILE_HOVER)
        } else {
            None
        };
        let want = if shown.is_some() { Visibility::Inherited } else { Visibility::Hidden };
        if *vis != want {
            *vis = want;
        }
        if let Some(handle) = shown
            && image.image != handle
        {
            image.image = handle;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    run: Res<GameRun>,
    planning: Res<Planning>,
    settings: Res<Settings>,
    phase: Res<State<RunPhase>>,
    playback: Option<Res<Playback>>,
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
    // The battle plays on this screen's field, so it is the same screen for
    // the whole run (despawned only when the run ends).
    let battle = match (phase.get(), playback.as_deref()) {
        (RunPhase::Battle, Some(pb)) => Some(pb),
        _ => None,
    };
    let stage =
        ui::spawn_stage(&mut commands, (PlanningRoot, DespawnOnExit(AppState::InRun)), color::BG);
    commands.entity(stage).with_children(|stage| {
        header(stage, lobby, &planning, battle.is_some(), &settings);
        stage.spawn(Node { flex_grow: 1.0, min_height: px(0), ..default() }).with_children(
            |body| {
                // Left: synergies (icons only) and the bench.
                body.spawn(Node {
                    position_type: PositionType::Absolute,
                    left: px(16),
                    top: px(12),
                    ..row(12.0)
                })
                .with_children(|left| {
                    left.spawn(Node { align_items: AlignItems::FlexStart, ..row(0.0) })
                        .with_children(|c| synergy_icons(c, run));
                    bench_grid(left, run, &planning, &sprites);
                });
                // Centre: the battlefield, with an arrow pointing at the enemy.
                body.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: px((ui::STAGE_W - FIELD_W) / 2.0),
                        top: px(36),
                        width: px(FIELD_W),
                        height: px(FIELD_H),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_children(|mid| {
                    match battle {
                        Some(pb) => spawn_battle_field(mid, &pb.battle, &sprites),
                        None => planning_field(mid, lobby, &planning, &sprites),
                    }
                    up_arrow(mid, lobby, battle);
                });
                // Right: the players list and the battle meters, each shown or
                // hidden with a square button at the edge.
                body.spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(10),
                    top: px(12),
                    bottom: px(12),
                    align_items: AlignItems::FlexStart,
                    ..row(8.0)
                })
                .with_children(|right| {
                    right
                        .spawn(Node {
                            max_height: percent(100),
                            align_items: AlignItems::FlexEnd,
                            ..column(8.0)
                        })
                        .with_children(|panels| {
                            if settings.show_players {
                                standings(panels, lobby);
                            }
                            if settings.show_meters {
                                meters::spawn_meters(
                                    panels,
                                    playback.as_deref(),
                                    &settings,
                                    &sprites,
                                );
                            }
                        });
                    right.spawn(column(6.0)).with_children(|buttons| {
                        square_button(
                            buttons,
                            UiAction::TogglePlayers,
                            settings.show_players,
                            kit::img::ICON_CROWN,
                            "Players",
                            "Every player's health. Click to show or hide.",
                        );
                        square_button(
                            buttons,
                            UiAction::ToggleMeters,
                            settings.show_meters,
                            kit::img::ICON_SWORD,
                            "Damage and healing",
                            "How much each of your units has dealt or healed, live in battle.",
                        );
                    });
                });
                // Overlays on the right: the program panel while planning, or
                // the card trace during battle if it is turned on.
                if battle.is_none() {
                    program_overlay(body, run, &planning, &sprites);
                } else if settings.card_trace {
                    body.spawn(Node {
                        position_type: PositionType::Absolute,
                        right: px(0),
                        top: px(0),
                        bottom: px(0),
                        width: px(320),
                        ..default()
                    })
                    .with_children(spawn_trace_panel);
                }
            },
        );
        shop(stage, run, &sprites, battle.is_some());
        notice(stage, &planning);
        if let (None, Some((report, _))) = (battle, &planning.result) {
            spawn_result_popup(stage, lobby, report, &sprites);
        }
    });
}

fn header(
    stage: &mut ChildSpawnerCommands,
    lobby: &Lobby,
    planning: &Planning,
    in_battle: bool,
    settings: &Settings,
) {
    let run = lobby.me();
    stage
        .spawn((
            Node {
                height: px(HEADER_H),
                flex_shrink: 0.0,
                padding: UiRect::new(px(16), px(16), px(0), px(6)),
                ..row(18.0)
            },
            kit::tiled(kit::img::HEADER_WOOD, true, false),
        ))
        .with_children(|h| {
            // The round on a red ribbon.
            h.spawn((
                Node {
                    width: px(170),
                    height: px(42),
                    flex_shrink: 0.0,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                kit::sliced(kit::img::BANNER, 6.0),
                Hovered::default(),
                ui::Tooltip {
                    title: format!("Round {}", lobby.round()),
                    accent: color::GOLD_LIGHT,
                    lines: vec![(cards::round_label(lobby.round_kind()), color::TEXT)],
                },
            ))
            .with_child(heading(format!("ROUND {}", lobby.round()), 18.0, color::TEXT));
            // Level and XP.
            h.spawn(row(8.0)).with_children(|lv| {
                lv.spawn(kit::icon(kit::img::ICON_CROWN, 30.0));
                lv.spawn(Node { width: px(132), ..column(3.0) }).with_children(|c| {
                    // "Level 3" with the XP towards the next level ("8/16") at the right.
                    c.spawn(Node {
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Baseline,
                        ..row(6.0)
                    })
                    .with_children(|r| {
                        r.spawn(heading(format!("Level {}", run.level()), 14.0, color::TEXT));
                        let xp = match run.xp_needed() {
                            Some(n) => format!("{}/{n}", run.xp()),
                            None => "max".to_string(),
                        };
                        r.spawn(heading(xp, 12.0, color::MUTED));
                    });
                    let fill =
                        run.xp_needed().map_or(100.0, |n| 100.0 * run.xp() as f32 / n as f32);
                    kit::spawn_bar(c, kit::img::FILL_XP, fill, percent(100), 14.0, ());
                });
                if !in_battle {
                    lv.spawn(ui::button(
                        UiAction::BuyXp,
                        SECONDARY,
                        Node { height: px(42), padding: UiRect::horizontal(px(10)), ..row(5.0) },
                    ))
                    .insert(ui::Tooltip {
                        title: "Buy XP (F)".to_string(),
                        accent: color::BLUE_TEXT,
                        lines: vec![(
                            format!("{XP_COST} gold for {XP_COST} XP. Levels raise board size and shop odds."),
                            color::TEXT,
                        )],
                    })
                    .with_children(|b| {
                        b.spawn(kit::icon(kit::img::ICON_STAR, 20.0));
                        b.spawn(kit::icon(kit::img::ICON_COIN, 18.0));
                        b.spawn(heading(XP_COST.to_string(), 15.0, color::TEXT));
                    });
                }
            });
            // The timer while planning; battle controls during battle.
            h.spawn(Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..column(3.0)
            })
            .with_children(|c| {
                if in_battle {
                    battle_controls(c, settings, false);
                    return;
                }
                let secs = planning.time_left.ceil() as u32;
                c.spawn(row(8.0)).with_children(|r| {
                    r.spawn(kit::icon(kit::img::ICON_HOURGLASS, 24.0));
                    r.spawn((
                        ui::outlined(format!("{}:{:02}", secs / 60, secs % 60), 22.0, color::GOLD_LIGHT),
                        TimerText,
                    ));
                });
                kit::spawn_bar(c, kit::img::FILL_TIME, 100.0, px(220), 12.0, TimerBar);
            });
            // Health and gold; details on hover.
            h.spawn(row(16.0)).with_children(|r| {
                stat(r, kit::img::ICON_HEART, run.hp().to_string(), color::TEXT, ui::Tooltip {
                    title: "Health".to_string(),
                    accent: color::BAD,
                    lines: vec![("Lose a battle and you lose health. At 0 you are out.".to_string(), color::TEXT)],
                });
                let streak = match run.streak() {
                    s if s >= 2 => format!("{s} wins in a row"),
                    s if s <= -2 => format!("{} losses in a row", -s),
                    _ => "No streak".to_string(),
                };
                stat(r, kit::img::ICON_COIN, run.gold().to_string(), color::GOLD_LIGHT, ui::Tooltip {
                    title: "Gold".to_string(),
                    accent: color::GOLD_LIGHT,
                    lines: vec![
                        (format!("+{} interest next round", run.interest_preview()), color::TEXT),
                        (streak, color::TEXT),
                    ],
                });
            });
        });
    // The toast line floats just under the header.
    stage
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(HEADER_H + 6.0),
                left: px(0),
                right: px(0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Pickable::IGNORE,
            GlobalZIndex(90),
        ))
        .with_child((ui::outlined("", 16.0, color::BAD), ToastText));
}

/// An icon and a big number, with a tooltip for the details.
fn stat(
    parent: &mut ChildSpawnerCommands,
    icon: Handle<Image>,
    value: String,
    value_color: Color,
    tooltip: ui::Tooltip,
) {
    parent.spawn((row(6.0), Hovered::default(), tooltip)).with_children(|r| {
        r.spawn(kit::icon(icon, 28.0));
        r.spawn(ui::outlined(value, 20.0, value_color));
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
    // 2x when the tile fits it, otherwise 1x.
    let scale = if size >= 64.0 { 2.0 } else { 1.0 };
    parent
        .spawn((
            Button,
            Hovered::default(),
            Clickable(UiAction::SelectUnit(loc)),
            DragSource::Unit(loc),
            // Let drops "see through" the tile being dragged to the target below.
            Pickable { should_block_lower: false, is_hoverable: true },
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

/// A square toggle button with an icon; lit while its panel is open.
fn square_button(
    parent: &mut ChildSpawnerCommands,
    action: UiAction,
    on: bool,
    icon: Handle<Image>,
    title: &str,
    help: &str,
) {
    let style = if on { ui::TOGGLED } else { SECONDARY };
    parent
        .spawn(ui::button(action, style, Node { width: px(40), height: px(40), ..default() }))
        .insert(ui::Tooltip {
            title: title.to_string(),
            accent: color::GOLD_LIGHT,
            lines: vec![(help.to_string(), color::TEXT)],
        })
        .with_child(kit::icon(icon, 24.0));
}

/// The lobby standings: a narrow sidebar, every player's health, best first.
fn standings(parent: &mut ChildSpawnerCommands, lobby: &Lobby) {
    let mut players: Vec<&Player> = lobby.players().iter().collect();
    players.sort_by_key(|p| {
        (std::cmp::Reverse(p.is_alive()), std::cmp::Reverse(p.hp()), p.placement())
    });
    parent
        .spawn(ui::panel(Node {
            width: px(132),
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

fn shop(stage: &mut ChildSpawnerCommands, run: &Player, sprites: &Sprites, in_battle: bool) {
    stage
        .spawn((
            Node {
                height: px(SHOP_H),
                flex_shrink: 0.0,
                padding: UiRect::new(px(16), px(16), px(6), px(4)),
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
                        height: px(88),
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
                    let mut card = offers.spawn(ui::button(UiAction::Buy(i), ui::PARCHMENT, node));
                    card.insert(cards::unit_tooltip(c, None, None));
                    // Buying this one would merge into an upgrade: a pulsing gold ring.
                    if merges {
                        card.insert((
                            Outline { width: px(4), offset: px(1), color: UPGRADE_GOLD },
                            UpgradeRing,
                        ));
                    }
                    card.with_children(|card| {
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
                            r.spawn((Node { min_width: px(0), ..column(1.0) }, Pickable::IGNORE))
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
                if in_battle {
                    // No buying during battle: the shop waits for the next round.
                    col.spawn(Node { justify_content: JustifyContent::Center, ..row(0.0) })
                        .with_child(ui::outlined("Battle!", 22.0, color::GOLD_LIGHT));
                    return;
                }
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

/// One tile per class in play: its icon and count, gold-framed when the synergy
/// is active, faded when the class is only on the bench. Words are in the tooltip.
fn synergy_icons(parent: &mut ChildSpawnerCommands, run: &Player) {
    let synergies = run.synergies();
    let on_bench: Vec<Class> = run.bench().iter().flatten().map(|u| u.class()).collect();
    let mut classes: Vec<Class> = Class::ALL
        .into_iter()
        .filter(|&c| synergies.count(c) > 0 || on_bench.contains(&c))
        .collect();
    classes.sort_by_key(|&c| std::cmp::Reverse(synergies.count(c)));
    parent.spawn(column(6.0)).with_children(|col| {
        for class in classes {
            let count = synergies.count(class);
            let (frame, tint) = match count {
                2.. => (kit::img::FRAME_EPIC, Color::WHITE),
                1 => (kit::img::FRAME_COMMON, Color::WHITE),
                _ => (kit::img::FRAME_COMMON, Color::srgba(1.0, 1.0, 1.0, 0.45)),
            };
            col.spawn((
                Node {
                    width: px(52),
                    height: px(52),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                kit::sliced(frame, 6.0).with_color(tint),
                BackgroundColor(if count >= 2 {
                    color::ACTIVE_FILL
                } else {
                    Color::srgba(0.0, 0.0, 0.0, 0.25)
                }),
                Hovered::default(),
                cards::synergy_tooltip(class, &synergies),
            ))
            .with_children(|tile| {
                tile.spawn((
                    ImageNode::new(kit::class_icon(class)).with_color(tint),
                    Node { width: px(28), height: px(28), ..default() },
                    Pickable::IGNORE,
                ));
                tile.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        right: px(3),
                        bottom: px(0),
                        ..default()
                    },
                    Pickable::IGNORE,
                ))
                .with_child(ui::outlined(
                    count.to_string(),
                    15.0,
                    if count >= 2 { color::GOLD_LIGHT } else { color::TEXT },
                ));
            });
        }
    });
}

/// The bench as a 3 x 3 grid of carved slots.
fn bench_grid(
    parent: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    parent
        .spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(3, SLOT),
            row_gap: px(6),
            column_gap: px(6),
            ..default()
        })
        .with_children(|grid| {
            for i in 0..BENCH_SIZE {
                grid.spawn(ui::button(
                    UiAction::ClickBench(i),
                    ui::ButtonStyle::flat(Color::NONE, color::CELL_HOVER, Color::NONE),
                    Node { width: px(SLOT), height: px(SLOT), ..default() },
                ))
                // A carved wooden slot; the flat style only adds a hover glow.
                .insert((DropTarget::Bench(i), kit::inset()))
                .remove::<ui::HoverSkin>()
                .with_children(|slot| {
                    let loc = Location::Bench(i);
                    if let Some(unit) = run.unit_at(loc) {
                        let selected = planning.selected == Some(unit.id);
                        unit_tile(slot, run, unit, loc, SLOT, selected, sprites);
                    }
                });
            }
        });
}

/// An arrow off the board's top-right edge pointing at the enemy side; hover
/// for who it is.
fn up_arrow(parent: &mut ChildSpawnerCommands, lobby: &Lobby, battle: Option<&Playback>) {
    let (title, lines) = match battle.and_then(|pb| pb.opponent) {
        Some(o) => (format!("vs {}", cards::opponent_label(lobby, o)), vec![]),
        None => match lobby.round_kind() {
            RoundKind::Pve(w) => {
                (format!("Next: {}", w.name()), vec![(w.hint().to_string(), color::TEXT)])
            }
            RoundKind::Versus => (
                "Next: versus".to_string(),
                vec![("Your opponent appears when the battle starts.".to_string(), color::TEXT)],
            ),
        },
    };
    let (node, transform) = field::arrow_node();
    parent.spawn((
        ImageNode::new(kit::img::ICON_ARROW_UP),
        node,
        transform,
        ZIndex(500),
        Hovered::default(),
        ui::Tooltip { title, accent: color::RED_TEXT, lines },
    ));
}

/// The battlefield while planning: the isometric board with grid lines over
/// your half and the next opponent, faded, on theirs. One picker underneath
/// turns the pointer into a cell (hover, click, drop); units stand on it and
/// can be clicked, dragged and hovered themselves.
fn planning_field(
    parent: &mut ChildSpawnerCommands,
    lobby: &Lobby,
    planning: &Planning,
    sprites: &Sprites,
) {
    let run = lobby.me();
    let own_cells = || (0..SIDE_DEPTH).flat_map(|x| (0..BOARD_LANES).map(move |y| Pos::new(x, y)));
    parent
        .spawn((
            Node { width: px(FIELD_W), height: px(FIELD_H), flex_shrink: 0.0, ..default() },
            Pickable::IGNORE,
        ))
        .with_children(|f| {
            field::spawn_board(f, true);
            for pos in own_cells() {
                f.spawn((
                    ImageNode::new(kit::img::TILE_HOVER),
                    field::tile_node(pos),
                    Visibility::Hidden,
                    CellMark(pos),
                    Pickable::IGNORE,
                ));
            }
            field::spawn_picker(f);
            enemy_preview(f, lobby, sprites);
            for pos in own_cells() {
                if let Some(unit) = run.unit_at(Location::Board(pos)) {
                    let selected = planning.selected == Some(unit.id);
                    board_unit(f, run, unit, pos, selected, sprites);
                }
            }
        });
}

/// A unit standing on your half while planning, facing the camera: a shadow
/// ring (gold when selected), the sprite, and level pips and card dots at its
/// feet. Drag it, click it to edit its program, hover for its tooltip.
fn board_unit(
    f: &mut ChildSpawnerCommands,
    run: &Player,
    unit: &OwnedUnit,
    pos: Pos,
    selected: bool,
    sprites: &Sprites,
) {
    let c = unit.character;
    let loc = Location::Board(pos);
    let max = run.synergies().card_slots(c, unit.level);
    let feet = field::cell_centre(pos);
    let size = SPRITE_PX * 2.0;
    let ring = if selected { color::GOLD_LIGHT } else { Color::srgba(0.314, 0.588, 1.0, 0.8) };
    f.spawn((
        Button,
        Hovered::default(),
        Clickable(UiAction::SelectUnit(loc)),
        DragSource::Unit(loc),
        DropTarget::Cell(pos),
        // Let the pointer through to the field picker below (drops, hover).
        Pickable { should_block_lower: false, is_hoverable: true },
        ZIndex(field::depth_z(feet.y)),
        field::standing(feet + Vec2::new(0.0, 8.0), size, size + 8.0),
        cards::unit_tooltip(
            c,
            Some(unit.level),
            Some(format!("Sells for {} gold (E)", unit.sell_value())),
        ),
    ))
    .with_children(|u| {
        u.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(size * 0.1),
                bottom: px(2),
                width: px(size * 0.8),
                height: px(14),
                border: UiRect::all(px(if selected { 3.0 } else { 2.0 })),
                border_radius: BorderRadius::all(percent(50)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.08, 0.12, 0.08, 0.45)),
            BorderColor::all(ring),
            Pickable::IGNORE,
        ));
        u.spawn((
            Node { position_type: PositionType::Absolute, left: px(0), bottom: px(8), ..default() },
            Pickable::IGNORE,
        ))
        .with_child(sprite_node(sprites.character(c), 2.0, false));
        // Equipped items as small icons beside the figure.
        if unit.items.iter().any(Option::is_some) {
            u.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    right: px(-10),
                    bottom: px(12),
                    ..column(1.0)
                },
                Pickable::IGNORE,
            ))
            .with_children(|col| {
                for item in unit.items.iter().flatten() {
                    col.spawn((
                        ImageNode::new(sprites.item(*item)),
                        Node { width: px(16), height: px(16), ..default() },
                        Pickable::IGNORE,
                    ));
                }
            });
        }
        u.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(6),
                right: px(6),
                bottom: px(-8),
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

/// The next opponent on the enemy half: a PvE wave stands where it will
/// fight, faded and facing you; a versus round just says so.
fn enemy_preview(f: &mut ChildSpawnerCommands, lobby: &Lobby, sprites: &Sprites) {
    match lobby.round_kind() {
        RoundKind::Pve(wave) => {
            for (pos, spec) in wave.units() {
                let feet = field::cell_centre(mirror(pos));
                let size = SPRITE_PX * battle_scale(spec.kind);
                f.spawn((
                    ImageNode {
                        image: sprites.unit(spec.kind),
                        flip_x: true,
                        color: Color::srgba(1.0, 1.0, 1.0, 0.7),
                        ..default()
                    },
                    field::standing(feet - Vec2::new(0.0, 4.0), size, size),
                    ZIndex(field::depth_z(feet.y)),
                    Pickable::IGNORE,
                ));
            }
        }
        RoundKind::Versus => {
            let at = field::cell_centre(Pos::new(6, 3));
            f.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(at.x - 60.0),
                    top: px(at.y - 14.0),
                    width: px(120),
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_child(ui::outlined("Versus", 22.0, color::RED_TEXT));
        }
    }
}

/// The program panel: open, it lies over the right of the screen; docked, only
/// a tab at the edge shows. Selecting a unit opens it; the battle docks it.
fn program_overlay(
    body: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    let open = planning.editor_open;
    body.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            bottom: px(0),
            align_items: AlignItems::Center,
            ..row(0.0)
        },
        GlobalZIndex(40),
        // A full-height strip down the right edge: let the pointer through
        // where it's empty, or it swallows clicks on the buttons beneath.
        Pickable::IGNORE,
    ))
    .with_children(|o| {
        o.spawn(ui::button(
            UiAction::ToggleEditor,
            SECONDARY,
            Node { width: px(34), height: px(110), flex_shrink: 0.0, ..column(4.0) },
        ))
        .insert(ui::Tooltip {
            title: if open { "Dock the program (Tab)" } else { "Open the program (Tab)" }
                .to_string(),
            accent: color::GOLD_LIGHT,
            lines: vec![],
        })
        .with_children(|tab| {
            tab.spawn(heading(if open { ">" } else { "<" }, 18.0, color::TEXT));
            tab.spawn(kit::icon(kit::img::ICON_STAFF, 20.0));
        });
        if open {
            o.spawn(Node { width: px(420), height: percent(100), ..default() })
                .with_children(|p| editor::spawn_program_panel(p, run, planning, sprites));
        }
    });
}

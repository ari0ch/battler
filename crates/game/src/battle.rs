//! Battle playback. The sim steps one tick at a time; this module turns each
//! tick's events into animation: units glide between cells, a badge shows the
//! card that fired, and floating numbers show damage and healing. The same
//! view plays replays from the run summary.

use std::collections::HashMap;

use battler_sim::run::economy::{BOARD_DEPTH, BOARD_LANES};
use battler_sim::run::{BattleSetup, Opponent};
use battler_sim::{Battle, CardRef, Class, Event, Outcome, Pos, StatusKind, Team, UnitId};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::cards;
use crate::kit;
use crate::planning::spawn_pips;
use crate::sprites::{SPRITE_PX, Sprites, battle_scale, sprite_node};
use crate::state::{AppState, GameRun, LastReport, LobbyExt, Planning, Rev, RunPhase, Settings};
use crate::ui::{
    self, SECONDARY, TOGGLED, UiAction, color, column, heading, row, spawn_button, text,
};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        let watching = in_state(RunPhase::Battle).or_else(in_state(AppState::Replay));
        app.add_systems(OnEnter(RunPhase::Battle), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(OnEnter(AppState::Replay), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(
                Update,
                (rebuild, advance, animate_units, animate_floaters, refresh_trace)
                    .chain()
                    .run_if(watching),
            );
    }
}

// Card trace inks, chosen to read on parchment.
const TRACE_BLUE: Color = Color::srgb(0.173, 0.373, 0.604);
const TRACE_RED: Color = Color::srgb(0.557, 0.165, 0.165);
const TRACE_FIRED: Color = Color::srgb(0.541, 0.290, 0.0);

/// Real seconds per tick at 1x speed.
const TICK_SECONDS: f32 = 0.5;
/// Pause on the final state before moving on.
const END_PAUSE: f32 = 1.2;
/// Distance between cell origins; cells are `CELL` wide with a 4 px gap.
const PITCH: f32 = 72.0;
const CELL: f32 = 68.0;
const BOARD_W: f32 = PITCH * BOARD_LANES as f32 - 4.0;
const BOARD_H: f32 = PITCH * BOARD_DEPTH as f32 - 4.0;

/// The battle being watched.
#[derive(Resource)]
pub struct Playback {
    pub setup: BattleSetup,
    pub battle: Battle,
    pub replay: bool,
    /// Who the human is fighting, for labels.
    pub opponent: Option<Opponent>,
    /// Seconds into the current tick.
    timer: f32,
    /// Where each unit was before the latest tick, to glide from.
    prev: HashMap<UnitId, Pos>,
    /// Which card each unit ran on the latest tick.
    fired: HashMap<UnitId, CardRef>,
    /// Badge text per unit and how long it stays up.
    badges: HashMap<UnitId, (String, f32)>,
    ended_for: f32,
    done: bool,
    pub skip: bool,
    trace_dirty: bool,
}

impl Playback {
    pub fn new(setup: BattleSetup, replay: bool, opponent: Option<Opponent>) -> Self {
        let battle = setup.build();
        let prev = battle.units().iter().map(|u| (u.id, u.pos)).collect();
        Self {
            setup,
            battle,
            replay,
            opponent,
            timer: 0.0,
            prev,
            fired: HashMap::new(),
            badges: HashMap::new(),
            ended_for: 0.0,
            done: false,
            skip: false,
            trace_dirty: true,
        }
    }
}

#[derive(Component)]
struct BattleRoot;
#[derive(Component)]
struct BoardLayer;
#[derive(Component)]
struct UnitView(UnitId);
#[derive(Component)]
struct HpFill(UnitId);
#[derive(Component)]
struct StatusLetters(UnitId);
#[derive(Component)]
struct Badge(UnitId);
#[derive(Component)]
struct TickText;
#[derive(Component)]
struct TracePanel;
#[derive(Component)]
struct TraceContent;

#[derive(Component)]
struct Floater {
    age: f32,
    base_top: f32,
    color: Color,
}

// The board is drawn rotated from sim coordinates: each lane (`y`) is a screen
// column, and depth (`x`) runs up the screen, so the player's back row is at
// the bottom and the enemy's back row at the top.

fn cell_left(p: Pos) -> f32 {
    p.y as f32 * PITCH
}

fn cell_top(p: Pos) -> f32 {
    (BOARD_DEPTH - 1 - p.x) as f32 * PITCH
}

#[allow(clippy::too_many_arguments)]
fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    pb: Res<Playback>,
    run: Option<Res<GameRun>>,
    settings: Res<Settings>,
    sprites: Res<Sprites>,
    state: Res<State<AppState>>,
    roots: Query<Entity, With<BattleRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let stage = if *state.get() == AppState::Replay {
        ui::spawn_stage(&mut commands, (BattleRoot, DespawnOnExit(AppState::Replay)), color::BG)
    } else {
        ui::spawn_stage(&mut commands, (BattleRoot, DespawnOnExit(RunPhase::Battle)), color::BG)
    };
    let battle = &pb.battle;
    let enemy_name = match (&run, pb.opponent) {
        (Some(run), Some(o)) => cards::opponent_label(&run.0, o),
        _ => "Enemy warband".to_string(),
    };
    commands.entity(stage).with_children(|stage| {
        // Header.
        stage
            .spawn((
                Node { height: px(64), flex_shrink: 0.0, padding: UiRect::new(px(16), px(16), px(0), px(6)), ..row(16.0) },
                kit::tiled(kit::img::HEADER_WOOD, true, false),
            ))
            .with_children(|h| {
                h.spawn((
                    Node { width: px(250), height: px(46), flex_shrink: 0.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..column(0.0) },
                    kit::sliced(kit::img::BANNER, 6.0),
                ))
                .with_children(|c| {
                    let title = if pb.replay {
                        format!("REPLAY - ROUND {}", pb.setup.round)
                    } else {
                        format!("ROUND {}", pb.setup.round)
                    };
                    c.spawn(heading(title, 18.0, color::TEXT));
                    c.spawn(text(format!("vs {enemy_name}"), 11.0, color::TEXT));
                });
                h.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, ..row(8.0) }).with_children(|c| {
                    c.spawn((heading(format!("Tick {}", battle.tick()), 16.0, color::MUTED), TickText, Node { width: px(80), ..default() }));
                    for s in [1, 2, 4] {
                        let style = if settings.speed == s { TOGGLED } else { SECONDARY };
                        spawn_button(c, &format!("{s}x"), UiAction::Speed(s), style, px(52), px(44));
                    }
                    let skip = if pb.replay { "Back to summary" } else { "Skip to result" };
                    let action = if pb.replay { UiAction::BackToSummary } else { UiAction::Skip };
                    spawn_button(c, skip, action, SECONDARY, px(160), px(44));
                    let trace = if settings.card_trace { "Card trace: on" } else { "Card trace: off" };
                    let style = if settings.card_trace { TOGGLED } else { SECONDARY };
                    spawn_button(c, trace, UiAction::ToggleTrace, style, px(150), px(44));
                });
                h.spawn(Node { width: px(200), justify_content: JustifyContent::FlexEnd, ..row(22.0) }).with_children(|r| {
                    if let Some(run) = &run {
                        r.spawn(kit::icon(kit::img::ICON_HEART, 28.0));
                        r.spawn(ui::outlined(run.0.me().hp().to_string(), 20.0, color::TEXT));
                        r.spawn(kit::icon(kit::img::ICON_COIN, 28.0));
                        r.spawn(ui::outlined(run.0.me().gold().to_string(), 20.0, color::GOLD_LIGHT));
                    }
                });
            });

        // Legend and synergies, the board, and the card trace.
        stage
            .spawn(Node { flex_grow: 1.0, padding: UiRect::axes(px(24), px(10)), min_height: px(0), ..row(24.0) })
            .with_children(|body| {
                body.spawn(ui::panel(Node { width: px(250), flex_shrink: 0.0, align_self: AlignSelf::FlexStart, padding: UiRect::all(px(18)), ..column(14.0) }))
                    .with_children(|side| {
                        synergy_strip(side, battle, Team::Enemy, "Enemy synergies", color::RED_TEXT);
                        synergy_strip(side, battle, Team::Player, "Your synergies", color::BLUE_TEXT);
                        side.spawn(text(
                            "Badges: the card each unit ran. Letters: S shield, G guard, I immune, F frenzy, T taunted, Z frozen, M marked, V vanished.",
                            12.0,
                            color::MUTED,
                        ));
                        side.spawn(text("Blue shadows: your warband. Red: the enemy.", 12.0, color::MUTED));
                    });
                body.spawn(Node { flex_grow: 1.0, justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..column(6.0) })
                    .with_children(|mid| {
                        mid.spawn(Node { width: px(BOARD_W), ..row(0.0) })
                            .with_child(heading(enemy_name.clone(), 14.0, color::RED_TEXT));
                        // One open grass field: no tiles. A red tint marks the enemy
                        // half and a dashed line the middle.
                        mid.spawn((
                            Node { width: px(BOARD_W), height: px(BOARD_H), overflow: Overflow::clip(), ..default() },
                            kit::tiled(kit::img::FIELD_GRASS, true, true),
                            Outline { width: px(4), offset: px(0), color: color::LINE_STRONG },
                            BoardLayer,
                        ))
                            .with_children(|board| {
                                board.spawn((
                                    Node { position_type: PositionType::Absolute, left: px(0), right: px(0), top: px(0), height: px(BOARD_H / 2.0), ..default() },
                                    BackgroundColor(Color::srgba(0.235, 0.078, 0.118, 0.28)),
                                    Pickable::IGNORE,
                                ));
                                let mid_y = BOARD_H / 2.0 - 1.0;
                                let mut x = 4.0;
                                while x < BOARD_W {
                                    board.spawn((
                                        Node { position_type: PositionType::Absolute, left: px(x), top: px(mid_y), width: px(10), height: px(2), ..default() },
                                        BackgroundColor(Color::srgba(0.953, 0.902, 0.769, 0.35)),
                                        Pickable::IGNORE,
                                    ));
                                    x += 20.0;
                                }
                                for unit in battle.units() {
                                    spawn_unit(board, unit, &sprites);
                                }
                            });
                        mid.spawn(Node { width: px(BOARD_W), ..row(0.0) })
                            .with_child(heading("Your warband", 14.0, color::BLUE_TEXT));
                    });
                if settings.card_trace {
                    body.spawn((
                        kit::parchment_panel(Node { width: px(310), flex_shrink: 0.0, padding: UiRect::all(px(18)), overflow: Overflow::clip(), ..column(8.0) }),
                        TracePanel,
                    ));
                }
            });
    });
}

/// A unit standing on the open field: a team-coloured shadow at its feet, the
/// sprite (bosses bigger), a health bar over its head, and level pips and
/// status letters at its feet. The outer node is the unit's cell; the sprite
/// may overhang it.
fn spawn_unit(board: &mut ChildSpawnerCommands, unit: &battler_sim::Unit, sprites: &Sprites) {
    let player = unit.team == Team::Player;
    let shadow = if player {
        Color::srgba(0.314, 0.588, 1.0, 0.55)
    } else {
        Color::srgba(0.878, 0.282, 0.282, 0.55)
    };
    let size = SPRITE_PX * battle_scale(unit.kind);
    let fill = if player { kit::img::FILL_HP } else { kit::img::FILL_ENEMY };
    board
        .spawn((
            UnitView(unit.id),
            Node {
                position_type: PositionType::Absolute,
                left: px(cell_left(unit.pos)),
                top: px(cell_top(unit.pos)),
                width: px(CELL),
                height: px(CELL),
                ..default()
            },
            ZIndex(1),
            Pickable::IGNORE,
        ))
        .with_children(|v| {
            // The shadow: an ellipse at the feet.
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px((CELL - size * 0.8) / 2.0),
                    bottom: px(0),
                    width: px(size * 0.8),
                    height: px((size / 5.0).max(12.0)),
                    border_radius: BorderRadius::all(percent(50)),
                    ..default()
                },
                BackgroundColor(shadow),
                Pickable::IGNORE,
            ));
            // The sprite, feet on the cell's bottom edge; enemies face left.
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px((CELL - size) / 2.0),
                    bottom: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_child(sprite_node(
                sprites.unit(unit.kind),
                battle_scale(unit.kind),
                !player,
            ));
            // The health bar over its head.
            let bar_w = (size - 12.0).max(52.0);
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px((CELL - bar_w) / 2.0),
                    bottom: px(size + 4.0),
                    width: px(bar_w),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|b| {
                kit::spawn_bar(b, fill, 100.0, percent(100), 12.0, HpFill(unit.id));
            });
            // The badge naming the card that just ran: a purple tag over the bar,
            // wider than the cell so long names fit.
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(-24),
                    right: px(-24),
                    bottom: px(size + 18.0),
                    height: px(26),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                kit::sliced(kit::img::TAG_SIGNATURE, 4.0),
                Visibility::Hidden,
                Badge(unit.id),
                ZIndex(3),
            ))
            .with_child(heading("", 12.0, color::SIG_TEXT));
            // Pips and status letters at the feet.
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px(4),
                    right: px(4),
                    bottom: px(-6),
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|foot| {
                // Monsters don't level up, so they show no pips.
                if unit.kind.character().is_some() {
                    spawn_pips(foot, unit.level);
                } else {
                    foot.spawn(Node::default());
                }
                foot.spawn((ui::outlined("", 12.0, color::BLUE_TEXT), StatusLetters(unit.id)));
            });
        });
}

#[allow(clippy::too_many_arguments)]
fn advance(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    mut pb: ResMut<Playback>,
    mut run: Option<ResMut<GameRun>>,
    mut next_app: ResMut<NextState<AppState>>,
    next_phase: Option<ResMut<NextState<RunPhase>>>,
    board: Query<Entity, With<BoardLayer>>,
    mut tick_text: Query<&mut Text, With<TickText>>,
    mut planning: ResMut<Planning>,
) {
    if pb.done {
        return;
    }
    let dt = time.delta_secs() * settings.speed as f32;
    for (_, left) in pb.badges.values_mut() {
        *left -= dt;
    }

    if pb.skip {
        pb.battle.run_to_end();
        pb.ended_for = END_PAUSE;
    }

    if pb.battle.outcome() == Outcome::Ongoing {
        pb.timer += dt;
        if pb.timer >= TICK_SECONDS {
            pb.timer = 0.0;
            let pb = &mut *pb;
            pb.prev = pb.battle.units().iter().map(|u| (u.id, u.pos)).collect();
            let events = pb.battle.step();
            pb.fired.clear();
            if let Ok(board) = board.single() {
                for event in &events {
                    show_event(&mut commands, board, pb, event);
                }
            }
            pb.trace_dirty = true;
        }
    } else {
        pb.ended_for += time.delta_secs();
    }
    for mut t in &mut tick_text {
        **t = format!("Tick {}", pb.battle.tick());
    }

    if pb.battle.outcome() != Outcome::Ongoing && pb.ended_for >= END_PAUSE {
        pb.done = true;
        if pb.replay {
            next_app.set(AppState::GameOver);
        } else if let Some(run) = run.as_mut() {
            // The lobby re-plays every battle of the round from its setup, so
            // this result matches the one just watched.
            run.0.resolve();
            let report = run.0.me().history().last().expect("just played").1.clone();
            planning.level_up(&report.level_ups);
            commands.insert_resource(LastReport(report));
            if let Some(mut next) = next_phase {
                next.set(RunPhase::Result);
            }
        }
    }
}

/// Turns one sim event into a badge or floating number.
fn show_event(commands: &mut Commands, board: Entity, pb: &mut Playback, event: &Event) {
    let pos_of = |id: UnitId, pb: &Playback| pb.battle.unit(id).map(|u| u.pos);
    match *event {
        Event::CardFired { unit, card } => {
            pb.fired.insert(unit, card);
            let label = match card {
                CardRef::Node(n) => pb
                    .battle
                    .unit(unit)
                    .and_then(|u| {
                        u.program.nodes().get(n).map(|node| cards::action_name(node.action))
                    })
                    .unwrap_or_default(),
                CardRef::Default => "Attack".to_string(),
            };
            pb.badges.insert(unit, (label, TICK_SECONDS * 1.6));
        }
        Event::Attacked { target, damage, crit, .. } => {
            if let Some(p) = pos_of(target, pb) {
                let (label, c, size) = if damage == 0 {
                    ("Blocked".to_string(), color::BLUE_TEXT, 14.0)
                } else if crit {
                    (format!("{damage}!"), color::CRIT, 22.0)
                } else {
                    (format!("-{damage}"), color::BAD, 16.0)
                };
                spawn_floater(commands, board, p, label, c, size);
            }
        }
        Event::Healed { target, amount, .. } => {
            if let Some(p) = pos_of(target, pb) {
                spawn_floater(commands, board, p, format!("+{amount}"), color::GOOD, 16.0);
            }
        }
        Event::StatusApplied { unit, status: StatusKind::Taunted { .. }, .. } => {
            if let Some(p) = pos_of(unit, pb) {
                spawn_floater(commands, board, p, "Taunted".into(), color::BAD, 12.0);
            }
        }
        Event::SignalSent { unit, .. } => {
            if let Some(p) = pos_of(unit, pb) {
                spawn_floater(commands, board, p, "Signal".into(), color::GOLD_LIGHT, 12.0);
            }
        }
        Event::TurnSkipped { unit } => {
            if let Some(p) = pos_of(unit, pb) {
                spawn_floater(commands, board, p, "Frozen".into(), color::BLUE_TEXT, 13.0);
            }
        }
        Event::Revived { unit, .. } => {
            if let Some(p) = pos_of(unit, pb) {
                spawn_floater(commands, board, p, "Revived!".into(), color::GOLD_LIGHT, 15.0);
            }
        }
        Event::CheatedDeath { unit } => {
            if let Some(p) = pos_of(unit, pb) {
                spawn_floater(commands, board, p, "Undying!".into(), color::GOLD_LIGHT, 15.0);
            }
        }
        _ => {}
    }
}

fn spawn_floater(
    commands: &mut Commands,
    board: Entity,
    p: Pos,
    label: String,
    c: Color,
    size: f32,
) {
    let top = cell_top(p) + 18.0;
    commands.entity(board).with_children(|b| {
        b.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(cell_left(p) + 44.0),
                top: px(top),
                ..default()
            },
            ZIndex(5),
            Floater { age: 0.0, base_top: top, color: c },
            Pickable::IGNORE,
        ))
        .with_child(ui::outlined(label, size, c));
    });
}

fn animate_floaters(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    mut floaters: Query<(Entity, &mut Floater, &mut Node, &Children)>,
    mut colors: Query<&mut TextColor>,
) {
    let dt = time.delta_secs() * (settings.speed as f32).sqrt();
    for (e, mut f, mut node, children) in &mut floaters {
        f.age += dt;
        if f.age > 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        node.top = px(f.base_top - 34.0 * f.age);
        for child in <[Entity]>::iter(children) {
            if let Ok(mut c) = colors.get_mut(*child) {
                c.0 = f.color.with_alpha(1.0 - f.age * f.age);
            }
        }
    }
}

#[allow(clippy::type_complexity)]
fn animate_units(
    pb: Res<Playback>,
    mut views: Query<(&UnitView, &mut Node), (Without<HpFill>, Without<Badge>)>,
    mut bars: Query<(&HpFill, &mut Node, &mut ImageNode), (Without<UnitView>, Without<Badge>)>,
    mut badges: Query<(&Badge, &mut Visibility, &Children), (Without<UnitView>, Without<HpFill>)>,
    mut letters: Query<(&StatusLetters, &mut Text)>,
    mut badge_text: Query<&mut Text, Without<StatusLetters>>,
) {
    let battle = &pb.battle;
    // Glide over the first 60% of a tick, easing out.
    let t = (pb.timer / (TICK_SECONDS * 0.6)).clamp(0.0, 1.0);
    let t = 1.0 - (1.0 - t) * (1.0 - t);
    let tick = battle.tick();

    for (view, mut node) in &mut views {
        let Some(unit) = battle.unit(view.0) else { continue };
        if !unit.is_alive() {
            node.display = Display::None;
            continue;
        }
        // Resurrection can bring a unit back.
        node.display = Display::Flex;
        let from = pb.prev.get(&unit.id).copied().unwrap_or(unit.pos);
        let lerp = |a: f32, b: f32| a + (b - a) * t;
        node.left = px(lerp(cell_left(from), cell_left(unit.pos)));
        node.top = px(lerp(cell_top(from), cell_top(unit.pos)));
    }
    for (bar, mut node, mut image) in &mut bars {
        if let Some(unit) = battle.unit(bar.0) {
            let pct = unit.hp as f32 * 100.0 / unit.stats.max_hp as f32;
            node.width = percent(pct);
            // Your units' bars turn red when low; enemy bars are always red.
            if unit.team == Team::Player {
                let fill = if pct < 35.0 { kit::img::FILL_HP_LOW } else { kit::img::FILL_HP };
                if image.image != fill {
                    image.image = fill;
                }
            }
        }
    }
    for (l, mut text) in &mut letters {
        if let Some(unit) = battle.unit(l.0) {
            let s = &unit.statuses;
            let mut out = String::new();
            if s.guarding {
                out.push('G');
            }
            if s.shielded_until >= tick && tick > 0 {
                out.push('S');
            }
            if s.immune_until >= tick && tick > 0 {
                out.push('I');
            }
            if s.frenzied_until >= tick && tick > 0 {
                out.push('F');
            }
            if s.taunted_by.is_some_and(|(_, until)| until >= tick) {
                out.push('T');
            }
            if s.frozen_turns > 0 {
                out.push('Z');
            }
            if s.hunters_marked {
                out.push('M');
            }
            if s.untargetable_until >= tick && tick > 0 {
                out.push('V');
            }
            if **text != out {
                **text = out;
            }
        }
    }
    for (badge, mut vis, children) in &mut badges {
        match pb.badges.get(&badge.0) {
            Some((label, left)) if *left > 0.0 => {
                *vis = Visibility::Inherited;
                for child in <[Entity]>::iter(children) {
                    if let Ok(mut t) = badge_text.get_mut(*child)
                        && **t != *label
                    {
                        **t = label.clone();
                    }
                }
            }
            _ => *vis = Visibility::Hidden,
        }
    }
}

/// Rewrites the card trace after each tick: every unit's cards, with the one
/// that fired highlighted and the ones above it (which fell through) dimmed.
fn refresh_trace(
    mut commands: Commands,
    mut pb: ResMut<Playback>,
    rev: Res<Rev>,
    panels: Query<Entity, With<TracePanel>>,
    contents: Query<Entity, With<TraceContent>>,
) {
    if !pb.trace_dirty && !rev.is_changed() {
        return;
    }
    pb.trace_dirty = false;
    let Ok(panel) = panels.single() else { return };
    for e in &contents {
        commands.entity(e).despawn();
    }
    let pb = &*pb;
    commands.entity(panel).with_children(|p| {
        p.spawn((column(8.0), TraceContent)).with_children(|c| {
            c.spawn(heading(format!("Card trace - tick {}", pb.battle.tick()), 14.0, color::INK));
            for unit in pb.battle.units().iter().filter(|u| u.is_alive()) {
                let head = if unit.team == Team::Player { TRACE_BLUE } else { TRACE_RED };
                c.spawn(column(2.0)).with_children(|u| {
                    let name = match unit.kind.character() {
                        Some(c) => format!("{} L{}", c.short_name(), unit.level.number()),
                        None => unit.kind.short_name().to_string(),
                    };
                    u.spawn(heading(name, 13.0, head));
                    let fired = pb.fired.get(&unit.id).copied();
                    let fired_at = match fired {
                        Some(CardRef::Node(n)) => Some(n),
                        Some(CardRef::Default) => Some(usize::MAX),
                        None => None,
                    };
                    for (i, node) in unit.program.nodes().iter().enumerate() {
                        let (prefix, ink) = match fired_at {
                            Some(f) if f == i => (">", TRACE_FIRED),
                            Some(f) if i < f => ("-", color::INK_FAINT),
                            _ => (" ", color::INK_MUTED),
                        };
                        u.spawn(text(format!("{prefix} {}", cards::card_label(node)), 11.0, ink));
                    }
                    let (prefix, ink) = if fired == Some(CardRef::Default) {
                        (">", TRACE_FIRED)
                    } else {
                        (" ", color::INK_FAINT)
                    };
                    u.spawn(text(format!("{prefix} locked: attack nearest"), 11.0, ink));
                });
            }
        });
    });
}

/// One side's synergies: active ones highlighted.
fn synergy_strip(
    parent: &mut ChildSpawnerCommands,
    battle: &Battle,
    team: Team,
    label: &str,
    ink: Color,
) {
    let syn = battle.synergies(team);
    parent.spawn(column(6.0)).with_children(|side| {
        side.spawn(heading(label, 14.0, ink));
        side.spawn(Node { flex_wrap: FlexWrap::Wrap, row_gap: px(6), ..row(6.0) }).with_children(
            |chips| {
                let mut any = false;
                for class in Class::ALL {
                    let count = syn.count(class);
                    if count == 0 {
                        continue;
                    }
                    any = true;
                    let (fill, line, ink) = if count >= 2 {
                        (color::ACTIVE_FILL, color::ACTIVE_LINE, color::GOLD_LIGHT)
                    } else {
                        (Color::srgba(0.0, 0.0, 0.0, 0.2), Color::NONE, color::MUTED)
                    };
                    chips
                        .spawn((
                            Node {
                                padding: UiRect::axes(px(6), px(3)),
                                border: UiRect::all(px(2)),
                                ..row(4.0)
                            },
                            BackgroundColor(fill),
                            BorderColor::all(line),
                            Hovered::default(),
                            cards::synergy_tooltip(class, &syn),
                        ))
                        .with_children(|chip| {
                            chip.spawn(kit::icon(kit::class_icon(class), 18.0));
                            chip.spawn(heading(format!("{count} {}", class.plural()), 12.0, ink));
                        });
                }
                if !any {
                    chips.spawn(text("none", 12.0, color::FAINT));
                }
            },
        );
    });
}

//! Battle playback. The sim steps one tick at a time; this module turns each
//! tick's events into animation: units glide between cells, a badge shows the
//! card that fired, and floating numbers show damage and healing. The same
//! view plays replays from the run summary.

use std::collections::HashMap;

use battler_sim::run::economy::{BOARD_DEPTH, BOARD_LANES};
use battler_sim::run::{BattleSetup, Opponent};
use battler_sim::{Battle, CardRef, Class, Event, Outcome, Pos, StatusKind, Team, UnitId};
use bevy::prelude::*;

use crate::cards;
use crate::kit;
use crate::motion::{self, Body};
use crate::planning::spawn_pips;
use crate::sprites::{SPRITE_PX, Sprites, battle_scale, sprite_node};
use crate::state::{AppState, GameRun, LobbyExt, Planning, Rev, RunPhase, Settings};
use crate::ui::{
    self, SECONDARY, TOGGLED, UiAction, color, column, heading, row, spawn_button, text,
};

pub struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        let watching = in_state(RunPhase::Battle).or_else(in_state(AppState::Replay));
        app.add_systems(OnEnter(RunPhase::Battle), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(OnEnter(AppState::Replay), |mut rev: ResMut<Rev>| rev.bump())
            // In a run the planning screen hosts the battle (`spawn_battle_field`);
            // replays from the run summary get a screen of their own.
            .add_systems(Update, rebuild_replay.run_if(in_state(AppState::Replay)))
            .add_systems(
                Update,
                (
                    advance,
                    move_bodies,
                    animate_units,
                    animate_projectiles,
                    animate_floaters,
                    refresh_trace,
                )
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
/// One cell of the battlefield on screen. The same field is used while
/// planning (with grid lines) and in battle (open grass).
pub const CELL: f32 = 66.0;
pub const BOARD_W: f32 = CELL * BOARD_LANES as f32;
pub const BOARD_H: f32 = CELL * BOARD_DEPTH as f32;

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
    /// How each unit is drawn: a free position on the field (see `motion.rs`).
    /// Updated every frame by `move_bodies`.
    bodies: HashMap<UnitId, Body>,
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
        let bodies = motion::spawn(&battle, field_centre, |u| {
            SPRITE_PX * battle_scale(u.kind) * BODY_RADIUS
        });
        Self {
            setup,
            battle,
            replay,
            opponent,
            timer: 0.0,
            bodies,
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
/// A unit's sprite image, flipped to face where the unit is going.
#[derive(Component)]
struct UnitSprite(UnitId);
/// An arrow or bolt in flight from where it was loosed to its target.
#[derive(Component)]
struct Projectile {
    from: Vec2,
    target: UnitId,
    age: f32,
}

/// Stacking order on the field: units use 1 plus a quarter of their height on
/// screen (at most about 150), so these always sit above them.
const PROJECTILE_Z: i32 = 400;
const FLOATER_Z: i32 = 500;
/// How long a projectile takes to land, in playback seconds.
const PROJECTILE_SECS: f32 = 0.18;
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

pub fn cell_left(p: Pos) -> f32 {
    p.y as f32 * CELL
}

pub fn cell_top(p: Pos) -> f32 {
    (BOARD_DEPTH - 1 - p.x) as f32 * CELL
}

/// The centre of a cell on the field, where bodies aim (see `motion.rs`).
fn field_centre(p: Pos) -> Vec2 {
    Vec2::new(cell_left(p) + CELL / 2.0, cell_top(p) + CELL / 2.0)
}

/// A body's collision radius as a fraction of its sprite size: about the
/// width of the figure, so neighbours can stand close without overlapping.
const BODY_RADIUS: f32 = 0.34;

/// The battlefield with every unit on it: one open grass field (no tiles),
/// a red tint over the enemy half and a dashed midline. Animation systems find
/// it by its `BoardLayer` marker.
pub fn spawn_battle_field(parent: &mut ChildSpawnerCommands, battle: &Battle, sprites: &Sprites) {
    parent
        .spawn((
            Node {
                width: px(BOARD_W),
                height: px(BOARD_H),
                flex_shrink: 0.0,
                overflow: Overflow::clip(),
                ..default()
            },
            kit::tiled(kit::img::FIELD_GRASS, true, true),
            Outline { width: px(4), offset: px(0), color: color::LINE_STRONG },
            BoardLayer,
        ))
        .with_children(|board| {
            field_backdrop(board);
            for unit in battle.units() {
                spawn_unit(board, unit, sprites);
            }
        });
}

/// The enemy half's tint and the dashed midline, shared with the planning field.
pub fn field_backdrop(board: &mut ChildSpawnerCommands) {
    board.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            right: px(0),
            top: px(0),
            height: px(BOARD_H / 2.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.235, 0.078, 0.118, 0.28)),
        Pickable::IGNORE,
    ));
    let mid_y = BOARD_H / 2.0 - 1.0;
    let mut x = 4.0;
    while x < BOARD_W {
        board.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(x),
                top: px(mid_y),
                width: px(10),
                height: px(2),
                ..default()
            },
            BackgroundColor(Color::srgba(0.953, 0.902, 0.769, 0.35)),
            Pickable::IGNORE,
        ));
        x += 20.0;
    }
}

/// Speed and skip buttons, and the card trace toggle, for the header.
pub fn battle_controls(parent: &mut ChildSpawnerCommands, settings: &Settings, replay: bool) {
    parent.spawn(row(6.0)).with_children(|c| {
        c.spawn((
            heading("Tick 0", 15.0, color::MUTED),
            TickText,
            Node { width: px(70), ..default() },
        ));
        for s in [1, 2, 4] {
            let style = if settings.speed == s { TOGGLED } else { SECONDARY };
            spawn_button(c, &format!("{s}x"), UiAction::Speed(s), style, px(52), px(42));
        }
        let (skip, action) =
            if replay { ("Back", UiAction::BackToSummary) } else { ("Skip", UiAction::Skip) };
        spawn_button(c, skip, action, SECONDARY, px(90), px(42));
        let style = if settings.card_trace { TOGGLED } else { SECONDARY };
        spawn_button(c, "Trace", UiAction::ToggleTrace, style, px(90), px(42));
    });
}

/// The card trace, as a parchment panel over the right side of the screen.
pub fn spawn_trace_panel(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        kit::parchment_panel(Node {
            position_type: PositionType::Absolute,
            right: px(0),
            top: px(0),
            bottom: px(0),
            width: px(320),
            padding: UiRect::all(px(18)),
            overflow: Overflow::clip(),
            ..column(8.0)
        }),
        GlobalZIndex(40),
        TracePanel,
    ));
}

/// A replay from the run summary: just the field, with controls above it.
fn rebuild_replay(
    mut commands: Commands,
    rev: Res<Rev>,
    pb: Res<Playback>,
    settings: Res<Settings>,
    sprites: Res<Sprites>,
    roots: Query<Entity, With<BattleRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let stage =
        ui::spawn_stage(&mut commands, (BattleRoot, DespawnOnExit(AppState::Replay)), color::BG);
    commands.entity(stage).with_children(|stage| {
        stage
            .spawn((
                Node {
                    height: px(56),
                    flex_shrink: 0.0,
                    padding: UiRect::new(px(16), px(16), px(0), px(6)),
                    justify_content: JustifyContent::SpaceBetween,
                    ..row(16.0)
                },
                kit::tiled(kit::img::HEADER_WOOD, true, false),
            ))
            .with_children(|h| {
                h.spawn(heading(format!("Replay - round {}", pb.setup.round), 18.0, color::TEXT));
                battle_controls(h, &settings, true);
            });
        stage
            .spawn(Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            })
            .with_children(|body| spawn_battle_field(body, &pb.battle, &sprites));
        if settings.card_trace {
            stage
                .spawn(Node {
                    position_type: PositionType::Absolute,
                    right: px(0),
                    top: px(56),
                    bottom: px(0),
                    width: px(320),
                    ..default()
                })
                .with_children(spawn_trace_panel);
        }
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
            .with_child((
                sprite_node(sprites.unit(unit.kind), battle_scale(unit.kind), !player),
                UnitSprite(unit.id),
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
            planning.show_result(report);
            // Straight back to planning: the next timer starts now, with the
            // result as a pop-up over it. Knocked out, or the lobby is over:
            // the run summary.
            if run.0.me().is_alive() && !run.0.is_over() {
                if let Some(mut next) = next_phase {
                    next.set(RunPhase::Planning);
                }
            } else {
                next_app.set(AppState::GameOver);
            }
        }
    }
}

/// Turns one sim event into a badge or floating number.
fn show_event(commands: &mut Commands, board: Entity, pb: &mut Playback, event: &Event) {
    // Floaters appear where the unit is drawn, not at its cell.
    let pos_of = |id: UnitId, pb: &Playback| pb.bodies.get(&id).map(|b| b.pos);
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
        Event::Attacked { attacker, target, damage, crit } => {
            // Melee attackers lunge at their target; ranged ones loose a
            // projectile. Either way the target is knocked back a little.
            let ranged = pb.battle.unit(attacker).is_some_and(|u| u.is_ranged());
            match (ranged, pos_of(attacker, pb)) {
                (true, Some(from)) => {
                    let mage = pb.battle.unit(attacker).is_some_and(|u| u.class == Class::Mage);
                    spawn_projectile(commands, board, from, target, mage);
                }
                _ => motion::lunge(&mut pb.bodies, attacker, target),
            }
            motion::knock(&mut pb.bodies, target, attacker, if crit { 160.0 } else { 90.0 });
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

/// Steps every unit's body (see `motion.rs`) by this frame's playback time.
fn move_bodies(time: Res<Time<Real>>, settings: Res<Settings>, mut pb: ResMut<Playback>) {
    // Long frames (a background tab) are capped so bodies don't fly apart.
    let dt = (time.delta_secs() * settings.speed as f32).min(0.1);
    // `ResMut` derefs to the whole `Playback`; reborrowing it as a plain
    // `&mut Playback` lets us borrow two of its fields at once (one mutably),
    // which a borrow through the smart pointer would not allow.
    let pb = &mut *pb;
    motion::step(&mut pb.bodies, &pb.battle, field_centre, Vec2::new(BOARD_W, BOARD_H), dt);
}

/// Looses a projectile from `from` towards `target`'s body: a gold arrow, or
/// an orange bolt for mages.
fn spawn_projectile(
    commands: &mut Commands,
    board: Entity,
    from: Vec2,
    target: UnitId,
    mage: bool,
) {
    let from = from - Vec2::new(0.0, 18.0);
    let fill = if mage { Color::srgb(0.910, 0.463, 0.165) } else { color::GOLD_LIGHT };
    commands.entity(board).with_children(|b| {
        b.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(from.x - 4.0),
                top: px(from.y - 4.0),
                width: px(8),
                height: px(8),
                ..default()
            },
            BackgroundColor(fill),
            Outline { width: px(2), offset: px(0), color: color::OUTLINE },
            ZIndex(PROJECTILE_Z),
            Projectile { from, target, age: 0.0 },
            Pickable::IGNORE,
        ));
    });
}

/// Flies projectiles along a shallow arc to their target and removes them on arrival.
fn animate_projectiles(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    pb: Res<Playback>,
    mut q: Query<(Entity, &mut Projectile, &mut Node)>,
) {
    let dt = time.delta_secs() * settings.speed as f32;
    for (e, mut p, mut node) in &mut q {
        p.age += dt;
        let t = (p.age / PROJECTILE_SECS).min(1.0);
        let to = pb.bodies.get(&p.target).map_or(p.from, |b| b.pos - Vec2::new(0.0, 18.0));
        let arc = 4.0 * t * (1.0 - t) * 20.0;
        let at = p.from.lerp(to, t) - Vec2::new(0.0, arc);
        node.left = px(at.x - 4.0);
        node.top = px(at.y - 4.0);
        if t >= 1.0 {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_floater(
    commands: &mut Commands,
    board: Entity,
    p: Vec2,
    label: String,
    c: Color,
    size: f32,
) {
    let top = p.y - 30.0;
    commands.entity(board).with_children(|b| {
        b.spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(p.x + 6.0),
                top: px(top),
                ..default()
            },
            ZIndex(FLOATER_Z),
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
    mut views: Query<(&UnitView, &mut Node, &mut ZIndex), (Without<HpFill>, Without<Badge>)>,
    mut bars: Query<
        (&HpFill, &mut Node, &mut ImageNode),
        (Without<UnitView>, Without<Badge>, Without<UnitSprite>),
    >,
    mut sprites: Query<(&UnitSprite, &mut ImageNode), Without<HpFill>>,
    mut badges: Query<(&Badge, &mut Visibility, &Children), (Without<UnitView>, Without<HpFill>)>,
    mut letters: Query<(&StatusLetters, &mut Text)>,
    mut badge_text: Query<&mut Text, Without<StatusLetters>>,
) {
    let battle = &pb.battle;
    let tick = battle.tick();
    // Snap to the art's pixel grid (2 screen pixels at 2x) so sprites stay crisp.
    let snap = |v: f32| (v / 2.0).round() * 2.0;

    for (view, mut node, mut z) in &mut views {
        let Some(unit) = battle.unit(view.0) else { continue };
        if !unit.is_alive() {
            node.display = Display::None;
            continue;
        }
        // Resurrection can bring a unit back.
        node.display = Display::Flex;
        let Some(body) = pb.bodies.get(&unit.id) else { continue };
        // A little hop with each step while walking.
        let hop =
            if body.vel.length() > 18.0 { (body.stride / 9.0).sin().abs() * 4.0 } else { 0.0 };
        node.left = px(snap(body.pos.x - CELL / 2.0));
        node.top = px(snap(body.pos.y - CELL / 2.0 - hop));
        // Units lower on the field are nearer the viewer, so draw them on top.
        let depth = 1 + (body.pos.y / 4.0) as i32;
        if z.0 != depth {
            z.0 = depth;
        }
    }
    for (s, mut image) in &mut sprites {
        if let Some(body) = pb.bodies.get(&s.0)
            && image.flip_x != body.facing_left
        {
            image.flip_x = body.facing_left;
        }
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

//! Battle playback. The sim steps one tick at a time; this module turns each
//! tick's events into animation: units glide between cells, attacks lunge or
//! fly, spells show their effects (`effects.rs`), and floating numbers show
//! damage and healing. The same view plays replays from the run summary.

use std::collections::HashMap;

use battler_sim::run::{BattleSetup, Opponent};
use battler_sim::{Ability, Battle, CardRef, Class, Event, Outcome, StatusKind, Team, UnitId};
use bevy::prelude::*;

use crate::cards;
use crate::effects::{self, Effect};
use crate::field::{self, FIELD_H, FIELD_W, Facing};
use crate::kit;
use crate::motion::{self, Body};
use crate::planning::spawn_pips;
use crate::sprites::{SPRITE_PX, Sprites, battle_scale, sprite_node};
use crate::state::{AppState, GameRun, Planning, Rev, RunPhase, Settings};
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
                    animate_effects,
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

/// Real seconds per tick at 1x speed (shared with the server's round clock).
const TICK_SECONDS: f32 = battler_sim::run::timing::TICK_MS as f32 / 1000.0;
/// Pause on the final state before moving on.
const END_PAUSE: f32 = battler_sim::run::timing::END_PAUSE_MS as f32 / 1000.0;
/// Roughly a cell's width on screen, for sizing spell effects.
const CELL: f32 = 72.0;
/// A unit's node: its feet are at the bottom centre; sprite and bars overhang.
const UNIT_W: f32 = 96.0;
const UNIT_H: f32 = 66.0;

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
    /// Where each body's feet are drawn on the isometric field (projected from
    /// `bodies` every frame).
    screen: HashMap<UnitId, Vec2>,
    /// Which card each unit ran on the latest tick.
    fired: HashMap<UnitId, CardRef>,
    /// The signature ability each unit's fired card uses this tick, so its
    /// hits can look like that spell (a Fireball explodes, a Backstab bleeds).
    casting: HashMap<UnitId, Ability>,
    /// Per-unit animation timers: hit flashes, cast hops, deaths.
    anims: HashMap<UnitId, UnitAnim>,
    /// Playback seconds since the battle began; drives idle breathing.
    clock: f32,
    /// Damage dealt and healing done by each unit so far (the meters).
    pub damage: HashMap<UnitId, u32>,
    pub healing: HashMap<UnitId, u32>,
    /// The totals changed since the meters were last drawn.
    pub meters_dirty: bool,
    /// Where the last enemy fell: monster battles leave their loot there.
    last_enemy_fall: Option<Vec2>,
    loot_shown: bool,
    ended_for: f32,
    done: bool,
    pub skip: bool,
    trace_dirty: bool,
}

impl Playback {
    pub fn new(setup: BattleSetup, replay: bool, opponent: Option<Opponent>) -> Self {
        let battle = setup.build();
        let bodies = motion::spawn(&battle, field::ground_centre, |u| {
            SPRITE_PX * battle_scale(u.kind) * BODY_RADIUS
        });
        let screen = project(&bodies);
        Self {
            setup,
            battle,
            replay,
            opponent,
            timer: 0.0,
            bodies,
            screen,
            fired: HashMap::new(),
            casting: HashMap::new(),
            anims: HashMap::new(),
            clock: 0.0,
            damage: HashMap::new(),
            healing: HashMap::new(),
            meters_dirty: true,
            last_enemy_fall: None,
            loot_shown: false,
            ended_for: 0.0,
            done: false,
            skip: false,
            trace_dirty: true,
        }
    }
}

/// Short-lived animation states of one unit, in playback seconds.
#[derive(Debug, Default, Clone, Copy)]
struct UnitAnim {
    /// Time left on the red flash after being hit.
    flash: f32,
    /// Time left on the hop after casting an ability.
    cast: f32,
    /// Time since the unit died: it topples and fades rather than vanishing.
    dead_for: Option<f32>,
}

const FLASH_SECS: f32 = 0.22;
const CAST_SECS: f32 = 0.35;
const DEATH_SECS: f32 = 0.7;

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

/// Projects every body from the ground plane onto the isometric field.
fn project(bodies: &HashMap<UnitId, Body>) -> HashMap<UnitId, Vec2> {
    bodies.iter().map(|(id, b)| (*id, field::ground_to_screen(b.pos))).collect()
}

/// A body's collision radius as a fraction of its sprite size: about the
/// width of the figure, so neighbours can stand close without overlapping.
const BODY_RADIUS: f32 = 0.34;

/// The battlefield with every unit on it: the isometric board (open grass,
/// no grid) with the units standing on it. Animation systems find it by its
/// `BoardLayer` marker.
pub fn spawn_battle_field(parent: &mut ChildSpawnerCommands, battle: &Battle, sprites: &Sprites) {
    parent
        .spawn((
            Node { width: px(FIELD_W), height: px(FIELD_H), flex_shrink: 0.0, ..default() },
            BoardLayer,
            Pickable::IGNORE,
        ))
        .with_children(|board| {
            field::spawn_board(board, false);
            for unit in battle.units() {
                spawn_unit(board, unit, sprites);
            }
        });
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
/// status letters at its feet. The outer node's bottom centre is the unit's
/// feet; the sprite and bars may overhang it.
fn spawn_unit(board: &mut ChildSpawnerCommands, unit: &battler_sim::Unit, sprites: &Sprites) {
    let player = unit.team == Team::Player;
    let shadow = if player {
        Color::srgba(0.314, 0.588, 1.0, 0.55)
    } else {
        Color::srgba(0.878, 0.282, 0.282, 0.55)
    };
    let size = SPRITE_PX * battle_scale(unit.kind);
    let fill = if player { kit::img::FILL_HP } else { kit::img::FILL_ENEMY };
    let feet = field::cell_centre(unit.pos);
    let facing = Facing::from_ground(motion::team_facing(unit.team));
    let (image, flip) = sprites.facing(unit.kind, facing);
    board
        .spawn((
            UnitView(unit.id),
            field::standing(feet, UNIT_W, UNIT_H),
            ZIndex(field::depth_z(feet.y)),
            Pickable::IGNORE,
        ))
        .with_children(|v| {
            // The shadow: an ellipse at the feet.
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px((UNIT_W - size * 0.8) / 2.0),
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
                    left: px((UNIT_W - size) / 2.0),
                    bottom: px(4),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_child((sprite_node(image, battle_scale(unit.kind), flip), UnitSprite(unit.id)));
            // The health bar over its head.
            let bar_w = (size - 12.0).max(52.0);
            v.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: px((UNIT_W - bar_w) / 2.0),
                    bottom: px(size + 4.0),
                    width: px(bar_w),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .with_children(|b| {
                kit::spawn_bar(b, fill, 100.0, percent(100), 12.0, HpFill(unit.id));
            });
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
    mut next_phase: Option<ResMut<NextState<RunPhase>>>,
    board: Query<Entity, With<BoardLayer>>,
    mut tick_text: Query<&mut Text, With<TickText>>,
    mut planning: ResMut<Planning>,
) {
    if pb.done {
        return;
    }
    let dt = time.delta_secs() * settings.speed as f32;

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
            pb.casting.clear();
            if let Ok(board) = board.single() {
                for event in &events {
                    show_event(&mut commands, board, pb, event);
                }
            }
            pb.trace_dirty = true;
            // Monster rounds always drop something: a glint of gold where the
            // last monster fell (the result pop-up lists what it was).
            if pb.battle.outcome() != Outcome::Ongoing
                && !pb.loot_shown
                && matches!(pb.opponent, Some(Opponent::Monsters(_)))
                && let (Some(p), Ok(board)) = (pb.last_enemy_fall, board.single())
            {
                pb.loot_shown = true;
                show_loot(&mut commands, board, p);
            }
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
            if run.is_online() {
                // Online, the server resolves the round once the longest
                // battle in the lobby has finished (see `net.rs`).
                planning.toast("Waiting for the other battles...");
            } else {
                // The lobby re-plays every battle of the round from its setup,
                // so this result matches the one just watched.
                run.lobby.resolve();
                finish_round(&**run, &mut planning, next_phase.as_deref_mut(), &mut next_app);
            }
        }
    }
}

impl Playback {
    /// Stops playback where it is (online, when the server moves on).
    pub fn finish(&mut self) {
        self.done = true;
    }
}

/// Starts watching this player's battle of the round just locked in: returns
/// the playback to insert. Docks the program panel and closes any leftover
/// result pop-up, since the battle plays on the field.
pub fn begin_battle(run: &GameRun, planning: &mut Planning) -> Option<Playback> {
    let (setup, _, opponent) = run.battle_for(run.my_id)?;
    planning.editor_open = false;
    planning.result = None;
    Some(Playback::new(setup.clone(), false, Some(opponent)))
}

/// After a round has resolved: show its result over the next planning phase,
/// or the run summary if this player is out or the lobby is over.
pub fn finish_round(
    run: &GameRun,
    planning: &mut Planning,
    next_phase: Option<&mut NextState<RunPhase>>,
    next_app: &mut NextState<AppState>,
) {
    if let Some((_, report)) = run.me().history().last() {
        planning.level_up(&report.level_ups);
        planning.show_result(report.clone());
    }
    if run.me().is_alive() && !run.is_over() {
        if let Some(next) = next_phase {
            next.set(RunPhase::Planning);
        }
    } else {
        next_app.set(AppState::GameOver);
    }
}

// Spell colours.
const ICE: Color = Color::srgb(0.62, 0.88, 1.0);
const FIRE: Color = Color::srgb(0.98, 0.55, 0.18);
const HOLY: Color = Color::srgb(1.0, 0.86, 0.40);
const HEAL: Color = Color::srgb(0.45, 0.92, 0.45);
const BLOOD: Color = Color::srgb(0.90, 0.22, 0.22);
const ARCANE: Color = Color::srgb(0.70, 0.45, 0.98);
const SHIELD: Color = Color::srgb(0.45, 0.70, 1.0);
const DUST: Color = Color::srgb(0.72, 0.65, 0.55);

/// Where effects aimed "at" a unit should land: the middle of its sprite,
/// which for big bosses is well above its feet.
fn chest(pb: &Playback, id: UnitId) -> Option<Vec2> {
    let feet = *pb.screen.get(&id)?;
    let size = pb.battle.unit(id).map_or(SPRITE_PX * 2.0, |u| SPRITE_PX * battle_scale(u.kind));
    Some(feet - Vec2::new(0.0, 4.0 + size / 2.0))
}

/// Where a unit stands, for effects on the ground around it (novas, auras).
fn feet(pb: &Playback, id: UnitId) -> Option<Vec2> {
    pb.screen.get(&id).copied()
}

/// Turns one sim event into animation: floating numbers, lunges and
/// projectiles, and spell effects (see `effects.rs`).
fn show_event(commands: &mut Commands, board: Entity, pb: &mut Playback, event: &Event) {
    match *event {
        Event::CardFired { unit, card } => {
            pb.fired.insert(unit, card);
            let action = match card {
                CardRef::Node(n) => pb
                    .battle
                    .unit(unit)
                    .and_then(|u| u.program.nodes().get(n).map(|node| node.action)),
                CardRef::Default => None,
            };
            if let Some(ability) = action.and_then(|a| a.ability()) {
                pb.casting.insert(unit, ability);
            }
        }
        Event::Attacked { attacker, target, damage, crit } => {
            show_hit(commands, board, pb, attacker, target, damage, crit);
        }
        Event::Healed { healer, target, amount } => {
            *pb.healing.entry(healer).or_default() += amount;
            pb.meters_dirty = true;
            if let Some(p) = chest(pb, target) {
                spawn_floater(commands, board, p, format!("+{amount}"), color::GOOD, 16.0);
            }
            effects::sparkles(commands, board, target, HEAL);
        }
        Event::Guarded { unit } => {
            if let Some(p) = chest(pb, unit) {
                effects::ring(p, 30.0, 36.0).color(SHIELD).life(0.4).spawn(commands, board);
            }
        }
        Event::StatusApplied { unit, status, .. } => show_status(commands, board, pb, unit, status),
        Event::SignalSent { unit, .. } => {
            if let Some(p) = chest(pb, unit) {
                spawn_floater(commands, board, p, "Signal".into(), color::GOLD_LIGHT, 12.0);
                effects::ring(p, 8.0, CELL * 2.2)
                    .color(color::GOLD_LIGHT)
                    .life(0.6)
                    .spawn(commands, board);
            }
        }
        Event::AbilityUsed { unit, ability } => show_cast(commands, board, pb, unit, ability),
        Event::Teleported { unit, from, to } => {
            // A puff where it vanished and another where it appears.
            let (a, b) = (field::cell_centre(from), field::cell_centre(to));
            let up = Vec2::new(0.0, -30.0);
            effects::spray(commands, board, a + up, 10, 90.0, ARCANE);
            effects::burst(a + up, 26.0).color(ARCANE).life(0.3).spawn(commands, board);
            motion::teleport(&mut pb.bodies, unit, field::ground_centre(to));
            pb.screen.insert(unit, b);
            effects::burst(b + up, 30.0).color(ARCANE).life(0.35).spawn(commands, board);
            effects::spray(commands, board, b + up, 10, 70.0, ARCANE);
        }
        Event::TurnSkipped { unit } => {
            if let Some(p) = chest(pb, unit) {
                spawn_floater(commands, board, p, "Frozen".into(), color::BLUE_TEXT, 13.0);
            }
        }
        Event::Died { unit } => {
            pb.anims.entry(unit).or_default().dead_for = Some(0.0);
            if let Some(p) = feet(pb, unit) {
                effects::spray(commands, board, p, 12, 60.0, DUST);
                if pb.battle.unit(unit).is_some_and(|u| u.team == Team::Enemy) {
                    pb.last_enemy_fall = Some(p);
                }
            }
        }
        Event::Revived { unit, .. } => {
            pb.anims.insert(unit, UnitAnim { cast: CAST_SECS, ..default() });
            if let Some(p) = chest(pb, unit) {
                spawn_floater(commands, board, p, "Revived!".into(), color::GOLD_LIGHT, 15.0);
                effects::burst(p, 44.0).color(HOLY).life(0.5).spawn(commands, board);
            }
            effects::sparkles(commands, board, unit, HOLY);
        }
        Event::CheatedDeath { unit } => {
            if let Some(p) = chest(pb, unit) {
                spawn_floater(commands, board, p, "Undying!".into(), color::GOLD_LIGHT, 15.0);
                effects::ring(p, 50.0, 20.0).color(HOLY).life(0.5).spawn(commands, board);
            }
        }
        Event::Moved { .. } => {}
    }
}

/// A burst of gold and sparkles where loot dropped.
fn show_loot(commands: &mut Commands, board: Entity, at: Vec2) {
    let up = at - Vec2::new(0.0, 14.0);
    effects::burst(up, 26.0).color(HOLY).life(0.6).spawn(commands, board);
    effects::ring(at, 6.0, 40.0).color(HOLY).life(0.7).flat().spawn(commands, board);
    effects::spray(commands, board, up, 12, 70.0, HOLY);
    spawn_floater(commands, board, up, "Loot!".into(), color::GOLD_LIGHT, 16.0);
}

/// An attack landing: a melee lunge or a projectile, a knock-back, a flash on
/// the target and a burst styled by the spell that caused it.
fn show_hit(
    commands: &mut Commands,
    board: Entity,
    pb: &mut Playback,
    attacker: UnitId,
    target: UnitId,
    damage: u32,
    crit: bool,
) {
    let ranged = pb.battle.unit(attacker).is_some_and(|u| u.is_ranged());
    let spell = pb.casting.get(&attacker).copied();
    // Ranged hits land when the projectile does.
    let mut delay = 0.0;
    match (ranged, chest(pb, attacker)) {
        (true, Some(from)) => {
            let mage = pb.battle.unit(attacker).is_some_and(|u| u.class == Class::Mage);
            spawn_projectile(commands, board, from, target, mage);
            delay = PROJECTILE_SECS;
        }
        _ => motion::lunge(&mut pb.bodies, attacker, target),
    }
    motion::knock(&mut pb.bodies, target, attacker, if crit { 160.0 } else { 90.0 });
    if damage > 0 {
        *pb.damage.entry(attacker).or_default() += damage;
        pb.meters_dirty = true;
        pb.anims.entry(target).or_default().flash = FLASH_SECS + delay;
    }
    let Some(p) = chest(pb, target) else { return };
    let (label, c, size) = if damage == 0 {
        ("Blocked".to_string(), color::BLUE_TEXT, 14.0)
    } else if crit {
        (format!("{damage}!"), color::CRIT, 22.0)
    } else {
        (format!("-{damage}"), color::BAD, 16.0)
    };
    spawn_floater(commands, board, p, label, c, size);

    // Spell-specific impacts; anything else gets a small white spark.
    match spell {
        Some(Ability::Fireball) => {
            effects::burst(p, 46.0).color(FIRE).life(0.45).delay(delay).spawn(commands, board);
            effects::ring(p, 10.0, 60.0).color(FIRE).life(0.4).delay(delay).spawn(commands, board);
        }
        Some(Ability::Firebolt) => {
            effects::burst(p, 24.0).color(FIRE).life(0.3).delay(delay).spawn(commands, board);
        }
        Some(Ability::FrostNova) => {
            effects::burst(p, 20.0).color(ICE).life(0.35).spawn(commands, board);
        }
        Some(Ability::Backstab | Ability::Assassinate) => {
            effects::burst(p, 30.0).color(BLOOD).life(0.35).spawn(commands, board);
            effects::spray(commands, board, p, 8, 80.0, BLOOD);
        }
        Some(Ability::AimedShot) => {
            effects::ring(p, 6.0, 30.0).color(HOLY).life(0.3).delay(delay).spawn(commands, board);
        }
        _ if damage > 0 => {
            effects::burst(p, 14.0)
                .color(Color::WHITE)
                .life(0.18)
                .delay(delay)
                .spawn(commands, board);
        }
        _ => {}
    }
    if crit {
        effects::spray(commands, board, p, 8, 120.0, color::CRIT);
    }
}

/// The caster's side of an ability: a hop, and for area spells a ring or
/// nova on the ground around it.
fn show_cast(commands: &mut Commands, board: Entity, pb: &mut Playback, unit: UnitId, a: Ability) {
    pb.anims.entry(unit).or_default().cast = CAST_SECS;
    let (Some(at), Some(ground)) = (chest(pb, unit), feet(pb, unit)) else { return };
    let ring = |from: f32, to: f32, c: Color, life: f32| {
        effects::ring(ground, from, to).color(c).life(life).flat()
    };
    match a {
        Ability::FrostNova => {
            ring(12.0, CELL * 1.6, ICE, 0.5).spawn(commands, board);
            effects::burst(ground, CELL * 0.9).color(ICE).life(0.4).flat().spawn(commands, board);
            effects::spray(commands, board, ground, 16, 170.0, ICE);
        }
        Ability::Cleave => ring(14.0, CELL * 1.2, FIRE, 0.3).spawn(commands, board),
        Ability::Rampage => {
            ring(14.0, CELL * 1.8, FIRE, 0.4).spawn(commands, board);
            effects::spray(commands, board, ground, 12, 150.0, DUST);
        }
        Ability::MassHeal => {
            ring(12.0, CELL * 2.5, HEAL, 0.6).spawn(commands, board);
            effects::burst(at, 36.0).color(HEAL).life(0.4).spawn(commands, board);
        }
        Ability::Sanctuary => {
            ring(12.0, CELL * 2.5, HOLY, 0.6).spawn(commands, board);
            ring(12.0, CELL * 1.6, HOLY, 0.5).delay(0.12).spawn(commands, board);
        }
        Ability::Heal => effects::burst(at, 22.0).color(HEAL).life(0.3).spawn(commands, board),
        Ability::Taunt => {
            ring(10.0, CELL * 1.5, BLOOD, 0.45).spawn(commands, board);
            ring(10.0, CELL * 1.0, BLOOD, 0.35).delay(0.1).spawn(commands, board);
        }
        Ability::ShieldWall => ring(10.0, CELL * 1.4, SHIELD, 0.45).spawn(commands, board),
        Ability::SecondWind => {
            effects::burst(at, 40.0).color(HEAL).life(0.45).spawn(commands, board)
        }
        Ability::Frenzy => {
            effects::ring(at, 40.0, 16.0).color(BLOOD).life(0.35).spawn(commands, board);
            effects::ring(at, 40.0, 16.0)
                .color(BLOOD)
                .life(0.35)
                .delay(0.15)
                .spawn(commands, board);
        }
        Ability::Fireball | Ability::Firebolt => {
            effects::burst(at, 20.0).color(FIRE).life(0.25).spawn(commands, board);
        }
        Ability::AimedShot | Ability::Volley => {
            effects::ring(at, 30.0, 8.0).color(HOLY).life(0.3).spawn(commands, board);
        }
        Ability::Kite => effects::spray(commands, board, ground, 8, 60.0, DUST),
        // These show where they land: puffs on `Teleported`, bursts on the hit.
        Ability::Blink | Ability::Shadowstep | Ability::Backstab | Ability::Assassinate => {}
    }
}

/// Lasting effects get a mark when they start: bubbles for shields, ice for
/// freezing, a red pulse for frenzy.
fn show_status(commands: &mut Commands, board: Entity, pb: &Playback, unit: UnitId, s: StatusKind) {
    let Some(p) = chest(pb, unit) else { return };
    // Bubbles move with the unit, so they are placed relative to its body.
    let off = pb.screen.get(&unit).map_or(Vec2::ZERO, |feet| p - *feet);
    match s {
        StatusKind::Shielded => {
            effects::bubble(off, 34.0).color(SHIELD).life(0.9).follow(unit).spawn(commands, board);
        }
        StatusKind::Immune => {
            effects::bubble(off, 36.0).color(HOLY).life(0.9).follow(unit).spawn(commands, board);
        }
        StatusKind::Frenzied => {
            effects::burst(p, 30.0).color(BLOOD).life(0.3).spawn(commands, board);
        }
        StatusKind::Frozen => {
            effects::burst(p, 30.0).color(ICE).life(0.5).spawn(commands, board);
            effects::spray(commands, board, p, 8, 50.0, ICE);
        }
        StatusKind::Marked => {
            effects::ring(p, 34.0, 18.0).color(BLOOD).life(0.4).spawn(commands, board);
        }
        StatusKind::Taunted { .. } => {
            spawn_floater(commands, board, p, "Taunted".into(), color::BAD, 12.0);
        }
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
    motion::step(&mut pb.bodies, &pb.battle, field::ground_centre, field::ground_size(), dt);
    pb.screen = project(&pb.bodies);
    pb.clock += dt;
    for anim in pb.anims.values_mut() {
        anim.flash = (anim.flash - dt).max(0.0);
        anim.cast = (anim.cast - dt).max(0.0);
        if let Some(t) = anim.dead_for.as_mut() {
            *t += dt;
        }
    }
}

/// Grows, moves and fades spell effects (see `effects.rs`).
#[allow(clippy::type_complexity)]
fn animate_effects(
    mut commands: Commands,
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    pb: Res<Playback>,
    mut q: Query<(Entity, &mut Effect, &mut Node, &mut BackgroundColor, &mut BorderColor)>,
) {
    let dt = (time.delta_secs() * settings.speed as f32).min(0.1);
    effects::animate(&mut commands, dt, &pb.screen, &mut q);
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
        let to = chest(&pb, p.target).unwrap_or(p.from);
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

/// How a unit's sprite is posed this frame, on top of where its body is.
#[derive(Debug, PartialEq)]
struct Pose {
    /// Pixels up (negative) or down from its spot.
    lift: f32,
    /// Clockwise tilt in degrees.
    lean: f32,
    tint: Color,
}

/// Works out a unit's pose from its animation timers. Kept free of ECS types
/// so it is easy to test.
fn unit_pose(
    unit: &battler_sim::Unit,
    body: &Body,
    anim: &UnitAnim,
    clock: f32,
    tick: u32,
) -> Pose {
    // Lean forwards, the way the sprite faces, so art facing left leans left.
    let forward = if Facing::from_ground(body.facing).is_east() { 1.0 } else { -1.0 };
    if let Some(t) = anim.dead_for.filter(|_| !unit.is_alive()) {
        // Topple over sideways in the first third, then fade out.
        let fall = (t / (DEATH_SECS * 0.35)).min(1.0);
        let fade = 1.0 - ((t - DEATH_SECS * 0.35) / (DEATH_SECS * 0.65)).clamp(0.0, 1.0);
        return Pose {
            lift: 8.0 * fall,
            lean: -90.0 * forward * fall * fall,
            tint: Color::srgba(0.75, 0.70, 0.70, fade),
        };
    }
    // Breathing: a 2-pixel bob, out of step between units.
    let phase = unit.id.0 as f32 * 1.7;
    let still = body.vel.length() < 18.0;
    let mut lift = if still && (clock * 2.4 + phase).sin() > 0.4 { -2.0 } else { 0.0 };
    // A hop when casting: up and down once.
    if anim.cast > 0.0 {
        let k = 1.0 - anim.cast / CAST_SECS;
        lift -= (std::f32::consts::PI * k).sin() * 10.0;
    }
    let lean = 14.0 * forward * body.lunge_reach();
    let tint = if anim.flash > 0.0 && anim.flash <= FLASH_SECS {
        Color::srgb(1.0, 0.45, 0.45)
    } else if unit.statuses.frozen_turns > 0 {
        Color::srgb(0.65, 0.85, 1.0)
    } else if unit.statuses.frenzied_until >= tick && tick > 0 {
        Color::srgb(1.0, 0.80, 0.75)
    } else {
        Color::WHITE
    };
    Pose { lift: (lift / 2.0).round() * 2.0, lean, tint }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn animate_units(
    pb: Res<Playback>,
    art: Res<Sprites>,
    mut views: Query<(&UnitView, &mut Node, &mut ZIndex), Without<HpFill>>,
    mut bars: Query<(&HpFill, &mut Node, &mut ImageNode), (Without<UnitView>, Without<UnitSprite>)>,
    mut sprites: Query<(&UnitSprite, &mut ImageNode, &mut UiTransform), Without<HpFill>>,
    mut letters: Query<(&StatusLetters, &mut Text)>,
) {
    let battle = &pb.battle;
    let tick = battle.tick();
    // Snap to the art's pixel grid (2 screen pixels at 2x) so sprites stay crisp.
    let snap = |v: f32| (v / 2.0).round() * 2.0;

    for (view, mut node, mut z) in &mut views {
        let Some(unit) = battle.unit(view.0) else { continue };
        // The dead stay up while they topple, then go.
        let dying = pb.anims.get(&unit.id).and_then(|a| a.dead_for).is_some_and(|t| t < DEATH_SECS);
        if !unit.is_alive() && !dying {
            node.display = Display::None;
            continue;
        }
        // Resurrection can bring a unit back.
        node.display = Display::Flex;
        let (Some(body), Some(feet)) = (pb.bodies.get(&unit.id), pb.screen.get(&unit.id)) else {
            continue;
        };
        // A little hop with each step while walking.
        let hop =
            if body.vel.length() > 18.0 { (body.stride / 9.0).sin().abs() * 4.0 } else { 0.0 };
        node.left = px(snap(feet.x - UNIT_W / 2.0));
        node.top = px(snap(feet.y - UNIT_H - hop));
        // Units lower on the field are nearer the viewer, so draw them on top.
        let depth = field::depth_z(feet.y);
        if z.0 != depth {
            z.0 = depth;
        }
    }
    for (s, mut image, mut transform) in &mut sprites {
        let (Some(body), Some(unit)) = (pb.bodies.get(&s.0), battle.unit(s.0)) else { continue };
        let (handle, flip) = art.facing(unit.kind, Facing::from_ground(body.facing));
        if image.image != handle {
            image.image = handle;
        }
        if image.flip_x != flip {
            image.flip_x = flip;
        }
        let anim = pb.anims.get(&s.0).copied().unwrap_or_default();
        let pose = unit_pose(unit, body, &anim, pb.clock, tick);
        if image.color != pose.tint {
            image.color = pose.tint;
        }
        let t = UiTransform {
            translation: Val2::px(0.0, pose.lift),
            scale: Vec2::ONE,
            rotation: Rot2::degrees(pose.lean),
        };
        if *transform != t {
            *transform = t;
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

#[cfg(test)]
mod tests {
    use super::*;
    use battler_sim::Pos;
    use battler_sim::run::economy::{BOARD_DEPTH, BOARD_LANES};
    use battler_sim::{Character, Level, Program, UnitSpec};

    /// One player unit facing right and one enemy facing left, with bodies.
    fn setup() -> (Battle, HashMap<UnitId, Body>) {
        let mut b = Battle::new(BOARD_DEPTH, BOARD_LANES, 50, 1);
        for (team, x) in [(Team::Player, 3), (Team::Enemy, 4)] {
            let spec = UnitSpec::new(Character::Nettle, Level::One, Program::new(Vec::new()));
            b.spawn(team, Pos::new(x, 3), spec).expect("free cell");
        }
        let bodies = motion::spawn(&b, field::ground_centre, |_| 22.0);
        (b, bodies)
    }

    #[test]
    fn a_resting_unit_only_breathes() {
        let (b, bodies) = setup();
        let u = &b.units()[0];
        for i in 0..20 {
            let pose = unit_pose(u, &bodies[&u.id], &UnitAnim::default(), i as f32 * 0.1, 0);
            assert!(pose.lift == 0.0 || pose.lift == -2.0, "{pose:?}");
            assert_eq!(pose.lean, 0.0);
            assert_eq!(pose.tint, Color::WHITE);
        }
    }

    #[test]
    fn a_hit_flashes_red_and_a_cast_hops() {
        let (b, bodies) = setup();
        let u = &b.units()[0];
        let hit = UnitAnim { flash: FLASH_SECS, ..default() };
        assert_ne!(unit_pose(u, &bodies[&u.id], &hit, 0.0, 0).tint, Color::WHITE);
        let cast = UnitAnim { cast: CAST_SECS / 2.0, ..default() };
        assert!(unit_pose(u, &bodies[&u.id], &cast, 0.0, 0).lift <= -8.0);
    }

    #[test]
    fn the_dead_topple_towards_their_facing_and_fade() {
        let (b, bodies) = setup();
        for u in b.units() {
            let mut dead = u.clone();
            dead.hp = 0;
            let body = &bodies[&u.id];
            let start = UnitAnim { dead_for: Some(0.0), ..default() };
            let end = UnitAnim { dead_for: Some(DEATH_SECS), ..default() };
            assert_eq!(unit_pose(&dead, body, &start, 0.0, 5).lean, 0.0);
            let fallen = unit_pose(&dead, body, &end, 0.0, 5);
            assert_eq!(fallen.lean.abs(), 90.0);
            // Facing left (enemies) falls the other way to facing right.
            let east = Facing::from_ground(body.facing).is_east();
            assert_eq!(fallen.lean > 0.0, !east);
            assert_eq!(fallen.tint.alpha(), 0.0);
        }
    }
}

//! Battler client. For now this just plays a demo battle from `battler-sim`
//! on a loop, to prove the sim -> render pipeline and the web deploy.

use battler_sim::{Battle, Instruction::*, Outcome, Pos, Program, Team, UnitId, UnitStats};
use bevy::prelude::*;

/// Size of one grid cell in world units (pixels at 1x zoom).
const CELL: f32 = 64.0;
/// Real seconds per simulation tick.
const TICK_SECONDS: f32 = 0.35;
/// Ticks to linger on the result before restarting the demo.
const RESTART_DELAY_TICKS: u32 = 6;

const BACKGROUND: Color = Color::srgb(0.08, 0.07, 0.10);
const TILE: Color = Color::srgb(0.16, 0.15, 0.20);
const PLAYER: Color = Color::srgb(0.35, 0.62, 0.95);
const ENEMY: Color = Color::srgb(0.90, 0.36, 0.30);

fn main() {
    App::new()
        .insert_resource(ClearColor(BACKGROUND))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battler".into(),
                // Web only: render into the canvas from index.html and size it to the page.
                canvas: Some("#battler-canvas".into()),
                fit_canvas_to_parent: true,
                prevent_default_event_handling: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(Sim::new())
        .insert_resource(SimClock(Timer::from_seconds(TICK_SECONDS, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(Update, (advance_sim, sync_units, update_status).chain())
        .run();
}

#[derive(Resource)]
struct Sim {
    battle: Battle,
    ticks_since_end: u32,
}

impl Sim {
    fn new() -> Self {
        Self { battle: demo_battle(), ticks_since_end: 0 }
    }
}

#[derive(Resource)]
struct SimClock(Timer);

/// Links a sprite to the sim unit it draws.
#[derive(Component)]
struct UnitSprite(UnitId);

#[derive(Component)]
struct StatusText;

fn demo_battle() -> Battle {
    let knight = UnitStats { max_hp: 10, attack: 3 };
    let goblin = UnitStats { max_hp: 6, attack: 2 };
    let mut b = Battle::new(8, 4, 200);
    b.spawn(Team::Player, Pos::new(0, 1), knight, Program::new([Advance, Attack]));
    b.spawn(Team::Player, Pos::new(0, 2), knight, Program::new([Advance, Guard, Attack]));
    b.spawn(Team::Enemy, Pos::new(7, 0), goblin, Program::new([Advance, Attack]));
    b.spawn(Team::Enemy, Pos::new(7, 1), goblin, Program::new([Advance, Attack]));
    b.spawn(Team::Enemy, Pos::new(7, 3), goblin, Program::new([Advance, Attack, Attack]));
    b
}

/// Converts a grid cell to a world position, centring the grid on the origin.
fn cell_to_world(pos: Pos, battle: &Battle, z: f32) -> Vec3 {
    let half_w = (battle.width() - 1) as f32 / 2.0;
    let half_h = (battle.height() - 1) as f32 / 2.0;
    Vec3::new((pos.x as f32 - half_w) * CELL, (half_h - pos.y as f32) * CELL, z)
}

fn setup(mut commands: Commands, sim: Res<Sim>) {
    commands.spawn(Camera2d);
    let battle = &sim.battle;

    for y in 0..battle.height() {
        for x in 0..battle.width() {
            commands.spawn((
                Sprite::from_color(TILE, Vec2::splat(CELL - 4.0)),
                Transform::from_translation(cell_to_world(Pos::new(x, y), battle, 0.0)),
            ));
        }
    }

    for unit in battle.units() {
        let color = match unit.team {
            Team::Player => PLAYER,
            Team::Enemy => ENEMY,
        };
        commands.spawn((
            UnitSprite(unit.id),
            Sprite::from_color(color, Vec2::splat(CELL * 0.7)),
            Transform::from_translation(cell_to_world(unit.pos, battle, 1.0)),
        ));
    }

    let top = (battle.height() as f32 / 2.0 + 1.0) * CELL;
    commands.spawn((StatusText, Text2d::new(""), Transform::from_xyz(0.0, top, 2.0)));
}

fn advance_sim(time: Res<Time>, mut clock: ResMut<SimClock>, mut sim: ResMut<Sim>) {
    if !clock.0.tick(time.delta()).just_finished() {
        return;
    }
    if sim.battle.outcome() == Outcome::Ongoing {
        sim.battle.step();
    } else {
        sim.ticks_since_end += 1;
        if sim.ticks_since_end >= RESTART_DELAY_TICKS {
            *sim = Sim::new();
        }
    }
}

fn sync_units(sim: Res<Sim>, mut sprites: Query<(&UnitSprite, &mut Transform, &mut Visibility)>) {
    for (UnitSprite(id), mut transform, mut visibility) in &mut sprites {
        let Some(unit) = sim.battle.unit(*id) else { continue };
        if !unit.is_alive() {
            *visibility = Visibility::Hidden;
            continue;
        }
        *visibility = Visibility::Inherited;
        transform.translation = cell_to_world(unit.pos, &sim.battle, 1.0);
        // Cheap health readout until there's real UI: shrink as HP drops.
        let health = unit.hp as f32 / unit.stats.max_hp as f32;
        transform.scale = Vec3::splat(0.5 + 0.5 * health);
    }
}

fn update_status(sim: Res<Sim>, mut status: Single<&mut Text2d, With<StatusText>>) {
    let result = match sim.battle.outcome() {
        Outcome::Ongoing => "fighting".to_string(),
        Outcome::Won(Team::Player) => "knights win".to_string(),
        Outcome::Won(Team::Enemy) => "goblins win".to_string(),
        Outcome::Draw => "draw".to_string(),
    };
    status.0 = format!("Tick {}  ·  {}", sim.battle.tick(), result);
}

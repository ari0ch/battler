//! Battler client. For now this plays a demo battle from `battler-sim` on a
//! loop, using every class and real rule-card programs. Each restart uses a
//! new seed, so crits can change how the fight goes.

use battler_sim::{
    Action::*, Battle, Class, Condition::*, Level, Node, Outcome, Pos, Program, Target, Team,
    Threshold, UnitId, UnitSpec,
};
use bevy::prelude::*;

/// Size of one grid cell in world units (pixels at 1x zoom).
const CELL: f32 = 64.0;
/// Real seconds per simulation tick.
const TICK_SECONDS: f32 = 0.4;
/// Ticks to linger on the result before restarting the demo.
const RESTART_DELAY_TICKS: u32 = 6;

const BACKGROUND: Color = Color::srgb(0.08, 0.07, 0.10);
const TILE: Color = Color::srgb(0.16, 0.15, 0.20);
const PLAYER: Color = Color::srgb(0.30, 0.55, 0.92);
const ENEMY: Color = Color::srgb(0.88, 0.34, 0.28);

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
        .insert_resource(Sim::new(1))
        .insert_resource(SimClock(Timer::from_seconds(TICK_SECONDS, TimerMode::Repeating)))
        .add_systems(Startup, setup)
        .add_systems(Update, (advance_sim, sync_units, update_status).chain())
        .run();
}

#[derive(Resource)]
struct Sim {
    seed: u64,
    battle: Battle,
    ticks_since_end: u32,
}

impl Sim {
    fn new(seed: u64) -> Self {
        Self { seed, battle: demo_battle(seed), ticks_since_end: 0 }
    }
}

#[derive(Resource)]
struct SimClock(Timer);

/// Links a sprite to the sim unit it draws.
#[derive(Component)]
struct UnitSprite(UnitId);

#[derive(Component)]
struct StatusText;

fn demo_battle(seed: u64) -> Battle {
    let card = Node::new;
    let mut b = Battle::new(8, 5, 200, seed);
    let mut add = |team, x, y, class, level, nodes: Vec<Node>| {
        let spec = UnitSpec::new(class, level, Program::new(nodes));
        b.spawn(team, Pos::new(x, y), spec).expect("demo units are valid");
    };

    // Knights, Ranger and Cleric: hold the line and focus the weakest enemy.
    add(
        Team::Player,
        1,
        2,
        Class::Knight,
        Level::Two,
        vec![card(EnemyAdjacent, Taunt), card(AllyHpBelow(Threshold::P50), ShieldWall)],
    );
    add(
        Team::Player,
        0,
        1,
        Class::Ranger,
        Level::Two,
        vec![
            card(EnemyAdjacent, Kite(Target::Nearest)),
            card(EnemyInRange, AimedShot(Target::LowestHp)),
            card(EnemyInRange, Attack(Target::LowestHp)),
        ],
    );
    add(
        Team::Player,
        0,
        3,
        Class::Cleric,
        Level::One,
        vec![
            card(AllyHpBelow(Threshold::P75), Heal(Target::LowestHp)),
            card(MyHpBelow(Threshold::P50), Sanctuary(Target::Nearest)),
        ],
    );

    // Berserker, Mage and Rogue: dive the backline.
    add(
        Team::Enemy,
        6,
        2,
        Class::Berserker,
        Level::Three,
        vec![card(EnemyAdjacent, Frenzy), card(Outnumbered, Guard), card(EnemyAdjacent, Cleave)],
    );
    add(
        Team::Enemy,
        7,
        1,
        Class::Mage,
        Level::Two,
        vec![card(EnemyAdjacent, Blink), card(EnemyInRange, Fireball(Target::Nearest))],
    );
    add(
        Team::Enemy,
        7,
        4,
        Class::Rogue,
        Level::Two,
        vec![
            card(Always, Shadowstep(Target::Ranged)),
            card(EnemyAdjacent, Backstab(Target::Nearest)),
        ],
    );
    b
}

fn class_letter(class: Class) -> &'static str {
    match class {
        Class::Knight => "K",
        Class::Ranger => "R",
        Class::Mage => "M",
        Class::Cleric => "C",
        Class::Rogue => "Ro",
        Class::Berserker => "B",
    }
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
        commands
            .spawn((
                UnitSprite(unit.id),
                Sprite::from_color(color, Vec2::splat(CELL * 0.7)),
                Transform::from_translation(cell_to_world(unit.pos, battle, 1.0)),
            ))
            .with_child((
                Text2d::new(class_letter(unit.class)),
                Transform::from_xyz(0.0, 0.0, 1.0),
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
            let next_seed = sim.seed + 1;
            *sim = Sim::new(next_seed);
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
        Outcome::Ongoing => "fighting",
        Outcome::Won(Team::Player) => "blue wins",
        Outcome::Won(Team::Enemy) => "red wins",
        Outcome::Draw => "draw",
    };
    status.0 = format!("Seed {}  ·  Tick {}  ·  {}", sim.seed, sim.battle.tick(), result);
}

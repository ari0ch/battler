//! Battler client: scenes, UI and battle presentation on top of `battler-sim`.
//!
//! Game rules live in the sim crate. This crate shows the state, turns clicks
//! and drags into [`ui::UiAction`]s, and plays battles back with animations.

mod actions;
mod battle;
mod cards;
mod editor;
mod effects;
mod field;
mod kit;
mod menu;
mod meters;
mod motion;
mod planning;
mod result;
mod sprites;
mod state;
mod ui;

use bevy::prelude::*;
use state::{AppState, MenuUi, Planning, Rev, RunPhase, Settings};

fn main() {
    App::new()
        .insert_resource(ClearColor(ui::color::BG_DEEP))
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
        .init_state::<AppState>()
        .add_sub_state::<RunPhase>()
        .init_resource::<Rev>()
        .init_resource::<Settings>()
        .init_resource::<MenuUi>()
        .init_resource::<Planning>()
        // After DefaultPlugins: decoding the sprites needs `Assets<Image>`.
        .init_resource::<sprites::Sprites>()
        .add_plugins(kit::KitPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(field::FieldPlugin)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2d);
        })
        .add_plugins((
            menu::MenuPlugin,
            planning::PlanningPlugin,
            editor::EditorPlugin,
            battle::BattlePlugin,
            meters::MetersPlugin,
            result::ResultPlugin,
        ))
        // Picking runs in PreUpdate, so clicks and drops are queued before this.
        .add_systems(Update, actions::apply_actions)
        .run();
}

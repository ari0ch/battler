//! Main menu.

use bevy::prelude::*;

use crate::state::{AppState, MenuUi, Rev};
use crate::ui::{self, PRIMARY, SECONDARY, UiAction, color, column, spawn_button, text};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(Update, rebuild.run_if(in_state(AppState::MainMenu)));
    }
}

#[derive(Component)]
struct MenuRoot;

fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    menu: Res<MenuUi>,
    roots: Query<Entity, With<MenuRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let stage =
        ui::spawn_stage(&mut commands, (MenuRoot, DespawnOnExit(AppState::MainMenu)), color::BG);
    commands.entity(stage).with_children(|stage| {
        stage
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                width: percent(100),
                height: percent(100),
                ..default()
            })
            .with_children(|body| {
                body.spawn(Node {
                    width: px(560),
                    padding: UiRect::new(px(96), px(0), px(96), px(64)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(40),
                    ..default()
                })
                .with_children(|left| {
                    left.spawn(column(12.0)).with_children(|title| {
                        title.spawn(text("BATTLER", 72.0, color::GOLD));
                        title.spawn(text(
                            "Program your warband. Watch the plan unfold.",
                            20.0,
                            color::MUTED,
                        ));
                    });
                    left.spawn(Node { width: px(320), ..column(14.0) }).with_children(|nav| {
                        spawn_button(
                            nav,
                            "New run",
                            UiAction::NewRun,
                            PRIMARY,
                            percent(100),
                            px(56),
                        );
                        let how = if menu.how_to { "Hide how to play" } else { "How to play" };
                        spawn_button(
                            nav,
                            how,
                            UiAction::ToggleHowTo,
                            SECONDARY,
                            percent(100),
                            px(52),
                        );
                    });
                    left.spawn(text("Prototype - placeholder art", 14.0, color::FAINT));
                });

                if menu.how_to {
                    body.spawn(ui::panel(Node {
                        width: px(560),
                        margin: UiRect::new(px(0), px(0), px(96), px(64)),
                        padding: UiRect::all(px(24)),
                        ..column(12.0)
                    }))
                    .with_children(|help| {
                        help.spawn(text("How to play", 22.0, color::TEXT));
                        for line in HOW_TO {
                            help.spawn(text(*line, 15.0, color::MUTED));
                        }
                    });
                }
            });
    });
}

const HOW_TO: &[&str] = &[
    "Each round has a timed planning phase, then a battle you watch.",
    "Buy units in the shop (3 gold). Three copies merge into the next level.",
    "Drag units onto your half of the board. One of each class at a time.",
    "Select a unit and press Edit cards to program it.",
    "A card is: if [condition] then [action]. Each tick a unit runs the first",
    "card that applies; if none does, it attacks the nearest enemy.",
    "Drag blocks from the palette into cards. Click a placed block to change",
    "its setting, such as a threshold or target.",
    "Losing a battle costs HP. Unspent gold earns interest. Survive 15 rounds.",
];

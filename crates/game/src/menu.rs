//! Main menu: solo play, and hosting or joining an online game.

use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle};
use bevy::ui_widgets::TextInput;

use crate::kit;
use crate::net::{self, Net};
use crate::state::{AppState, MenuUi, Rev};
use crate::ui::{self, PRIMARY, SECONDARY, UiAction, color, column, row, spawn_button, text};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, remember_inputs)
            .add_systems(OnEnter(AppState::MainMenu), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(
                Update,
                (rebuild, read_inputs).chain().run_if(in_state(AppState::MainMenu)),
            );
    }
}

#[derive(Component)]
struct MenuRoot;

/// Which box a text input is.
#[derive(Component, Clone, Copy, PartialEq, Eq)]
enum MenuField {
    Name,
    Code,
}

/// Fills in the name typed last time and the code from an invite link.
fn remember_inputs(mut menu: ResMut<MenuUi>) {
    menu.name = net::saved_name().unwrap_or_default();
    menu.code = net::invited_code().unwrap_or_default();
}

/// Keeps `MenuUi` in step with what's typed, so the Host and Join buttons
/// (handled in `actions.rs`) can read it.
fn read_inputs(fields: Query<(&EditableText, &MenuField)>, mut menu: ResMut<MenuUi>) {
    for (input, field) in &fields {
        let value = input.value().to_string();
        let slot = match field {
            MenuField::Name => &mut menu.name,
            MenuField::Code => &mut menu.code,
        };
        // Compare first: writing through `ResMut` marks it changed even if
        // the value is the same.
        if *slot != value {
            *slot = value;
        }
    }
}

/// A one-line text box in the kit's style.
fn text_box(field: MenuField, value: &str, max: usize, width: Val) -> impl Bundle {
    let filter = match field {
        MenuField::Name => EditableTextFilter::new(|c| c.is_ascii_graphic() || c == ' '),
        MenuField::Code => EditableTextFilter::new(|c| c.is_ascii_alphabetic()),
    };
    (
        Node {
            width,
            height: px(44),
            padding: UiRect::axes(px(12), px(8)),
            border: UiRect::all(px(2)),
            border_radius: BorderRadius::all(px(4)),
            align_items: AlignItems::Center,
            ..default()
        },
        TextInput,
        EditableText {
            max_characters: Some(max),
            allow_newlines: false,
            ..EditableText::new(value)
        },
        filter,
        TextLayout::no_wrap(),
        TextFont { font: kit::FONT_BODY.into(), font_size: FontSize::Px(24.0), ..default() },
        TextColor(color::TEXT),
        TextCursorStyle::default(),
        BackgroundColor(Color::srgb(0.10, 0.09, 0.13)),
        BorderColor::all(color::GOLD),
        field,
    )
}

fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    menu: Res<MenuUi>,
    net: Res<Net>,
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
                    padding: UiRect::new(px(96), px(0), px(64), px(48)),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(28),
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
                    left.spawn(Node { width: px(360), ..column(14.0) }).with_children(|nav| {
                        nav.spawn(text("Your name", 16.0, color::MUTED));
                        let mut name =
                            nav.spawn(text_box(MenuField::Name, &menu.name, 12, percent(100)));
                        if menu.name.is_empty() {
                            // `AutoFocus` puts the keyboard in this box when it appears.
                            name.insert(AutoFocus);
                        }
                        spawn_button(
                            nav,
                            "Play solo",
                            UiAction::NewRun,
                            PRIMARY,
                            percent(100),
                            px(52),
                        );
                        spawn_button(
                            nav,
                            "Host an online game",
                            UiAction::HostGame,
                            SECONDARY,
                            percent(100),
                            px(48),
                        );
                        nav.spawn(Node { width: percent(100), ..row(10.0) }).with_children(|r| {
                            let code = text_box(MenuField::Code, &menu.code, 4, px(120));
                            r.spawn(code);
                            spawn_button(
                                r,
                                "Join with code",
                                UiAction::JoinGame,
                                SECONDARY,
                                px(230),
                                px(44),
                            );
                        });
                        if let Some((code, _)) = net::saved_session() {
                            spawn_button(
                                nav,
                                &format!("Rejoin game {code}"),
                                UiAction::RejoinGame,
                                SECONDARY,
                                percent(100),
                                px(44),
                            );
                        }
                        if let Some(line) = net.status_line() {
                            let ink = match net.status {
                                net::Status::Failed(_) => color::RED_TEXT,
                                _ => color::GOLD_LIGHT,
                            };
                            nav.spawn(text(line, 15.0, ink));
                        }
                        let how = if menu.how_to { "Hide how to play" } else { "How to play" };
                        spawn_button(
                            nav,
                            how,
                            UiAction::ToggleHowTo,
                            SECONDARY,
                            percent(100),
                            px(44),
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
    "Eight players share one pool of characters. Last one standing wins.",
    "Play solo against seven bots, or host a room and share its code: up to",
    "eight people play together, and bots fill any empty seats.",
    "Each round has a timed planning phase, then a battle you watch. Rounds 1-3",
    "are monsters, and a boss comes every 4 rounds from round 7.",
    "Buy characters in the shop (1-3 gold by rarity). Three copies merge",
    "into the next level, which adds a card slot. One copy of each on the board.",
    "Your player level is your board size. Buy XP to level up: higher levels",
    "unlock new blocks and rarer characters.",
    "Click a unit to program it. A card is: if [condition] -> [action].",
    "Each tick a unit runs the first card that applies; otherwise it attacks",
    "the nearest enemy. Drag blocks into cards; click one to change it.",
    "Two of a class on the board unlocks its synergy. Hover anything for details.",
    "Losing a battle costs HP. Unspent gold earns interest.",
];

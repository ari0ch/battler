//! The waiting room of an online game: the room code to share, who's in, and
//! (for the host) the Start button. Empty seats are played by bots.

use battler_proto::MAX_PLAYERS;
use bevy::prelude::*;

use crate::net::Net;
use crate::state::{AppState, Rev};
use crate::ui::{
    self, PRIMARY, SECONDARY, UiAction, color, column, heading, row, spawn_button, text,
};

pub struct LobbyRoomPlugin;

impl Plugin for LobbyRoomPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Room), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(Update, rebuild.run_if(in_state(AppState::Room)));
    }
}

#[derive(Component)]
struct RoomRoot;

fn rebuild(
    mut commands: Commands,
    rev: Res<Rev>,
    net: Res<Net>,
    roots: Query<Entity, With<RoomRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    for e in &roots {
        commands.entity(e).despawn();
    }
    let Some(room) = &net.room else { return };
    let i_host = room.you == room.host;
    let stage =
        ui::spawn_stage(&mut commands, (RoomRoot, DespawnOnExit(AppState::Room)), color::BG);
    commands.entity(stage).with_children(|stage| {
        stage
            .spawn(ui::panel(Node { width: px(560), padding: UiRect::all(px(28)), ..column(16.0) }))
            .with_children(|panel| {
                panel.spawn(text("Online game", 18.0, color::MUTED));
                panel.spawn(Node { align_items: AlignItems::Center, ..row(16.0) }).with_children(
                    |r| {
                        r.spawn(heading(format!("Room {}", room.code), 40.0, color::GOLD_LIGHT));
                        spawn_button(
                            r,
                            "Copy invite link",
                            UiAction::CopyLink,
                            SECONDARY,
                            px(190),
                            px(40),
                        );
                    },
                );
                panel.spawn(text(
                    "Share the code (or the link). Bots take any empty seats.",
                    15.0,
                    color::MUTED,
                ));

                panel.spawn(column(6.0)).with_children(|list| {
                    for seat in 0..MAX_PLAYERS {
                        list.spawn(Node {
                            height: px(36),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect::horizontal(px(12)),
                            ..row(8.0)
                        })
                        .insert(BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.25)))
                        .with_children(|line| {
                            match room.seats.get(seat) {
                                Some(info) => {
                                    let mut name = info.name.clone();
                                    if seat == room.host {
                                        name.push_str("  (host)");
                                    }
                                    if seat == room.you {
                                        name.push_str("  (you)");
                                    }
                                    let ink = if seat == room.you {
                                        color::GOLD_LIGHT
                                    } else {
                                        color::TEXT
                                    };
                                    line.spawn(text(name, 18.0, ink));
                                    if i_host && seat != room.you {
                                        spawn_button(
                                            line,
                                            "Remove",
                                            UiAction::Kick(seat),
                                            SECONDARY,
                                            px(96),
                                            px(28),
                                        );
                                    }
                                }
                                None => {
                                    line.spawn(text("Bot", 18.0, color::FAINT));
                                }
                            }
                        });
                    }
                });

                if let Some(notice) = &net.notice {
                    panel.spawn(text(notice.clone(), 15.0, color::GOLD_LIGHT));
                }
                panel
                    .spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(12.0) })
                    .with_children(|buttons| {
                        spawn_button(
                            buttons,
                            "Leave",
                            UiAction::LeaveRoom,
                            SECONDARY,
                            px(140),
                            px(48),
                        );
                        if i_host {
                            spawn_button(
                                buttons,
                                "Start game",
                                UiAction::StartGame,
                                PRIMARY,
                                px(220),
                                px(52),
                            );
                        } else {
                            buttons.spawn(text(
                                "Waiting for the host to start...",
                                16.0,
                                color::MUTED,
                            ));
                        }
                    });
            });
    });
}

//! The round result pop-up (shown over the planning screen), and game over with
//! the run summary.

use battler_sim::run::economy::{BOSS_EXTRA_DAMAGE, LOBBY_SIZE, round_damage};
use battler_sim::run::{Lobby, Opponent, RoundReport, RunStatus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::cards;
use crate::kit;
use crate::planning::spawn_pips;
use crate::sprites::{Sprites, sprite_node};
use crate::state::{AppState, GameRun, LobbyExt, Rev};
use crate::ui::{
    self, PRIMARY, SECONDARY, UiAction, color, column, heading, row, spawn_button, text,
};

pub struct ResultPlugin;

impl Plugin for ResultPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::GameOver), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(Update, rebuild_game_over.run_if(in_state(AppState::GameOver)));
    }
}

#[derive(Component)]
struct GameOverRoot;

/// The round's result pop-up's countdown label, updated in place.
#[derive(Component)]
pub struct ResultCountdown;

fn outcome_words(report: &RoundReport) -> (&'static str, Color) {
    match report.result {
        Some(true) => ("Victory!", color::GOOD),
        Some(false) => ("Defeat", color::BAD),
        None => ("Draw", color::MUTED),
    }
}

/// "1st", "2nd", "3rd", "4th"...
pub fn ordinal(n: u32) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, 11) | (2, 12) | (3, 13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

/// Why the player lost the HP they did (or didn't).
fn damage_note(r: &RoundReport) -> String {
    match (r.opponent, r.damage) {
        (Opponent::Ghost(_), _) if r.result == Some(false) => {
            "Losing to a ghost costs no health.".to_string()
        }
        (_, 0) => "No damage taken.".to_string(),
        (Opponent::Monsters(w), d) if w.is_boss() => format!(
            "-{d} HP: {} for round {} + {BOSS_EXTRA_DAMAGE} for a boss",
            round_damage(r.round),
            r.round
        ),
        (Opponent::Monsters(_), d) => format!("-{d} HP to the monsters"),
        (_, d) => {
            let base = round_damage(r.round);
            format!(
                "-{d} HP: {base} for round {} + {} for surviving enemies",
                r.round,
                d.saturating_sub(base)
            )
        }
    }
}

/// The last round's result as a small pop-up over the planning screen, so the
/// next planning timer can already be running underneath it. Hover the gold
/// line for the breakdown.
pub fn spawn_result_popup(stage: &mut ChildSpawnerCommands, lobby: &Lobby, r: &RoundReport) {
    stage
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                right: px(0),
                top: px(150),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(80),
            Pickable::IGNORE,
        ))
        .with_children(|centre| {
            centre
                .spawn(ui::panel(Node {
                    width: px(420),
                    padding: UiRect::axes(px(30), px(24)),
                    align_items: AlignItems::Center,
                    ..column(10.0)
                }))
                .with_children(|card| {
                    let (headline, ink) = outcome_words(r);
                    card.spawn(text(
                        format!(
                            "Round {} - vs {}",
                            r.round,
                            cards::opponent_label(lobby, r.opponent)
                        ),
                        13.0,
                        color::MUTED,
                    ));
                    card.spawn(ui::outlined(headline, 40.0, ink));
                    card.spawn(row(18.0)).with_children(|line| {
                        line.spawn(row(6.0)).with_children(|hp| {
                            hp.spawn(kit::icon(kit::img::ICON_HEART, 24.0));
                            hp.spawn(heading(r.hp_after.to_string(), 18.0, color::TEXT));
                        });
                        line.spawn((row(6.0), Hovered::default(), gold_tooltip(r))).with_children(
                            |g| {
                                g.spawn(kit::icon(kit::img::ICON_COIN, 24.0));
                                g.spawn(heading(
                                    format!("{} (+{})", r.gold_after, r.income.total()),
                                    18.0,
                                    color::GOLD_LIGHT,
                                ));
                            },
                        );
                    });
                    card.spawn(text(damage_note(r), 13.0, color::MUTED));
                    if let Some(level) = r.level_ups.last() {
                        card.spawn(heading(format!("Level {level}!"), 15.0, color::GOLD_LIGHT));
                    }
                    card.spawn(ui::button(
                        UiAction::Continue,
                        PRIMARY,
                        Node {
                            width: px(160),
                            height: px(44),
                            margin: UiRect::top(px(4)),
                            ..default()
                        },
                    ))
                    .with_child((heading("OK", 16.0, color::ON_GOLD), ResultCountdown));
                });
        });
}

/// The gold breakdown, on hover.
fn gold_tooltip(r: &RoundReport) -> ui::Tooltip {
    let streak = if r.streak >= 0 { "wins" } else { "losses" };
    let reward = if r.opponent.is_pve() { "Reward" } else { "Win" };
    ui::Tooltip {
        title: "Gold for next round".to_string(),
        accent: color::GOLD_LIGHT,
        lines: vec![
            (format!("Banked {}", r.banked), color::TEXT),
            (format!("Base +{}", r.income.base), color::TEXT),
            (format!("{reward} +{}", r.income.win), color::TEXT),
            (format!("Interest +{}", r.income.interest), color::TEXT),
            (
                format!("Streak ({} {streak}) +{}", r.streak.unsigned_abs(), r.income.streak),
                color::TEXT,
            ),
            (format!("Total {} gold", r.gold_after), color::GOLD_LIGHT),
        ],
    }
}

fn rebuild_game_over(
    mut commands: Commands,
    rev: Res<Rev>,
    run: Option<Res<GameRun>>,
    sprites: Res<Sprites>,
    roots: Query<Entity, With<GameOverRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    let Some(run) = run else { return };
    let lobby = &run.0;
    let run = lobby.me();
    for e in &roots {
        commands.entity(e).despawn();
    }
    let reports: Vec<&RoundReport> = run.history().iter().map(|(_, r)| r).collect();
    let wins = reports.iter().filter(|r| r.result == Some(true)).count();
    let last_round = reports.last().map_or(1, |r| r.round);
    let placement = run.placement().unwrap_or(LOBBY_SIZE as u32);
    let best_streak = reports.iter().map(|r| r.streak).max().unwrap_or(0).max(0);
    let gold_earned: u32 = reports.iter().map(|r| r.income.total()).sum();

    let stage = ui::spawn_stage(
        &mut commands,
        (GameOverRoot, DespawnOnExit(AppState::GameOver)),
        color::BG,
    );
    commands.entity(stage).insert(Node {
        width: px(ui::STAGE_W),
        height: px(ui::STAGE_H),
        padding: UiRect::axes(px(56), px(48)),
        ..row(32.0)
    });
    commands.entity(stage).with_children(|stage| {
        stage
            .spawn(Node {
                width: px(420),
                flex_shrink: 0.0,
                align_self: AlignSelf::Stretch,
                ..column(28.0)
            })
            .with_children(|left| {
                let (headline, ink) = match run.status() {
                    RunStatus::Victory => ("Victory".to_string(), color::GOOD),
                    _ => (format!("{} of {LOBBY_SIZE}", ordinal(placement)), color::BAD),
                };
                left.spawn(column(8.0)).with_children(|c| {
                    c.spawn(text(headline, 64.0, ink));
                    let sub = match run.status() {
                        RunStatus::Victory => {
                            format!("Last one standing after {last_round} rounds")
                        }
                        _ => format!("Knocked out in round {last_round}"),
                    };
                    c.spawn(text(sub, 17.0, color::MUTED));
                });
                left.spawn(Node {
                    flex_wrap: FlexWrap::Wrap,
                    row_gap: px(12),
                    column_gap: px(12),
                    ..default()
                })
                .with_children(|grid| {
                    let stats = [
                        ("Rounds won", format!("{wins} / {}", reports.len())),
                        ("Best win streak", best_streak.to_string()),
                        ("Gold earned", format!("{gold_earned} g")),
                        ("Placement", ordinal(placement)),
                    ];
                    for (label, value) in stats {
                        grid.spawn(ui::panel(Node {
                            width: px(198),
                            padding: UiRect::all(px(14)),
                            ..column(4.0)
                        }))
                        .with_children(|s| {
                            s.spawn(text(label, 13.0, color::MUTED));
                            s.spawn(text(value, 22.0, color::TEXT));
                        });
                    }
                });
                left.spawn(column(10.0)).with_children(|w| {
                    w.spawn(text("Final board", 13.0, color::MUTED));
                    w.spawn(row(8.0)).with_children(|r| {
                        for (_, unit) in run.board() {
                            r.spawn((
                                Node {
                                    width: px(56),
                                    height: px(56),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    row_gap: px(3),
                                    border: UiRect::all(px(2)),
                                    border_radius: BorderRadius::all(px(10)),
                                    ..default()
                                },
                                BackgroundColor(color::BLUE_FILL),
                                BorderColor::all(color::BLUE),
                            ))
                            .with_children(|t| {
                                t.spawn(sprite_node(sprites.character(unit.character), 1.0, false));
                                spawn_pips(t, unit.level);
                            });
                        }
                        if run.board().is_empty() {
                            r.spawn(text("(empty)", 14.0, color::FAINT));
                        }
                    });
                });
                left.spawn(Node { margin: UiRect::top(Val::Auto), ..row(12.0) }).with_children(
                    |b| {
                        spawn_button(b, "New run", UiAction::NewRun, PRIMARY, px(150), px(52));
                        spawn_button(
                            b,
                            "Main menu",
                            UiAction::MainMenu,
                            SECONDARY,
                            px(150),
                            px(52),
                        );
                    },
                );
            });

        stage
            .spawn(ui::panel(Node {
                flex_grow: 1.0,
                align_self: AlignSelf::Stretch,
                padding: UiRect::all(px(20)),
                overflow: Overflow::clip(),
                ..column(6.0)
            }))
            .with_children(|table| {
                table
                    .spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(0.0) })
                    .with_children(|h| {
                        h.spawn(text("Run summary", 16.0, color::TEXT));
                        let note = if reports.len() > MAX_ROWS {
                            format!("Last {MAX_ROWS} of {} rounds", reports.len())
                        } else {
                            "Replays rebuild each battle from its seed".to_string()
                        };
                        h.spawn(text(note, 13.0, color::FAINT));
                    });
                let header = ["Round", "Opponent", "Result", "HP change", "HP after"];
                table
                    .spawn(Node { padding: UiRect::horizontal(px(10)), ..row(0.0) })
                    .with_children(|h| {
                        for (i, label) in header.iter().enumerate() {
                            h.spawn(Node { width: px(COLS[i]), ..default() }).with_child(text(
                                *label,
                                12.0,
                                color::FAINT,
                            ));
                        }
                    });
                // Long runs don't fit: show the most recent rounds.
                let shown = reports.len().saturating_sub(MAX_ROWS);
                for (i, r) in reports.iter().enumerate().skip(shown) {
                    table
                        .spawn((
                            Node {
                                padding: UiRect::axes(px(10), px(2)),
                                border_radius: BorderRadius::all(px(6)),
                                ..row(0.0)
                            },
                            BackgroundColor(color::PANEL_2),
                        ))
                        .with_children(|line| {
                            let (word, ink) = outcome_words(r);
                            let delta = if r.damage > 0 {
                                format!("-{}", r.damage)
                            } else {
                                "-".to_string()
                            };
                            let cells = [
                                (r.round.to_string(), color::TEXT),
                                (cards::opponent_label(lobby, r.opponent), color::MUTED),
                                (word.to_string(), ink),
                                (delta, color::MUTED),
                                (r.hp_after.to_string(), color::TEXT),
                            ];
                            for (j, (value, c)) in cells.into_iter().enumerate() {
                                line.spawn(Node { width: px(COLS[j]), ..default() })
                                    .with_child(text(value, 14.0, c));
                            }
                            spawn_button(
                                line,
                                "Replay",
                                UiAction::Replay(i),
                                SECONDARY,
                                px(90),
                                px(28),
                            );
                        });
                }
            });
    });
}

/// Column widths for the run summary table.
const COLS: [f32; 5] = [64.0, 170.0, 90.0, 100.0, 80.0];
/// Rounds listed in the run summary.
const MAX_ROWS: usize = 16;

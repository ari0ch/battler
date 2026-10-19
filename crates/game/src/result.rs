//! The round result screen, and game over with the run summary.

use battler_sim::run::economy::{BOSS_EXTRA_DAMAGE, LOBBY_SIZE, round_damage};
use battler_sim::run::{Lobby, Opponent, RoundReport, RunStatus, round_kind};
use bevy::prelude::*;

use crate::cards;
use crate::planning::spawn_pips;
use crate::state::{AppState, GameRun, LastReport, LobbyExt, Rev, RunPhase};
use crate::ui::{
    self, ActionQueue, PRIMARY, SECONDARY, UiAction, color, column, row, spawn_button, text,
};

pub struct ResultPlugin;

impl Plugin for ResultPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AutoContinue>()
            .add_systems(OnEnter(RunPhase::Result), enter_result)
            .add_systems(
                Update,
                (rebuild_result, count_down).chain().run_if(in_state(RunPhase::Result)),
            )
            .add_systems(OnEnter(AppState::GameOver), |mut rev: ResMut<Rev>| rev.bump())
            .add_systems(Update, rebuild_game_over.run_if(in_state(AppState::GameOver)));
    }
}

/// Seconds before the result screen moves on by itself.
const AUTO_CONTINUE: f32 = 10.0;

#[derive(Resource, Default)]
struct AutoContinue(f32);

#[derive(Component)]
struct ResultRoot;
#[derive(Component)]
struct ContinueLabel;
#[derive(Component)]
struct GameOverRoot;

fn enter_result(mut rev: ResMut<Rev>, mut auto: ResMut<AutoContinue>) {
    auto.0 = AUTO_CONTINUE;
    rev.bump();
}

fn count_down(
    time: Res<Time<Real>>,
    mut auto: ResMut<AutoContinue>,
    mut queue: ResMut<ActionQueue>,
    mut labels: Query<&mut Text, With<ContinueLabel>>,
) {
    let before = auto.0;
    auto.0 -= time.delta_secs();
    if before > 0.0 && auto.0 <= 0.0 {
        queue.0.push(UiAction::Continue);
    }
    for mut t in &mut labels {
        **t = format!("Continue - {}", auto.0.max(0.0).ceil() as u32);
    }
}

fn outcome_words(report: &RoundReport) -> (&'static str, Color) {
    match report.result {
        Some(true) => ("Victory", color::GOOD),
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
            "Lost {d} HP: {} for round {} + {BOSS_EXTRA_DAMAGE} for a boss.",
            round_damage(r.round),
            r.round
        ),
        (Opponent::Monsters(_), d) => format!("Lost {d} HP to the monsters."),
        (_, d) => {
            let base = round_damage(r.round);
            format!(
                "Lost {d} HP: {base} for round {} + {} for surviving enemy levels.",
                r.round,
                d.saturating_sub(base)
            )
        }
    }
}

fn rebuild_result(
    mut commands: Commands,
    rev: Res<Rev>,
    report: Option<Res<LastReport>>,
    run: Option<Res<GameRun>>,
    roots: Query<Entity, With<ResultRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    let (Some(report), Some(run)) = (report, run) else { return };
    let r = &report.0;
    let lobby: &Lobby = &run.0;
    for e in &roots {
        commands.entity(e).despawn();
    }
    let stage = ui::spawn_stage(
        &mut commands,
        (ResultRoot, DespawnOnExit(RunPhase::Result)),
        color::BG_DEEP,
    );
    commands.entity(stage).insert(Node {
        width: px(ui::STAGE_W),
        height: px(ui::STAGE_H),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    });
    commands.entity(stage).with_children(|stage| {
        stage
            .spawn(ui::panel(Node {
                width: px(760),
                padding: UiRect::axes(px(48), px(40)),
                ..column(28.0)
            }))
            .with_children(|card| {
                let (headline, ink) = outcome_words(r);
                card.spawn(Node { align_items: AlignItems::Center, ..column(8.0) }).with_children(
                    |c| {
                        c.spawn(text(format!("Round {}", r.round), 14.0, color::MUTED));
                        c.spawn(text(headline, 56.0, ink));
                        let ticks = if r.ticks == 1 { "tick" } else { "ticks" };
                        c.spawn(text(
                            format!(
                                "vs {} - {} of your units survived - {} {ticks}",
                                cards::opponent_label(lobby, r.opponent),
                                r.player_survivors,
                                r.ticks
                            ),
                            16.0,
                            color::MUTED,
                        ));
                    },
                );

                card.spawn(row(20.0)).with_children(|cols| {
                    cols.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            padding: UiRect::all(px(18)),
                            align_self: AlignSelf::Stretch,
                            border_radius: BorderRadius::all(px(12)),
                            ..column(12.0)
                        },
                        BackgroundColor(color::PANEL_2),
                    ))
                    .with_children(|hp| {
                        hp.spawn(text("Health", 13.0, color::MUTED));
                        hp.spawn(text(format!("{} / 100", r.hp_after), 32.0, color::TEXT));
                        hp.spawn((
                            Node {
                                height: px(8),
                                border_radius: BorderRadius::all(px(4)),
                                overflow: Overflow::clip(),
                                ..default()
                            },
                            BackgroundColor(color::LINE),
                        ))
                        .with_child((
                            Node { width: percent(r.hp_after as f32), height: px(8), ..default() },
                            BackgroundColor(color::HP_FILL),
                        ));
                        hp.spawn(text(damage_note(r), 13.0, color::FAINT));
                    });

                    cols.spawn((
                        Node {
                            flex_grow: 1.0,
                            flex_basis: px(0),
                            padding: UiRect::all(px(18)),
                            border_radius: BorderRadius::all(px(12)),
                            ..column(8.0)
                        },
                        BackgroundColor(color::PANEL_2),
                    ))
                    .with_children(|g| {
                        g.spawn(text("Gold for next round", 13.0, color::MUTED));
                        let streak = if r.streak >= 0 {
                            format!("Streak ({} wins)", r.streak)
                        } else {
                            format!("Streak ({} losses)", -r.streak)
                        };
                        let lines = [
                            ("Banked".to_string(), r.banked.to_string()),
                            ("Base".to_string(), format!("+{}", r.income.base)),
                            (
                                if r.opponent.is_pve() { "Reward" } else { "Win" }.to_string(),
                                format!("+{}", r.income.win),
                            ),
                            (
                                format!("Interest ({} banked)", r.banked),
                                format!("+{}", r.income.interest),
                            ),
                            (streak, format!("+{}", r.income.streak)),
                        ];
                        for (label, value) in lines {
                            g.spawn(Node {
                                justify_content: JustifyContent::SpaceBetween,
                                ..row(0.0)
                            })
                            .with_children(|l| {
                                l.spawn(text(label, 15.0, color::MUTED));
                                l.spawn(text(value, 15.0, color::GOLD_LIGHT));
                            });
                        }
                        g.spawn((
                            Node { height: px(1), margin: UiRect::vertical(px(4)), ..default() },
                            BackgroundColor(color::LINE_STRONG),
                        ));
                        g.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(0.0) })
                            .with_children(|l| {
                                l.spawn(text("Total", 17.0, color::TEXT));
                                l.spawn(text(
                                    format!("{} g", r.gold_after),
                                    17.0,
                                    color::GOLD_LIGHT,
                                ));
                            });
                    });
                });

                card.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(20.0) })
                    .with_children(|f| {
                        let next = match r.placement {
                            Some(1) => "You are the last one standing!".to_string(),
                            Some(n) => {
                                format!("Knocked out - you placed {} of {LOBBY_SIZE}.", ordinal(n))
                            }
                            None => {
                                let kind = cards::round_label(round_kind(r.round + 1));
                                let left = lobby.alive_count();
                                match r.level_ups.last() {
                                    Some(level) => format!(
                                        "Next: round {} ({kind}) - you reached level {level}!",
                                        r.round + 1
                                    ),
                                    None => format!(
                                        "Next: round {} ({kind}) - {left} players left",
                                        r.round + 1
                                    ),
                                }
                            }
                        };
                        f.spawn(text(next, 14.0, color::MUTED));
                        f.spawn(ui::button(
                            UiAction::Continue,
                            PRIMARY,
                            Node { width: px(190), height: px(52), ..default() },
                        ))
                        .with_child((text("Continue", 16.0, color::ON_GOLD), ContinueLabel));
                    });
            });
    });
}

fn rebuild_game_over(
    mut commands: Commands,
    rev: Res<Rev>,
    run: Option<Res<GameRun>>,
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
                                t.spawn(text(
                                    cards::initials(unit.character),
                                    16.0,
                                    color::BLUE_TEXT,
                                ));
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

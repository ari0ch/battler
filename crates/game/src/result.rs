//! The round result screen, and game over with the run summary.

use battler_sim::run::{RoundReport, RunStatus};
use battler_sim::{Outcome, Team};
use bevy::prelude::*;

use crate::planning::{class_letter, spawn_pips};
use crate::state::{AppState, GameRun, LastReport, Rev, RunPhase};
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
    match report.outcome {
        Outcome::Won(Team::Player) => ("Victory", color::GOOD),
        Outcome::Won(Team::Enemy) => ("Defeat", color::BAD),
        _ => ("Draw", color::MUTED),
    }
}

fn rebuild_result(
    mut commands: Commands,
    rev: Res<Rev>,
    report: Option<Res<LastReport>>,
    roots: Query<Entity, With<ResultRoot>>,
) {
    if !rev.is_changed() {
        return;
    }
    let Some(report) = report else { return };
    let r = &report.0;
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
                        c.spawn(text(
                            format!(
                                "{} of your units survived - {} ticks",
                                r.player_survivors, r.ticks
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
                        let note = if r.damage > 0 {
                            format!("Lost {} HP: 2 + levels of surviving enemies.", r.damage)
                        } else {
                            "No damage taken.".to_string()
                        };
                        hp.spawn(text(note, 13.0, color::FAINT));
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
                            ("Win".to_string(), format!("+{}", r.income.win)),
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
                        let next = match r.status {
                            RunStatus::Planning => format!("Next: round {}", r.round + 1),
                            RunStatus::Victory => "You survived every round!".to_string(),
                            RunStatus::Defeat => "Your warband has fallen.".to_string(),
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
    let run = &run.0;
    for e in &roots {
        commands.entity(e).despawn();
    }
    let reports: Vec<&RoundReport> = run.history().iter().map(|(_, r)| r).collect();
    let wins = reports.iter().filter(|r| r.outcome == Outcome::Won(Team::Player)).count();
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
                    RunStatus::Victory => ("Victory", color::GOOD),
                    _ => ("Defeat", color::BAD),
                };
                left.spawn(column(8.0)).with_children(|c| {
                    c.spawn(text(headline, 64.0, ink));
                    let sub = match run.status() {
                        RunStatus::Victory => format!("Survived all {} rounds", run.round()),
                        _ => format!("Fell in round {} of 15", run.round()),
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
                        ("HP left", run.hp().to_string()),
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
                                t.spawn(text(class_letter(unit.class), 17.0, color::BLUE_TEXT));
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
                        h.spawn(text(
                            "Replays rebuild each battle from its seed",
                            13.0,
                            color::FAINT,
                        ));
                    });
                let header = ["Round", "Result", "HP change", "HP after"];
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
                for (i, r) in reports.iter().enumerate() {
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
const COLS: [f32; 4] = [80.0, 100.0, 140.0, 120.0];

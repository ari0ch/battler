//! Applies queued [`UiAction`]s: the one place where UI input changes the run.

use battler_sim::run::{Location, Run, RunStatus};
use bevy::prelude::*;

use crate::battle::Playback;
use crate::editor;
use crate::state::{AppState, GameRun, MenuUi, Planning, Rev, RunPhase, Settings};
use crate::ui::{ActionQueue, DragSource, DropTarget, UiAction};

#[allow(clippy::too_many_arguments)]
pub fn apply_actions(
    mut commands: Commands,
    mut queue: ResMut<ActionQueue>,
    mut rev: ResMut<Rev>,
    time: Res<Time<Real>>,
    state: Res<State<AppState>>,
    mut next_app: ResMut<NextState<AppState>>,
    mut next_phase: Option<ResMut<NextState<RunPhase>>>,
    mut run: Option<ResMut<GameRun>>,
    mut planning: ResMut<Planning>,
    mut menu: ResMut<MenuUi>,
    mut settings: ResMut<Settings>,
    mut playback: Option<ResMut<Playback>>,
) {
    if queue.0.is_empty() {
        return;
    }
    let mut actions = std::mem::take(&mut queue.0);
    // Clicking a unit tile can also register as a click on the cell beneath it
    // (tiles let the pointer through so drops work). The tile wins.
    if actions.iter().any(|a| matches!(a, UiAction::SelectUnit(_))) {
        actions.retain(|a| !matches!(a, UiAction::ClickCell(_) | UiAction::ClickBench(_)));
    }

    for action in actions {
        rev.bump();
        match action {
            UiAction::NewRun => {
                // Any varying number makes a fresh seed; real elapsed time works on the web too.
                let seed = time.elapsed().as_nanos() as u64 ^ 0x5EED_BA77_1E50_0001;
                commands.insert_resource(GameRun(Run::new(seed)));
                *planning = Planning::default();
                next_app.set(AppState::InRun);
                if let Some(next) = next_phase.as_mut() {
                    next.set(RunPhase::Planning);
                }
            }
            UiAction::ToggleHowTo => menu.how_to = !menu.how_to,
            UiAction::MainMenu => next_app.set(AppState::MainMenu),
            UiAction::Speed(s) => settings.speed = s,
            UiAction::ToggleTrace => settings.card_trace = !settings.card_trace,
            UiAction::Skip => {
                if let Some(pb) = playback.as_mut() {
                    pb.skip = true;
                }
            }
            UiAction::BackToSummary => next_app.set(AppState::GameOver),
            UiAction::Replay(i) => {
                let Some(run) = run.as_ref() else { continue };
                if let Some((setup, _)) = run.0.history().get(i) {
                    commands.insert_resource(Playback::new(setup.clone(), true));
                    next_app.set(AppState::Replay);
                }
            }
            UiAction::Continue => {
                let Some(run) = run.as_ref() else { continue };
                if run.0.status() == RunStatus::Planning {
                    if let Some(next) = next_phase.as_mut() {
                        next.set(RunPhase::Planning);
                    }
                } else {
                    next_app.set(AppState::GameOver);
                }
            }
            other => {
                if *state.get() != AppState::InRun {
                    continue;
                }
                let Some(run) = run.as_mut() else { continue };
                planning_action(
                    &mut commands,
                    &mut run.0,
                    &mut planning,
                    next_phase.as_deref_mut(),
                    other,
                );
            }
        }
    }
}

fn report<T>(planning: &mut Planning, result: Result<T, impl std::fmt::Display>) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(e) => {
            planning.toast(e.to_string());
            None
        }
    }
}

fn selected_loc(run: &Run, planning: &Planning) -> Option<Location> {
    planning.selected.and_then(|id| run.locate(id))
}

fn move_selected_to(run: &mut Run, planning: &mut Planning, to: Location) {
    if let Some(from) = selected_loc(run, planning) {
        report(planning, run.move_unit(from, to));
    }
}

fn planning_action(
    commands: &mut Commands,
    run: &mut Run,
    planning: &mut Planning,
    next_phase: Option<&mut NextState<RunPhase>>,
    action: UiAction,
) {
    match action {
        UiAction::Buy(i) => {
            if let Some(id) = report(planning, run.buy(i)) {
                if planning.selected.is_none() {
                    planning.selected = Some(id);
                }
            }
        }
        UiAction::Reroll => {
            report(planning, run.reroll());
        }
        UiAction::Ready => {
            if let Some(setup) = report(planning, run.start_battle()) {
                planning.editor_open = false;
                commands.insert_resource(Playback::new(setup, false));
                if let Some(next) = next_phase {
                    next.set(RunPhase::Battle);
                }
            }
        }
        UiAction::SelectUnit(loc) => {
            planning.selected = run.unit_at(loc).map(|u| u.id);
            if planning.editor_open {
                editor::load_draft(run, planning);
            }
        }
        UiAction::ClickCell(p) => move_selected_to(run, planning, Location::Board(p)),
        UiAction::ClickBench(i) => move_selected_to(run, planning, Location::Bench(i)),
        UiAction::Sell => {
            if let Some(loc) = selected_loc(run, planning) {
                if report(planning, run.sell(loc)).is_some() {
                    planning.selected = None;
                    planning.editor_open = false;
                }
            }
        }
        UiAction::OpenEditor => {
            if selected_loc(run, planning).is_some() {
                planning.editor_open = true;
                editor::load_draft(run, planning);
            }
        }
        UiAction::CloseEditor => planning.editor_open = false,
        UiAction::PaletteBlock(block) => editor::click_block(run, planning, block),
        UiAction::CycleSlot(card, slot) => editor::cycle_slot(run, planning, card, slot),
        UiAction::ClearCard(card) => editor::clear_card(run, planning, card),
        UiAction::CardUp(card) => editor::card_up(run, planning, card),
        UiAction::Drop(source, target) => match (source, target) {
            (DragSource::Unit(from), DropTarget::Cell(p)) => {
                if report(planning, run.move_unit(from, Location::Board(p))).is_some() {
                    planning.selected = run.unit_at(Location::Board(p)).map(|u| u.id);
                }
            }
            (DragSource::Unit(from), DropTarget::Bench(i)) => {
                report(planning, run.move_unit(from, Location::Bench(i)));
            }
            (DragSource::Unit(from), DropTarget::Sell) => {
                let sold = run.unit_at(from).map(|u| u.id);
                if report(planning, run.sell(from)).is_some() && planning.selected == sold {
                    planning.selected = None;
                    planning.editor_open = false;
                }
            }
            (DragSource::Block(block), DropTarget::Card(card, slot)) => {
                editor::drop_block(run, planning, card, slot, block)
            }
            (DragSource::Block(_), _) | (DragSource::Unit(_), DropTarget::Card(..)) => {}
        },
        _ => {}
    }
    // Keep the editor pointed at a unit that still exists.
    if planning.editor_open && selected_loc(run, planning).is_none() {
        planning.editor_open = false;
    }
}

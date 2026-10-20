//! Applies queued [`UiAction`]s: the one place where UI input changes the run.

use battler_sim::run::{Applied, Command, Lobby, Location, Player};
use bevy::prelude::*;

use crate::battle::Playback;
use crate::editor;
use crate::state::{AppState, GameRun, LobbyExt, MenuUi, Planning, Rev, RunPhase, Settings};
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
    // The field picker may report the same click more than once.
    actions.dedup();
    // A unit on the field is also a drop target for its own cell, and it is
    // under the pointer for the whole drag (it follows it), so every drop
    // would "land" back where it started. Ignore drops onto yourself.
    actions.retain(|a| {
        !matches!(a, UiAction::Drop(DragSource::Unit(Location::Board(from)), DropTarget::Cell(to)) if from == to)
    });
    // A drop onto a unit standing on the field also lands on the field under
    // the pointer, which may be the cell behind it (sprites are taller than
    // their tile). The unit's own cell wins.
    if actions.iter().any(|a| matches!(a, UiAction::Drop(_, DropTarget::Cell(_)))) {
        actions.retain(|a| !matches!(a, UiAction::Drop(_, DropTarget::Field(_))));
    }

    for action in actions {
        rev.bump();
        match action {
            UiAction::NewRun => {
                // Any varying number makes a fresh seed; real elapsed time works on the web too.
                let seed = time.elapsed().as_nanos() as u64 ^ 0x5EED_BA77_1E50_0001;
                commands.insert_resource(GameRun(Lobby::new(seed, 1)));
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
            UiAction::TogglePlayers => settings.show_players = !settings.show_players,
            UiAction::ToggleMeters => settings.show_meters = !settings.show_meters,
            UiAction::MeterTab(tab) => settings.meter_tab = tab,
            UiAction::Skip => {
                if let Some(pb) = playback.as_mut() {
                    pb.skip = true;
                }
            }
            UiAction::BackToSummary => next_app.set(AppState::GameOver),
            UiAction::Replay(i) => {
                let Some(run) = run.as_ref() else { continue };
                if let Some((setup, report)) = run.0.me().history().get(i) {
                    let opponent = Some(report.opponent);
                    commands.insert_resource(Playback::new(setup.clone(), true, opponent));
                    next_app.set(AppState::Replay);
                }
            }
            UiAction::Continue => planning.result = None,
            UiAction::ToggleEditor => planning.editor_open = !planning.editor_open,
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

fn selected_loc(me: &Player, planning: &Planning) -> Option<Location> {
    planning.selected.and_then(|id| me.locate(id))
}

fn move_selected_to(lobby: &mut Lobby, planning: &mut Planning, to: Location) {
    if let Some(from) = selected_loc(lobby.me(), planning) {
        report(planning, lobby.act(Command::Move { from, to }));
    }
}

/// Clicking a board cell or bench slot: if a unit is there, select it (as if
/// the unit itself was clicked); if it's empty, move the selected unit there.
///
/// Clicking a unit also reaches the spot beneath it (units let the pointer
/// through for drops). The unit's own click normally wins, but if it is ever
/// lost, moving the selected unit here would swap two units by surprise, so
/// an occupied spot never takes a move from a click. Swaps are for dragging.
fn click_spot(lobby: &mut Lobby, planning: &mut Planning, loc: Location) {
    match lobby.me().unit_at(loc).map(|u| u.id) {
        Some(id) => {
            if planning.selected != Some(id) {
                planning.selected = Some(id);
                editor::load_draft(lobby.me(), planning);
            }
            planning.editor_open = true;
        }
        None => move_selected_to(lobby, planning, loc),
    }
}

fn planning_action(
    commands: &mut Commands,
    lobby: &mut Lobby,
    planning: &mut Planning,
    next_phase: Option<&mut NextState<RunPhase>>,
    action: UiAction,
) {
    match action {
        UiAction::Buy(i) => {
            if let Some(Applied::Bought(id)) = report(planning, lobby.act(Command::Buy(i)))
                && planning.selected.is_none()
            {
                planning.selected = Some(id);
                editor::load_draft(lobby.me(), planning);
            }
        }
        UiAction::BuyXp => {
            if let Some(Applied::BoughtXp(levels)) = report(planning, lobby.act(Command::BuyXp)) {
                planning.level_up(&levels);
            }
        }
        UiAction::DismissNotice => planning.notice = None,
        UiAction::Reroll => {
            report(planning, lobby.act(Command::Reroll));
        }
        UiAction::Ready => {
            if report(planning, lobby.start_round()).is_some()
                && let Some((setup, _, opponent)) = lobby.battle_for(crate::state::ME)
            {
                commands.insert_resource(Playback::new(setup.clone(), false, Some(opponent)));
                // The battle plays on the field: dock the program panel and
                // close any leftover result pop-up.
                planning.editor_open = false;
                planning.result = None;
                if let Some(next) = next_phase {
                    next.set(RunPhase::Battle);
                }
            }
        }
        UiAction::SelectUnit(loc) => {
            let picked = lobby.me().unit_at(loc).map(|u| u.id);
            if planning.selected != picked {
                planning.selected = picked;
                editor::load_draft(lobby.me(), planning);
            }
            // Picking a unit brings up its program.
            planning.editor_open = true;
        }
        UiAction::ClickCell(p) => click_spot(lobby, planning, Location::Board(p)),
        UiAction::ClickBench(i) => click_spot(lobby, planning, Location::Bench(i)),
        UiAction::Sell => {
            if let Some(loc) = selected_loc(lobby.me(), planning) {
                sell(lobby, planning, loc);
            }
        }
        UiAction::SellAt(loc) => sell(lobby, planning, loc),
        UiAction::PaletteBlock(block) => editor::click_block(lobby, planning, block),
        UiAction::CycleSlot(card, slot) => editor::cycle_slot(lobby, planning, card, slot),
        UiAction::ClearCard(card) => editor::clear_card(lobby, planning, card),
        UiAction::RemoveBlock(card, slot) => editor::remove_block(lobby, planning, card, slot),
        UiAction::CardUp(card) => editor::card_up(lobby, planning, card),
        UiAction::Drop(source, target) => match (source, target) {
            (DragSource::Unit(from), DropTarget::Cell(p) | DropTarget::Field(p)) => {
                let to = Location::Board(p);
                if report(planning, lobby.act(Command::Move { from, to })).is_some() {
                    planning.selected = lobby.me().unit_at(to).map(|u| u.id);
                }
            }
            (DragSource::Unit(from), DropTarget::Bench(i)) => {
                report(planning, lobby.act(Command::Move { from, to: Location::Bench(i) }));
            }
            (DragSource::Unit(from), DropTarget::Sell) => sell(lobby, planning, from),
            (DragSource::Block(block), DropTarget::Card(card)) => {
                editor::drop_block(lobby, planning, card, block)
            }
            (DragSource::Placed(from, slot), DropTarget::Card(to)) => {
                editor::move_block(lobby, planning, from, slot, to)
            }
            (DragSource::Stash(item), target) => {
                if let Some((to, slot)) = item_target(lobby.me(), planning, target) {
                    report(planning, lobby.act(Command::Equip { item, to, slot }));
                }
            }
            (DragSource::Equipped(from, slot), target) => {
                match item_target(lobby.me(), planning, target) {
                    Some((to, to_slot)) => move_item(lobby, planning, from, slot, to, to_slot),
                    // Dropped on empty ground: the drag counted it as landing on
                    // the board, so put it back in the stash here.
                    None if matches!(
                        target,
                        DropTarget::Cell(_) | DropTarget::Field(_) | DropTarget::Bench(_)
                    ) =>
                    {
                        report(planning, lobby.act(Command::Unequip { from, slot }));
                    }
                    // Anywhere else, the end of the drag puts it back.
                    None => {}
                }
            }
            // Units dropped on program cards or item spots, blocks dropped on
            // anything but a card: nothing to do.
            _ => {}
        },
        UiAction::EquipStash(item) => {
            if let Some(to) = selected_loc(lobby.me(), planning) {
                report(planning, lobby.act(Command::Equip { item, to, slot: None }));
            }
        }
        UiAction::Unequip(from, slot) => {
            report(planning, lobby.act(Command::Unequip { from, slot }));
        }
        _ => {}
    }
    // Keep the selection pointed at a unit that still exists (merges remove copies).
    if planning.selected.is_some() && selected_loc(lobby.me(), planning).is_none() {
        planning.selected = None;
        planning.draft.clear();
    }
}

/// The unit (and slot, for the program pane's slots) an item dropped on
/// `target` would go to, if there is a unit there.
fn item_target(
    me: &Player,
    planning: &Planning,
    target: DropTarget,
) -> Option<(Location, Option<usize>)> {
    let (loc, slot) = match target {
        DropTarget::ItemSlot(s) => (selected_loc(me, planning)?, Some(s)),
        DropTarget::Cell(p) | DropTarget::Field(p) => (Location::Board(p), None),
        DropTarget::Bench(b) => (Location::Bench(b), None),
        _ => return None,
    };
    me.unit_at(loc).map(|_| (loc, slot))
}

/// Moves an equipped item to another unit (or slot), as unequip-then-equip
/// commands. Between two slots of the same unit the items swap; if the other
/// unit has no room, the item goes back where it was.
fn move_item(
    lobby: &mut Lobby,
    planning: &mut Planning,
    from: Location,
    slot: usize,
    to: Location,
    to_slot: Option<usize>,
) {
    let last = |lobby: &Lobby| lobby.me().stash().len().saturating_sub(1);
    if from == to {
        let Some(t) = to_slot.filter(|&t| t != slot) else { return };
        let other = lobby.me().unit_at(from).and_then(|u| u.items[t]);
        // Both go to the end of the stash (this one, then the other), then
        // come back in swapped slots.
        if report(planning, lobby.act(Command::Unequip { from, slot })).is_none() {
            return;
        }
        if other.is_some() {
            let _ = lobby.act(Command::Unequip { from, slot: t });
        }
        let first = last(lobby) - usize::from(other.is_some());
        let _ = lobby.act(Command::Equip { item: first, to: from, slot: Some(t) });
        if other.is_some() {
            let _ = lobby.act(Command::Equip { item: last(lobby), to: from, slot: Some(slot) });
        }
        return;
    }
    if report(planning, lobby.act(Command::Unequip { from, slot })).is_none() {
        return;
    }
    let item = last(lobby);
    if report(planning, lobby.act(Command::Equip { item, to, slot: to_slot })).is_none() {
        let _ = lobby.act(Command::Equip { item, to: from, slot: Some(slot) });
    }
}

fn sell(lobby: &mut Lobby, planning: &mut Planning, loc: Location) {
    let sold = lobby.me().unit_at(loc).map(|u| u.id);
    if report(planning, lobby.act(Command::Sell(loc))).is_some() && planning.selected == sold {
        planning.selected = None;
        planning.draft.clear();
    }
}

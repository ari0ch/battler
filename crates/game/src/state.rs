//! Scene states and the resources shared between screens.

use battler_sim::run::{OwnedId, RoundReport, Run};
use battler_sim::{Action, Condition};
use bevy::prelude::*;

/// Top-level scenes.
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    InRun,
    GameOver,
    /// Watching a past battle from the run summary.
    Replay,
}

/// The round loop, which only exists while a run is in progress.
#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InRun)]
pub enum RunPhase {
    #[default]
    Planning,
    Battle,
    Result,
}

/// The run in progress (kept through game over for the summary).
#[derive(Resource)]
pub struct GameRun(pub Run);

/// The most recent round's result.
#[derive(Resource)]
pub struct LastReport(pub RoundReport);

/// Bumped whenever something visible changes, so the current screen rebuilds.
#[derive(Resource, Default)]
pub struct Rev(pub u64);

impl Rev {
    pub fn bump(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// Player preferences that persist between battles.
#[derive(Resource)]
pub struct Settings {
    pub speed: u32,
    pub card_trace: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { speed: 1, card_trace: false }
    }
}

/// Main menu UI state.
#[derive(Resource, Default)]
pub struct MenuUi {
    pub how_to: bool,
}

/// One card being edited. An empty condition means "always".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DraftCard {
    pub cond: Option<Condition>,
    pub act: Option<Action>,
}

/// Planning screen UI state.
#[derive(Resource, Default)]
pub struct Planning {
    pub selected: Option<OwnedId>,
    pub editor_open: bool,
    /// Seconds left on the planning timer.
    pub time_left: f32,
    /// A short message (usually why something was refused) and seconds left to show it.
    pub toast: Option<(String, f32)>,
    /// The selected unit's cards while the editor is open, including half-built ones.
    pub draft: Vec<DraftCard>,
}

impl Planning {
    pub fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), 2.5));
    }
}

//! Scene states and the resources shared between screens.

use battler_sim::run::{Applied, Command, Lobby, OwnedId, Player, PlayerId, RoundReport, RunError};
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

/// The round loop, which only exists while a run is in progress. Both phases
/// share one screen: the battle plays on the planning screen's field, and the
/// round's result is a pop-up over the next planning phase.
#[derive(SubStates, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[source(AppState = AppState::InRun)]
pub enum RunPhase {
    #[default]
    Planning,
    Battle,
}

/// The lobby in progress (kept through game over for the summary). The human
/// is always player 0; the other seven are bots.
#[derive(Resource)]
pub struct GameRun(pub Lobby);

/// The human player's id in the lobby.
pub const ME: PlayerId = PlayerId(0);

/// Shortcuts for the human player.
///
/// This is an *extension trait*: Rust lets a crate add methods to a type from
/// another crate (here `Lobby` from `battler-sim`) by implementing its own trait
/// for it. The methods are only visible where the trait is imported.
pub trait LobbyExt {
    /// The human player.
    fn me(&self) -> &Player;
    /// Sends a command as the human player.
    fn act(&mut self, command: Command) -> Result<Applied, RunError>;
}

impl LobbyExt for Lobby {
    fn me(&self) -> &Player {
        self.player(ME)
    }

    fn act(&mut self, command: Command) -> Result<Applied, RunError> {
        self.apply(ME, command)
    }
}

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
    /// The players list on the right (toggled with its square button).
    pub show_players: bool,
    /// The damage and healing meters (toggled with their square button).
    pub show_meters: bool,
    pub meter_tab: MeterTab,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            speed: 1,
            card_trace: false,
            show_players: true,
            show_meters: false,
            meter_tab: MeterTab::Damage,
        }
    }
}

/// Which total the battle meters rank units by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeterTab {
    Damage,
    Healing,
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

/// Seconds a level-up notice stays up.
pub const NOTICE_SECS: f32 = 5.0;

/// Planning screen UI state.
#[derive(Resource, Default)]
pub struct Planning {
    pub selected: Option<OwnedId>,
    /// A level-up notice: the level reached and seconds left to show it.
    pub notice: Option<(u32, f32)>,
    /// Seconds left on the planning timer.
    pub time_left: f32,
    /// A short message (usually why something was refused) and seconds left to show it.
    pub toast: Option<(String, f32)>,
    /// The selected unit's cards while the editor is open, including half-built ones.
    pub draft: Vec<DraftCard>,
    /// Whether the program panel is open over the field, or docked to a tab.
    pub editor_open: bool,
    /// The last round's result, shown as a pop-up, and seconds left to show it.
    pub result: Option<(RoundReport, f32)>,
}

/// Seconds the round result pop-up stays up unless dismissed.
pub const RESULT_SECS: f32 = 6.0;

impl Planning {
    pub fn toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), 2.5));
    }

    /// Shows a round's result as a pop-up.
    pub fn show_result(&mut self, report: RoundReport) {
        self.result = Some((report, RESULT_SECS));
    }

    /// Shows a notice for the highest level reached, if any.
    pub fn level_up(&mut self, levels: &[u32]) {
        if let Some(&level) = levels.last() {
            self.notice = Some((level, NOTICE_SECS));
        }
    }
}

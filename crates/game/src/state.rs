//! Scene states and the resources shared between screens.

use battler_sim::run::{Applied, Command, Lobby, OwnedId, Player, PlayerId, RoundReport, RunError};
use battler_sim::{Action, Condition};
use bevy::prelude::*;

/// Top-level scenes.
#[derive(States, Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    MainMenu,
    /// Waiting with other players for the host to start an online game.
    Room,
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

/// The lobby in progress (kept through game over for the summary), and which
/// player in it is this one. Solo, that's player 0 among seven bots; online,
/// it's whichever seat the server gave us.
///
/// `GameRun` derefs to the [`Lobby`], so screens read it like a lobby
/// (`run.round()`, `run.players()`). It deliberately does *not* deref mutably:
/// every change goes through [`GameRun::act`], which also sends it to the
/// server when playing online.
#[derive(Resource)]
pub struct GameRun {
    /// What the screens show. Online, that's the server's confirmed lobby plus
    /// this player's commands still on their way (see [`crate::net::Online`]).
    pub lobby: Lobby,
    pub my_id: PlayerId,
    /// Present when playing online.
    pub online: Option<crate::net::Online>,
}

/// `Deref` is the trait behind `*`. Implementing it lets a `&GameRun` be used
/// where a `&Lobby` is expected (Rust inserts the deref), and lets lobby
/// methods be called on a `GameRun` directly.
impl std::ops::Deref for GameRun {
    type Target = Lobby;
    fn deref(&self) -> &Lobby {
        &self.lobby
    }
}

impl GameRun {
    /// A solo run: player 0 against seven bots.
    pub fn solo(seed: u64) -> Self {
        Self { lobby: Lobby::new(seed, 1), my_id: PlayerId(0), online: None }
    }

    /// This player.
    pub fn me(&self) -> &Player {
        self.lobby.player(self.my_id)
    }

    /// Sends a command as this player. Online, it also goes to the server,
    /// which has the final say (a refusal rolls it back).
    pub fn act(&mut self, command: Command) -> Result<Applied, RunError> {
        let applied = self.lobby.apply(self.my_id, command.clone())?;
        if let Some(online) = &mut self.online {
            online.sent(command);
        }
        Ok(applied)
    }

    pub fn is_online(&self) -> bool {
        self.online.is_some()
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
    /// What's typed in the name and room code boxes.
    pub name: String,
    pub code: String,
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
    /// The unit `draft` was loaded from. Checked after every action, so the
    /// program pane can never show one unit's cards while another is selected.
    pub draft_owner: Option<OwnedId>,
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

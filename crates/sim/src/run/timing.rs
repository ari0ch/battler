//! How long things take in real time: the planning timer and battle playback.
//!
//! These belong to presentation rather than to the rules, but both the game
//! (to animate) and the multiplayer server (to run the shared round clock)
//! need the same numbers, so they live here, in milliseconds.

use super::{Lobby, Player};

/// Real time per battle tick at 1x speed.
pub const TICK_MS: u32 = 500;

/// The pause after a battle's last tick before the result shows.
pub const END_PAUSE_MS: u32 = 1200;

/// Planning time for a player: longer once there's more to manage.
pub fn planning_ms(player: &Player) -> u32 {
    let units = player.board().len() + player.bench().iter().flatten().count();
    if units >= 5 { 75_000 } else { 60_000 }
}

/// How long a battle of `ticks` ticks takes to watch at 1x, end pause included.
pub fn battle_ms(ticks: u32) -> u32 {
    ticks * TICK_MS + END_PAUSE_MS
}

impl Lobby {
    /// The shared planning time in multiplayer: the longest any living player
    /// in `humans` would get on their own.
    pub fn shared_planning_ms(&self, humans: &[super::PlayerId]) -> u32 {
        humans
            .iter()
            .map(|&id| self.player(id))
            .filter(|p| p.is_alive())
            .map(planning_ms)
            .max()
            .unwrap_or(60_000)
    }
}

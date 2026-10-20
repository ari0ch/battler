//! The lobby: eight players, one shared pool, rounds until one player is left.
//!
//! A round goes:
//! 1. **Planning.** Bots plan first, then the human. Every change to a player
//!    goes through [`Lobby::apply`] with a [`Command`].
//! 2. [`Lobby::start_round`] locks in this round's battles: everyone against
//!    the same monsters on PvE rounds, otherwise random pairs (plus one ghost
//!    battle when an odd number of players is left).
//! 3. [`Lobby::resolve`] plays every battle, applies damage and income, knocks
//!    out players at 0 HP, refreshes the shops and starts the next planning phase.

use super::bot::Bot;
use super::economy::*;
use super::player::{Player, RoundOutcome};
use super::pool::Pool;
use super::{BattleSetup, Location, Opponent, OwnedId, PlayerId, RunError, RunStatus};
use crate::monster::mirror;
use crate::{Character, Item, Monster, Outcome, Pos, Program, Rarity, Rng, Team, UnitSpec, Wave};

/// What kind of round a round number is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundKind {
    /// Everyone fights the same monsters.
    Pve(Wave),
    /// Players fight each other.
    Versus,
}

/// Monster rounds 1 to 3, then a boss every [`BOSS_EVERY`] rounds from
/// [`FIRST_BOSS_ROUND`], cycling through the bosses and growing stronger each
/// cycle. Every other round is player against player.
pub fn round_kind(round: u32) -> RoundKind {
    const BOSSES: [Monster; 3] = [Monster::OgreChieftain, Monster::FrostWyrm, Monster::Lich];
    match round {
        1 => RoundKind::Pve(Wave::Slimes),
        2 => RoundKind::Pve(Wave::Wolves),
        3 => RoundKind::Pve(Wave::Goblins),
        r if r >= FIRST_BOSS_ROUND && (r - FIRST_BOSS_ROUND).is_multiple_of(BOSS_EVERY) => {
            let n = ((r - FIRST_BOSS_ROUND) / BOSS_EVERY) as usize;
            let boss = BOSSES[n % BOSSES.len()];
            RoundKind::Pve(Wave::Boss { boss, tier: (n / BOSSES.len()) as u32 })
        }
        _ => RoundKind::Versus,
    }
}

/// A change a player wants to make during planning. Humans and bots both send
/// these; nothing else changes a player's board.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Command {
    Buy(usize),
    Sell(Location),
    Reroll,
    BuyXp,
    /// Move a unit; moving onto an occupied spot swaps the two.
    Move {
        from: Location,
        to: Location,
    },
    SetProgram(Location, Program),
    /// Equip stash item `item` on the unit at `to`, into `slot` (swapping out
    /// what's there) or the first free slot.
    Equip {
        item: usize,
        to: Location,
        slot: Option<usize>,
    },
    /// Put the item in `slot` of the unit at `from` back in the stash.
    Unequip {
        from: Location,
        slot: usize,
    },
    /// Lock (or unlock) the shop: a locked shop keeps its offers into the
    /// next round instead of being refreshed.
    LockShop(bool),
}

/// What a successful [`Command`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Applied {
    /// The unit bought (or the unit it merged into).
    Bought(OwnedId),
    /// Gold refunded.
    Sold(u32),
    Rerolled,
    /// Levels reached, if any.
    BoughtXp(Vec<u32>),
    Moved,
    ProgramSet,
    Equipped,
    Unequipped,
    /// The shop's lock is now on (true) or off.
    ShopLocked(bool),
}

/// Who sits in one seat of a lobby: a named human, or a bot.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Seat {
    pub name: String,
    pub bot: bool,
}

impl Seat {
    pub fn human(name: impl Into<String>) -> Self {
        Self { name: name.into(), bot: false }
    }
}

/// One battle of the round. `home` fights on the left.
#[derive(Debug, Clone)]
struct Matchup {
    home: PlayerId,
    opponent: Opponent,
    setup: BattleSetup,
}

const BOT_NAMES: [&str; 7] = ["Mossa", "Grimble", "Tarrow", "Ylva", "Brisk", "Odo", "Fennick"];

#[derive(Debug, Clone)]
pub struct Lobby {
    seed: u64,
    rng: Rng,
    round: u32,
    pool: Pool,
    players: Vec<Player>,
    /// The bot controlling each player, or `None` for a human.
    bots: Vec<Option<Bot>>,
    /// This round's battles, once locked in.
    matchups: Option<Vec<Matchup>>,
    /// Each player's previous versus opponent, to avoid instant rematches.
    last_opponent: Vec<Option<PlayerId>>,
}

impl Lobby {
    /// A lobby of [`LOBBY_SIZE`] players: `humans` humans (ids from 0, named
    /// "You" for the first) and bots for the rest. Bots plan straight away.
    pub fn new(seed: u64, humans: usize) -> Self {
        let seats: Vec<Seat> =
            (0..humans).map(|i| Seat::human(if i == 0 { "You" } else { "Player" })).collect();
        Self::with_seats(seed, &seats)
    }

    /// A lobby whose first seats are `seats` (in player id order); any seats
    /// left over, up to [`LOBBY_SIZE`], are filled with named bots.
    ///
    /// Everything that follows depends only on the seed and the seats, which
    /// is what lets every client of a multiplayer game build the same lobby.
    ///
    /// # Panics
    /// If there are more than [`LOBBY_SIZE`] seats.
    pub fn with_seats(seed: u64, seats: &[Seat]) -> Self {
        assert!(seats.len() <= LOBBY_SIZE, "at most {LOBBY_SIZE} seats");
        let mut rng = Rng::new(seed);
        let mut pool = Pool::new();
        let mut players = Vec::new();
        let mut bots = Vec::new();
        for i in 0..LOBBY_SIZE {
            let (name, bot) = match seats.get(i) {
                Some(seat) => (seat.name.clone(), seat.bot),
                None => (BOT_NAMES[(i - seats.len()) % BOT_NAMES.len()].to_string(), true),
            };
            players.push(Player::new(PlayerId(i), name, rng.next_u64(), &mut pool));
            // `bool::then` runs the closure (and so draws a seed) only for bots.
            bots.push(bot.then(|| Bot::new(rng.next_u64())));
        }
        let mut lobby = Self {
            seed,
            rng,
            round: 1,
            pool,
            players,
            bots,
            matchups: None,
            last_opponent: vec![None; LOBBY_SIZE],
        };
        lobby.begin_planning();
        lobby
    }

    /// Hands a seat to a bot (a human who left) or back to its human.
    ///
    /// A bot that takes over during planning plans its turn straight away.
    /// Its seed comes from the lobby seed and the seat, not from the lobby's
    /// RNG, so the takeover never changes anything else in the game.
    pub fn set_bot(&mut self, id: PlayerId, bot: bool) {
        match (bot, self.bots[id.0].is_some()) {
            (true, false) => {
                let salt = 0x9e37_79b9_7f4a_7c15_u64.wrapping_mul(id.0 as u64 + 1);
                let mut bot = Bot::new(Rng::new(self.seed ^ salt).next_u64());
                if !self.in_battle() && !self.is_over() && self.players[id.0].is_alive() {
                    bot.plan(self, id);
                }
                self.bots[id.0] = Some(bot);
            }
            (false, true) => self.bots[id.0] = None,
            _ => {}
        }
    }

    // ----- Reading state -------------------------------------------------

    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn round(&self) -> u32 {
        self.round
    }
    pub fn round_kind(&self) -> RoundKind {
        round_kind(self.round)
    }
    pub fn players(&self) -> &[Player] {
        &self.players
    }
    pub fn player(&self, id: PlayerId) -> &Player {
        &self.players[id.0]
    }
    pub fn is_bot(&self, id: PlayerId) -> bool {
        self.bots[id.0].is_some()
    }
    /// The longest battle of the locked-in round, in ticks, or `None` before
    /// [`Lobby::start_round`]. Battles are deterministic, so playing each one
    /// out here gives exactly the length every client will see.
    pub fn longest_battle_ticks(&self) -> Option<u32> {
        let matchups = self.matchups.as_ref()?;
        Some(
            matchups
                .iter()
                .map(|m| {
                    let mut battle = m.setup.build();
                    battle.run_to_end();
                    battle.tick()
                })
                .max()
                .unwrap_or(0),
        )
    }
    /// A fingerprint of the whole lobby state, for checking that two copies of
    /// a lobby (the server's and a client's) still agree.
    ///
    /// It hashes the `Debug` text of the lobby with 64-bit FNV-1a. That text
    /// covers every field, and it is stable because the sim never stores
    /// hash-ordered collections. Writing straight into the hasher through
    /// [`std::fmt::Write`] avoids building the (long) string first.
    pub fn state_hash(&self) -> u64 {
        use std::fmt::Write;
        struct Fnv(u64);
        impl Write for Fnv {
            fn write_str(&mut self, s: &str) -> std::fmt::Result {
                for byte in s.bytes() {
                    self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
                }
                Ok(())
            }
        }
        let mut fnv = Fnv(0xcbf2_9ce4_8422_2325);
        write!(fnv, "{self:?}").expect("writing to a hasher can't fail");
        fnv.0
    }
    /// Copies of a character left in the shared pool.
    pub fn pool_remaining(&self, character: Character) -> u32 {
        self.pool.remaining(character)
    }
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
    pub fn alive_count(&self) -> usize {
        self.players.iter().filter(|p| p.is_alive()).count()
    }
    /// True once at most one player is left.
    pub fn is_over(&self) -> bool {
        self.players.iter().any(|p| p.status() == RunStatus::Victory)
    }
    /// True between [`Lobby::start_round`] and [`Lobby::resolve`].
    pub fn in_battle(&self) -> bool {
        self.matchups.is_some()
    }

    /// The battle a player is in this round (after [`Lobby::start_round`]),
    /// the side they're on, and who they're fighting.
    pub fn battle_for(&self, id: PlayerId) -> Option<(&BattleSetup, Team, Opponent)> {
        self.matchups.as_ref()?.iter().find_map(|m| {
            if m.home == id {
                Some((&m.setup, Team::Player, m.opponent))
            } else if m.opponent == Opponent::Player(id) {
                Some((&m.setup, Team::Enemy, Opponent::Player(m.home)))
            } else {
                None
            }
        })
    }

    // ----- Commands ------------------------------------------------------

    /// Applies one player's command. The only way a player's board changes
    /// during planning, for humans and bots alike.
    pub fn apply(&mut self, id: PlayerId, command: Command) -> Result<Applied, RunError> {
        if self.matchups.is_some() {
            return Err(RunError::BattleInProgress);
        }
        let player = &mut self.players[id.0];
        if !player.is_alive() {
            return Err(RunError::RunOver);
        }
        // `player` borrows `self.players`, and `&mut self.pool` borrows a
        // different field, so both mutable borrows can live at once.
        let pool = &mut self.pool;
        Ok(match command {
            Command::Buy(slot) => Applied::Bought(player.buy(slot)?),
            Command::Sell(loc) => Applied::Sold(player.sell(loc, pool)?),
            Command::Reroll => {
                player.reroll(pool)?;
                Applied::Rerolled
            }
            Command::BuyXp => Applied::BoughtXp(player.buy_xp()?),
            Command::Move { from, to } => {
                player.move_unit(from, to)?;
                Applied::Moved
            }
            Command::SetProgram(loc, program) => {
                player.set_program(loc, program)?;
                Applied::ProgramSet
            }
            Command::Equip { item, to, slot } => {
                player.equip(item, to, slot)?;
                Applied::Equipped
            }
            Command::Unequip { from, slot } => {
                player.unequip(from, slot)?;
                Applied::Unequipped
            }
            Command::LockShop(locked) => {
                player.set_shop_lock(locked);
                Applied::ShopLocked(locked)
            }
        })
    }

    // ----- Rounds --------------------------------------------------------

    /// Locks in this round's battles. Does nothing if they're already locked.
    pub fn start_round(&mut self) -> Result<(), RunError> {
        if self.is_over() {
            return Err(RunError::RunOver);
        }
        if self.matchups.is_some() {
            return Ok(());
        }
        let alive: Vec<PlayerId> =
            self.players.iter().filter(|p| p.is_alive()).map(Player::id).collect();
        let mut matchups = Vec::new();
        match self.round_kind() {
            RoundKind::Pve(wave) => {
                let enemy = mirrored(wave.units());
                for &home in &alive {
                    let setup = self.setup(self.player(home).board_specs(), enemy.clone());
                    matchups.push(Matchup { home, opponent: Opponent::Monsters(wave), setup });
                }
            }
            RoundKind::Versus => {
                let order = self.pairing(&alive);
                for pair in order.chunks(2) {
                    let (home, opponent, enemy) = match *pair {
                        [a, b] => {
                            let (home, away) = (a.min(b), a.max(b));
                            self.last_opponent[home.0] = Some(away);
                            self.last_opponent[away.0] = Some(home);
                            (home, Opponent::Player(away), self.player(away).board_specs())
                        }
                        // Odd one out: a copy of another living player's board.
                        [a] => {
                            let others: Vec<PlayerId> =
                                alive.iter().copied().filter(|&p| p != a).collect();
                            let of = others[self.rng.below(others.len() as u32) as usize];
                            (a, Opponent::Ghost(of), self.player(of).board_specs())
                        }
                        _ => unreachable!("chunks(2) yields one or two"),
                    };
                    let setup = self.setup(self.player(home).board_specs(), mirrored(enemy));
                    matchups.push(Matchup { home, opponent, setup });
                }
            }
        }
        self.matchups = Some(matchups);
        Ok(())
    }

    /// Plays every battle of the round and moves on to the next planning phase.
    ///
    /// The battles are re-simulated from their setups, so the result always
    /// matches what was shown on screen (battles are deterministic).
    ///
    /// # Panics
    /// If [`Lobby::start_round`] wasn't called first.
    pub fn resolve(&mut self) {
        let matchups = self.matchups.take().expect("resolve() without start_round()");
        let round = self.round;
        let alive_before = self.alive_count();
        // (player, HP after damage, possibly below zero) for elimination order.
        let mut finishing: Vec<(PlayerId, i64)> = Vec::new();

        for m in matchups {
            let mut battle = m.setup.build();
            let outcome = battle.run_to_end();
            let alive_on =
                |team| battle.units().iter().filter(move |u| u.is_alive() && u.team == team);
            let levels = |team| alive_on(team).map(|u| u.level.number()).sum::<u32>();
            let result_for = |team: Team| match outcome {
                Outcome::Won(t) => Some(t == team),
                _ => None,
            };
            let mut sides = vec![(m.home, Team::Player, m.opponent)];
            if let Opponent::Player(away) = m.opponent {
                sides.push((away, Team::Enemy, Opponent::Player(m.home)));
            }
            for (id, side, opponent) in sides {
                let result = result_for(side);
                let lost = result == Some(false);
                let won = result == Some(true);
                let (damage, reward) = match opponent {
                    Opponent::Monsters(wave) => (
                        if lost { pve_loss_damage(wave.is_boss(), round) } else { 0 },
                        if won { pve_reward(wave.is_boss()) } else { 0 },
                    ),
                    Opponent::Ghost(_) => (0, if won { WIN_BONUS } else { 0 }),
                    Opponent::Player(_) => (
                        if lost { loss_damage(round, levels(side.opponent())) } else { 0 },
                        if won { WIN_BONUS } else { 0 },
                    ),
                };
                let loot = match opponent {
                    Opponent::Monsters(wave) => loot(wave, result, &mut self.rng),
                    _ => Vec::new(),
                };
                let player = &mut self.players[id.0];
                finishing.push((id, i64::from(player.hp()) - i64::from(damage)));
                let survivors = alive_on(side).count();
                let ticks = battle.tick();
                let outcome = RoundOutcome {
                    round,
                    opponent,
                    side,
                    result,
                    ticks,
                    survivors,
                    damage,
                    reward,
                    loot,
                };
                player.settle(outcome, m.setup.clone());
            }
        }

        // Knock out everyone at 0 HP: the lowest (most overkilled) places last.
        let mut out: Vec<(PlayerId, i64)> =
            finishing.into_iter().filter(|&(id, _)| self.player(id).hp() == 0).collect();
        out.sort_by_key(|&(id, hp)| (hp, std::cmp::Reverse(id)));
        for (n, &(id, _)) in out.iter().enumerate() {
            let placement = (alive_before - n) as u32;
            if placement == 1 {
                // Everyone left fell in the same round: the least overkilled wins.
                self.players[id.0].win();
            } else {
                self.players[id.0].eliminate(placement, &mut self.pool);
            }
        }
        if self.alive_count() == 1 {
            let last = self.players.iter_mut().find(|p| p.is_alive()).expect("one left");
            last.win();
        }
        if self.is_over() {
            return;
        }

        self.round += 1;
        for player in self.players.iter_mut().filter(|p| p.is_alive()) {
            player.new_round_shop(&mut self.pool);
        }
        self.begin_planning();
    }

    /// Lets each bot plan its turn, in player order.
    fn begin_planning(&mut self) {
        for i in 0..self.players.len() {
            if !self.players[i].is_alive() {
                continue;
            }
            // Take the bot out of the lobby while it plans: `plan` needs
            // `&mut Lobby` to send commands, which it couldn't have while the
            // bot itself was still borrowed from inside the lobby.
            if let Some(mut bot) = self.bots[i].take() {
                bot.plan(self, PlayerId(i));
                self.bots[i] = Some(bot);
            }
        }
    }

    fn setup(&mut self, player: Vec<(Pos, UnitSpec)>, enemy: Vec<(Pos, UnitSpec)>) -> BattleSetup {
        BattleSetup { round: self.round, seed: self.rng.next_u64(), player, enemy }
    }

    /// Random pairs of living players, avoiding last round's opponents when a
    /// few shuffles allow it.
    fn pairing(&mut self, alive: &[PlayerId]) -> Vec<PlayerId> {
        let mut order = alive.to_vec();
        for _ in 0..20 {
            self.rng.shuffle(&mut order);
            let rematch = order.chunks(2).any(|pair| match *pair {
                [a, b] => self.last_opponent[a.0] == Some(b),
                _ => false,
            });
            if !rematch {
                break;
            }
        }
        order
    }

    // ----- Test helpers --------------------------------------------------

    #[cfg(test)]
    pub(super) fn player_mut(&mut self, id: PlayerId) -> &mut Player {
        &mut self.players[id.0]
    }
}

/// What a PvE round drops for one player (see the Items section of the design
/// notes): the opening monster waves always drop two commons; a boss drops a
/// rare on a win, plus an epic from the second boss of the run on (the Ogre
/// Chieftain's first visit drops only the rare), or a common on a loss.
pub(super) fn loot(wave: Wave, result: Option<bool>, rng: &mut Rng) -> Vec<Item> {
    let first_boss = |w| matches!(w, Wave::Boss { boss: Monster::OgreChieftain, tier: 0 });
    let rarities: &[Rarity] = match (wave, result) {
        (Wave::Boss { .. }, Some(true)) if !first_boss(wave) => &[Rarity::Rare, Rarity::Epic],
        (Wave::Boss { .. }, Some(true)) => &[Rarity::Rare],
        (Wave::Boss { .. }, _) => &[Rarity::Common],
        _ => &[Rarity::Common, Rarity::Common],
    };
    rarities.iter().map(|&r| Item::roll(r, rng)).collect()
}

/// Moves a board from a player's half onto the right half of the battlefield.
fn mirrored(units: Vec<(Pos, UnitSpec)>) -> Vec<(Pos, UnitSpec)> {
    units.into_iter().map(|(p, s)| (mirror(p), s)).collect()
}

//! Balance harness: headless simulations that report how characters and
//! whole runs perform. Run with:
//!
//! ```sh
//! cargo run -p battler-sim --example balance --release
//! ```
//!
//! Optional arguments: number of team fights, number of lobbies (defaults 6000, 200).
//! Output is Markdown, ready to paste into `docs/DESIGN.md`.

use battler_sim::run::bot::preset_program;
use battler_sim::run::economy::{BENCH_SIZE, PLAYER_COLUMNS, XP_COST};
use battler_sim::run::{Command, Lobby, Location, Opponent, PlayerId};
use battler_sim::{
    Battle, Character, Class, Level, Outcome, Pos, Program, Rng, Synergies, Team, UnitSpec, Wave,
};

fn main() {
    let mut args = std::env::args().skip(1).map(|a| a.parse::<u64>().expect("a number"));
    let fights = args.next().unwrap_or(6000);
    let lobbies = args.next().unwrap_or(200);

    team_fights(fights);
    full_lobbies(lobbies);
}

// ----- Random team fights --------------------------------------------------

/// Places characters on one side: melee in the front columns, ranged at the back.
fn place(team: Team, characters: &[Character], level: Level) -> Vec<(Pos, UnitSpec)> {
    let rows = [2, 1, 3, 0, 4];
    let (front, back) = match team {
        Team::Player => (PLAYER_COLUMNS - 1, 0),
        Team::Enemy => (PLAYER_COLUMNS, 7),
    };
    let step = if front > back { -1 } else { 1 };
    let columns: Vec<i32> = (0..PLAYER_COLUMNS).map(|i| front + step * i).collect();
    let mut used = Vec::new();
    characters
        .iter()
        .map(|&c| {
            let order: Vec<i32> = if c.class().is_ranged() {
                columns.iter().rev().copied().collect()
            } else {
                columns.clone()
            };
            let pos = order
                .iter()
                .flat_map(|&x| rows.iter().map(move |&y| Pos::new(x, y)))
                .find(|p| !used.contains(p))
                .unwrap();
            used.push(pos);
            let mut nodes = preset_program(c);
            nodes.truncate(level.max_nodes());
            (pos, UnitSpec::new(c, level, Program::new(nodes)))
        })
        .collect()
}

fn random_team(size: usize, rng: &mut Rng) -> Vec<Character> {
    let mut all = Character::ALL;
    rng.shuffle(&mut all);
    all[..size].to_vec()
}

fn team_fights(fights: u64) {
    let mut games = [0u32; Character::COUNT];
    let mut wins = [0u32; Character::COUNT];
    let (mut syn_games, mut syn_wins) = (0u32, 0u32);
    let (mut draws, mut ticks) = (0u32, 0u64);
    let mut rng = Rng::new(2026);

    for seed in 0..fights {
        let size = 3 + (seed % 4) as usize; // 3 to 6 a side
        let level = if seed % 3 == 0 { Level::Two } else { Level::One };
        let a = random_team(size, &mut rng);
        let b = random_team(size, &mut rng);
        let mut battle = Battle::new(8, 5, 200, seed);
        for (team, chars) in [(Team::Player, &a), (Team::Enemy, &b)] {
            for (pos, spec) in place(team, chars, level) {
                battle.spawn(team, pos, spec).unwrap();
            }
        }
        let outcome = battle.run_to_end();
        ticks += u64::from(battle.tick());
        for (team, chars) in [(Team::Player, &a), (Team::Enemy, &b)] {
            let won = outcome == Outcome::Won(team);
            for c in chars {
                games[c.index()] += 1;
                wins[c.index()] += u32::from(won);
            }
            let syn = Synergies::from_characters(chars.iter().copied());
            let other = if team == Team::Player { &b } else { &a };
            let other_syn = Synergies::from_characters(other.iter().copied());
            if syn.active_count() > other_syn.active_count() {
                syn_games += 1;
                syn_wins += u32::from(won);
            }
        }
        draws += u32::from(outcome == Outcome::Draw);
    }

    println!("## Character win rates ({fights} random team fights, 3-6 a side, level 1-2)\n");
    println!("| Character | Class | Rarity | Win rate | Games |");
    println!("|---|---|---|---|---|");
    let mut rows: Vec<(f64, Character)> = Character::ALL
        .iter()
        .map(|&c| (f64::from(wins[c.index()]) / f64::from(games[c.index()].max(1)), c))
        .collect();
    rows.sort_by(|x, y| y.0.total_cmp(&x.0));
    for (rate, c) in rows {
        println!(
            "| {} | {} | {} | {:.0}% | {} |",
            c.name(),
            c.class().name(),
            c.rarity().name(),
            rate * 100.0,
            games[c.index()]
        );
    }
    println!();
    println!("| Class | Win rate |");
    println!("|---|---|");
    for class in Class::ALL {
        let (g, w): (u32, u32) = Character::ALL
            .iter()
            .filter(|c| c.class() == class)
            .fold((0, 0), |(g, w), c| (g + games[c.index()], w + wins[c.index()]));
        println!("| {} | {:.0}% |", class.name(), 100.0 * f64::from(w) / f64::from(g.max(1)));
    }
    println!();
    println!(
        "Teams with more active synergies than their opponent won {:.0}% of {} such fights.",
        100.0 * f64::from(syn_wins) / f64::from(syn_games.max(1)),
        syn_games
    );
    println!(
        "Draws: {:.1}%. Average battle length: {:.1} ticks.\n",
        100.0 * f64::from(draws) / fights as f64,
        ticks as f64 / fights as f64
    );
}

// ----- Full lobbies -----------------------------------------------------------

const ME: PlayerId = PlayerId(0);

/// A naive player in the human's seat: buys anything affordable, buys XP with
/// spare gold, fills the board and uses the preset programs. Roughly a new
/// player who doesn't plan merges or synergies.
fn naive_turn(lobby: &mut Lobby) {
    loop {
        let p = lobby.player(ME);
        let keep = if lobby.round() > 3 { 10 } else { 0 };
        let affordable =
            p.shop().iter().position(|o| o.is_some_and(|c| c.rarity().cost() + keep <= p.gold()));
        let Some(slot) = affordable else { break };
        if lobby.apply(ME, Command::Buy(slot)).is_err() {
            break;
        }
    }
    while lobby.player(ME).gold() >= XP_COST + 20 && lobby.apply(ME, Command::BuyXp).is_ok() {}
    for i in 0..BENCH_SIZE {
        let p = lobby.player(ME);
        let Some(unit) = p.bench()[i].clone() else { continue };
        if p.board().len() >= p.board_limit()
            || p.board().iter().any(|(_, b)| b.character == unit.character)
        {
            continue;
        }
        if let Some(to) = p.suggested_spot(unit.character) {
            let from = Location::Bench(i);
            let _ = lobby.apply(ME, Command::Move { from, to: Location::Board(to) });
        }
    }
    let p = lobby.player(ME);
    let programs: Vec<(Location, Program)> = p
        .board()
        .iter()
        .map(|(pos, u)| {
            let mut nodes = preset_program(u.character);
            nodes.truncate(p.synergies().card_slots(u.character, u.level));
            (Location::Board(*pos), Program::new(nodes))
        })
        .collect();
    for (loc, program) in programs {
        let _ = lobby.apply(ME, Command::SetProgram(loc, program));
    }
}

/// Wins and fights per PvE round label.
#[derive(Default)]
struct Tally {
    rows: Vec<(String, u32, u32)>,
}

impl Tally {
    fn add(&mut self, label: String, won: bool) {
        match self.rows.iter_mut().find(|(l, _, _)| *l == label) {
            Some(row) => {
                row.1 += u32::from(won);
                row.2 += 1;
            }
            None => self.rows.push((label, u32::from(won), 1)),
        }
    }
}

/// Screen time of one round in seconds, with every timer running out: the
/// planning timer (`planning_seconds` in the game crate), the battle at 0.5 s a
/// tick plus a 1.2 s end pause, and the 10 s result screen. An upper bound:
/// players can press Ready and skip the result screen.
fn round_seconds(units_owned: usize, ticks: u32) -> f64 {
    let planning = if units_owned >= 5 { 75.0 } else { 60.0 };
    planning + f64::from(ticks) * 0.5 + 1.2 + 10.0
}

fn full_lobbies(lobbies: u64) {
    let mut rounds = Vec::new();
    // Minutes the winner spends in the lobby, and the average over all players.
    let (mut winner_minutes, mut player_minutes) = (Vec::new(), Vec::new());
    let mut naive_placements = Vec::new();
    let mut pve = Tally::default();
    let mut levels_at = [(7u32, 0u32, 0u32), (11, 0, 0), (15, 0, 0)];
    for seed in 0..lobbies {
        let mut lobby = Lobby::new(seed, 1);
        let mut seconds = [0.0f64; 8];
        while !lobby.is_over() {
            let owned: Vec<(bool, usize)> = lobby
                .players()
                .iter()
                .map(|p| (p.is_alive(), p.board().len() + p.bench().iter().flatten().count()))
                .collect();
            if lobby.player(ME).is_alive() {
                naive_turn(&mut lobby);
            }
            for row in levels_at.iter_mut().filter(|r| r.0 == lobby.round()) {
                for p in lobby.players().iter().filter(|p| p.is_alive()) {
                    row.1 += p.level();
                    row.2 += 1;
                }
            }
            lobby.start_round().unwrap();
            lobby.resolve();
            for (i, p) in lobby.players().iter().enumerate() {
                if let (true, units) = owned[i]
                    && let Some((_, r)) = p.history().last()
                {
                    seconds[i] += round_seconds(units, r.ticks);
                }
            }
            assert!(lobby.round() < 200, "lobby {seed} never ended");
        }
        rounds.push(lobby.round());
        let winner = lobby.players().iter().position(|p| p.placement() == Some(1)).unwrap();
        winner_minutes.push(seconds[winner] / 60.0);
        player_minutes.extend(seconds.iter().map(|s| s / 60.0));
        naive_placements.push(lobby.player(ME).placement().unwrap());
        for p in lobby.players().iter().skip(1) {
            for (_, r) in p.history() {
                if let Opponent::Monsters(wave) = r.opponent {
                    let label = match wave {
                        Wave::Boss { tier, .. } => {
                            format!("{} (round {}, tier {tier})", wave.name(), r.round)
                        }
                        _ => format!("{} (round {})", wave.name(), r.round),
                    };
                    pve.add(label, r.result == Some(true));
                }
            }
        }
    }
    let n = lobbies as f64;
    let avg = |v: &[u32]| v.iter().sum::<u32>() as f64 / v.len() as f64;
    println!("## Full lobbies ({lobbies} lobbies: 7 bots and a naive player)\n");
    println!("| Avg. rounds | Shortest | Longest | Naive player avg. placement | Naive top 4 |");
    println!("|---|---|---|---|---|");
    println!(
        "| {:.1} | {} | {} | {:.1} | {:.0}% |",
        avg(&rounds),
        rounds.iter().min().unwrap(),
        rounds.iter().max().unwrap(),
        avg(&naive_placements),
        100.0 * naive_placements.iter().filter(|&&p| p <= 4).count() as f64 / n
    );
    println!();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    println!(
        "Game length with every timer running out: the winner plays {:.1} minutes on \
         average; the average player {:.1} minutes.\n",
        mean(&winner_minutes),
        mean(&player_minutes)
    );
    println!(
        "Average bot-lobby player level: {}.\n",
        levels_at
            .iter()
            .map(|(r, sum, k)| format!(
                "round {r}: {:.1}",
                f64::from(*sum) / f64::from((*k).max(1))
            ))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("| PvE round (bots) | Win rate | Fights |");
    println!("|---|---|---|");
    pve.rows.sort_by_key(|(l, _, _)| {
        l.split("round ")
            .nth(1)
            .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|r| r.parse::<u32>().ok())
            .unwrap_or(0)
    });
    for (label, won, fights) in &pve.rows {
        if *fights >= 5 {
            println!(
                "| {label} | {:.0}% | {fights} |",
                100.0 * f64::from(*won) / f64::from(*fights)
            );
        }
    }
}

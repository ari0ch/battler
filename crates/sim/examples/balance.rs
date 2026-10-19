//! Balance harness: headless simulations that report how characters and
//! whole runs perform. Run with:
//!
//! ```sh
//! cargo run -p battler-sim --example balance --release
//! ```
//!
//! Optional arguments: number of team fights, number of runs (defaults 6000, 300).
//! Output is Markdown, ready to paste into `docs/DESIGN.md`.

use battler_sim::run::economy::{BENCH_SIZE, PLAYER_COLUMNS, XP_COST};
use battler_sim::run::{Location, Run, RunStatus, ai::preset_program};
use battler_sim::{
    Battle, Character, Class, Level, Outcome, Pos, Program, Rng, Synergies, Team, UnitSpec,
};

fn main() {
    let mut args = std::env::args().skip(1).map(|a| a.parse::<u64>().expect("a number"));
    let fights = args.next().unwrap_or(6000);
    let runs = args.next().unwrap_or(300);

    team_fights(fights);
    full_runs(runs);
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

// ----- Full runs with a simple bot -------------------------------------------

/// A naive bot: buys anything affordable, buys XP with spare gold, fills the
/// board with distinct characters and gives them the AI's preset programs.
fn bot_turn(run: &mut Run) {
    loop {
        let keep = if run.round() > 3 { 10 } else { 0 };
        let affordable = run
            .shop()
            .iter()
            .position(|o| o.is_some_and(|c| c.rarity().cost() + keep <= run.gold()));
        let Some(slot) = affordable else { break };
        if run.buy(slot).is_err() {
            break;
        }
    }
    while run.gold() >= XP_COST + 20 && run.buy_xp().is_ok() {}

    for i in 0..BENCH_SIZE {
        let Some(unit) = run.bench()[i].clone() else { continue };
        if run.board().len() >= run.board_limit()
            || run.board().iter().any(|(_, b)| b.character == unit.character)
        {
            continue;
        }
        let spot = (0..PLAYER_COLUMNS)
            .flat_map(|x| (0..5).map(move |y| Pos::new(x, y)))
            .find(|p| run.unit_at(Location::Board(*p)).is_none());
        if let Some(p) = spot {
            let _ = run.move_unit(Location::Bench(i), Location::Board(p));
        }
    }
    for (p, u) in run.board().to_vec() {
        let mut nodes = preset_program(u.character);
        nodes.truncate(run.synergies().card_slots(u.character, u.level));
        let _ = run.set_program(Location::Board(p), Program::new(nodes));
    }
}

fn full_runs(runs: u64) {
    let (mut victories, mut rounds, mut levels, mut wins) = (0u32, 0u32, 0u32, 0u32);
    for seed in 0..runs {
        let mut run = Run::new(seed);
        while run.status() == RunStatus::Planning {
            bot_turn(&mut run);
            let setup = run.start_battle().unwrap();
            let mut battle = setup.build();
            battle.run_to_end();
            wins += u32::from(battle.outcome() == Outcome::Won(Team::Player));
            run.resolve(&battle);
        }
        victories += u32::from(run.status() == RunStatus::Victory);
        rounds += run.round();
        levels += run.level();
    }
    let n = runs as f64;
    println!("## Full runs ({runs} runs, naive bot)\n");
    println!("| Victory rate | Avg. final round | Avg. final level | Avg. battles won |");
    println!("|---|---|---|---|");
    println!(
        "| {:.0}% | {:.1} | {:.1} | {:.1} |",
        100.0 * f64::from(victories) / n,
        f64::from(rounds) / n,
        f64::from(levels) / n,
        f64::from(wins) / n
    );
}

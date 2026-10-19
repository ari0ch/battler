use super::economy::*;
use super::*;
use crate::{Action, Condition, Node, Target};
use Character::*;

/// A run with plenty of gold and a shop full of one character.
fn run_with_shop(character: Character, gold: u32) -> Run {
    let mut run = Run::new(1);
    run.gold = gold;
    run.shop = [Some(character); SHOP_SIZE];
    run
}

fn restock(run: &mut Run, character: Character) {
    run.shop = [Some(character); SHOP_SIZE];
}

fn find(run: &Run, character: Character) -> Location {
    let id = run
        .board()
        .iter()
        .map(|(_, u)| u)
        .chain(run.bench().iter().flatten())
        .find(|u| u.character == character)
        .expect("owned")
        .id;
    run.locate(id).unwrap()
}

#[test]
fn new_run_starts_at_level_three_with_a_shop() {
    let run = Run::new(42);
    assert_eq!((run.round(), run.hp(), run.gold(), run.level()), (1, START_HP, START_GOLD, 3));
    assert_eq!(run.board_limit(), 3);
    assert!(run.shop().iter().all(Option::is_some));
    let drawn: u32 =
        Character::ALL.iter().map(|&c| c.rarity().pool_copies() - run.pool_remaining(c)).sum();
    assert_eq!(drawn, SHOP_SIZE as u32, "shop offers come out of the pool");
}

#[test]
fn level_three_shops_never_offer_epics() {
    let mut run = Run::new(5);
    run.gold = 1000;
    for _ in 0..100 {
        run.reroll().unwrap();
        assert!(run.shop().iter().flatten().all(|c| c.rarity() != Rarity::Epic));
    }
}

#[test]
fn same_seed_same_shop() {
    assert_eq!(Run::new(7).shop(), Run::new(7).shop());
}

#[test]
fn buying_costs_rarity_price_and_auto_places() {
    let mut run = run_with_shop(SirAldric, 10);
    run.buy(0).unwrap();
    assert_eq!(run.gold(), 10 - Rarity::Rare.cost());
    assert_eq!(run.shop()[0], None);
    assert_eq!(run.board().len(), 1, "room on the board: placed straight away");
    assert_eq!(run.board()[0].0.x, PLAYER_COLUMNS - 1, "melee goes to the front");
    assert_eq!(run.buy(0), Err(RunError::NoOffer(0)));

    restock(&mut run, PipQuickfeather);
    run.buy(0).unwrap();
    assert!(
        run.board().iter().any(|(p, u)| u.character == PipQuickfeather && p.x == 0),
        "ranged at the back"
    );
}

#[test]
fn a_second_copy_waits_on_the_bench() {
    let mut run = run_with_shop(Nettle, 10);
    run.buy(0).unwrap();
    run.buy(1).unwrap();
    assert_eq!(run.board().len(), 1);
    assert_eq!(run.bench().iter().flatten().count(), 1);
}

#[test]
fn buying_needs_enough_gold() {
    let mut run = run_with_shop(Ysolde, 2);
    assert_eq!(run.buy(0), Err(RunError::NotEnoughGold { need: 3, have: 2 }));
}

#[test]
fn three_copies_merge_and_keep_the_board_program() {
    let mut run = run_with_shop(SirAldric, 100);
    run.buy(0).unwrap();
    let spot = find(&run, SirAldric);
    let program = Program::new([Node::new(
        Condition::AllyHpBelow(crate::Threshold::P50),
        Action::ShieldWall,
    )]);
    run.set_program(spot, program.clone()).unwrap();
    run.buy(1).unwrap();
    run.buy(2).unwrap();
    let aldric = run.unit_at(spot).expect("merged unit stays on the board");
    assert_eq!(aldric.level, Level::Two);
    assert_eq!(aldric.program, program);
    assert!(run.bench().iter().all(Option::is_none));
}

#[test]
fn nine_copies_cascade_to_level_three() {
    let mut run = run_with_shop(Grub, 100);
    for _ in 0..9 {
        if run.shop.iter().all(Option::is_none) {
            restock(&mut run, Grub);
        }
        let slot = run.shop.iter().position(Option::is_some).unwrap();
        run.buy(slot).unwrap();
    }
    assert_eq!(run.copies_owned(Grub), 9);
    assert_eq!(run.unit_at(find(&run, Grub)).unwrap().level, Level::Three);
}

#[test]
fn a_full_bench_still_accepts_a_merging_copy() {
    let mut run = Run::new(5);
    run.gold = 100;
    run.level = MAX_LEVEL; // board space doesn't matter here
    let mut others: Vec<Character> =
        Character::ALL.iter().copied().filter(|&c| c != SisterAgnes && c != Tobble).collect();
    // Fill the board first so new buys stay on the bench.
    for _ in 0..MAX_LEVEL {
        let c = others.remove(0);
        restock(&mut run, c);
        run.buy(0).unwrap();
    }
    for c in [SisterAgnes, SisterAgnes].into_iter().chain(others.into_iter().take(7)) {
        restock(&mut run, c);
        run.buy(0).unwrap();
    }
    assert!(run.bench().iter().all(Option::is_some));
    restock(&mut run, Tobble);
    assert_eq!(run.buy(0), Err(RunError::BenchFull));
    restock(&mut run, SisterAgnes);
    run.buy(0).expect("third Agnes merges, so it fits");
    assert_eq!(run.bench().len(), BENCH_SIZE);
}

#[test]
fn selling_refunds_rarity_price_and_returns_copies() {
    let mut run = run_with_shop(Kestrel, 9);
    let before = run.pool_remaining(Kestrel);
    run.buy(0).unwrap();
    let refund = run.sell(find(&run, Kestrel)).unwrap();
    assert_eq!(refund, Rarity::Rare.cost());
    assert_eq!(run.gold(), 9);
    assert_eq!(run.pool_remaining(Kestrel), before + 1);
}

#[test]
fn reroll_costs_gold_and_returns_offers() {
    let mut run = Run::new(3);
    let in_pool = |r: &Run| Character::ALL.iter().map(|&c| r.pool_remaining(c)).sum::<u32>();
    let before = in_pool(&run);
    run.reroll().unwrap();
    assert_eq!(run.gold(), START_GOLD - REROLL_COST);
    assert_eq!(in_pool(&run), before);
}

#[test]
fn buying_xp_levels_up_and_grows_the_board() {
    let mut run = Run::new(4);
    run.gold = 100;
    assert_eq!(run.buy_xp(), Ok(vec![]));
    assert_eq!(run.xp(), 4);
    assert_eq!(run.buy_xp(), Ok(vec![4]), "6 XP for level 4");
    assert_eq!((run.level(), run.xp()), (4, 2));
    assert_eq!(run.board_limit(), 4);
    run.level = MAX_LEVEL;
    assert_eq!(run.buy_xp(), Err(RunError::MaxLevel));
}

#[test]
fn board_rules_are_enforced() {
    let mut run = Run::new(2);
    run.gold = 100;
    let b = |x, y| Location::Board(Pos::new(x, y));
    for c in [BramGatewarden, SirAldric, Tobble] {
        restock(&mut run, c);
        run.buy(0).unwrap();
    }
    assert_eq!(run.board().len(), 3, "level 3: three on the board");
    restock(&mut run, SisterAgnes);
    run.buy(0).unwrap();
    let agnes = find(&run, SisterAgnes);
    assert!(matches!(agnes, Location::Bench(_)), "board full: stays on the bench");
    assert_eq!(run.move_unit(agnes, b(0, 4)), Err(RunError::BoardFull { max: 3 }));
    assert_eq!(run.move_unit(agnes, b(5, 0)), Err(RunError::NotPlayerSide(Pos::new(5, 0))));

    // Swapping a bench unit with a board unit is fine even when the board is full.
    let tobble = find(&run, Tobble);
    run.move_unit(agnes, tobble).unwrap();
    assert_eq!(run.unit_at(tobble).unwrap().character, SisterAgnes);

    // Two different Knights are fine; the same character twice is not.
    restock(&mut run, BramGatewarden);
    run.buy(0).unwrap();
    let second_bram = Location::Bench(
        run.bench()
            .iter()
            .position(|s| s.as_ref().is_some_and(|u| u.character == BramGatewarden))
            .unwrap(),
    );
    let aldric = find(&run, SirAldric);
    assert_eq!(run.move_unit(second_bram, aldric), Err(RunError::AlreadyOnBoard(BramGatewarden)));
}

#[test]
fn programs_respect_unlocks_and_signatures() {
    let mut run = run_with_shop(BramGatewarden, 10);
    run.buy(0).unwrap();
    let loc = find(&run, BramGatewarden);
    let fireball = Program::new([Node::new(Condition::Always, Action::Fireball(Target::Nearest))]);
    assert!(matches!(run.set_program(loc, fireball), Err(RunError::InvalidProgram(_))));
    let level_four = Program::new([Node::new(Condition::EnemyInRange, Action::Guard)]);
    assert!(run.set_program(loc, level_four.clone()).is_err());
    run.level = 4;
    assert_eq!(run.set_program(loc, level_four), Ok(()));
}

#[test]
fn synergy_previews() {
    let mut run = run_with_shop(BramGatewarden, 10);
    run.buy(0).unwrap();
    assert!(run.would_raise_synergy(SirAldric));
    assert!(!run.would_raise_synergy(BramGatewarden), "already on the board");
    assert!(!run.would_raise_synergy(Tobble));
}

fn play_round(run: &mut Run) -> RoundReport {
    let setup = run.start_battle().unwrap();
    let mut battle = setup.build();
    battle.run_to_end();
    run.resolve(&battle)
}

#[test]
fn an_empty_board_loses_takes_damage_and_gains_xp() {
    let mut run = Run::new(11);
    let report = play_round(&mut run);
    assert_eq!(report.outcome, Outcome::Won(Team::Enemy));
    assert_eq!(report.damage, 2 + 3, "three level 1 enemies survive");
    assert_eq!(run.hp(), START_HP - 5);
    assert_eq!(run.round(), 2);
    assert_eq!(run.xp(), XP_PER_ROUND);
    assert_eq!(report.income.total(), BASE_INCOME + interest(START_GOLD));
}

#[test]
fn round_xp_can_level_up() {
    let mut run = Run::new(12);
    run.xp = 5;
    let report = play_round(&mut run);
    assert_eq!(report.level_ups, vec![4]);
    assert_eq!(run.level(), 4);
}

#[test]
fn replaying_a_setup_reproduces_the_battle() {
    let mut run = run_with_shop(Hilda, 10);
    run.buy(0).unwrap();
    play_round(&mut run);
    let (setup, report) = &run.history()[0];
    let mut again = setup.build();
    assert_eq!(again.run_to_end(), report.outcome);
    assert_eq!(again.tick(), report.ticks);
}

#[test]
fn losing_every_round_ends_in_defeat() {
    let mut run = Run::new(13);
    let mut rounds = 0;
    while run.status() == RunStatus::Planning {
        play_round(&mut run);
        rounds += 1;
        assert!(rounds < 50, "run never ended");
    }
    assert_eq!(run.status(), RunStatus::Defeat);
    assert_eq!(run.buy(0), Err(RunError::RunOver));
}

#[test]
fn surviving_the_final_round_is_victory() {
    let mut run = Run::new(14);
    run.round = FINAL_ROUND;
    let report = play_round(&mut run);
    assert_eq!(report.status, RunStatus::Victory);
}

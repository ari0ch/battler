use super::economy::*;
use super::*;
use crate::{Action, Condition, Node, Target};

/// A run with plenty of gold and a shop full of one class, for focused tests.
fn run_with_shop(class: Class, gold: u32) -> Run {
    let mut run = Run::new(1);
    run.gold = gold;
    run.shop = [Some(class); SHOP_SIZE];
    run
}

fn first_bench(run: &Run) -> Location {
    Location::Bench(run.bench.iter().position(Option::is_some).expect("a bench unit"))
}

#[test]
fn new_run_starts_with_full_shop_and_defaults() {
    let run = Run::new(42);
    assert_eq!((run.round(), run.hp(), run.gold()), (1, START_HP, START_GOLD));
    assert!(run.shop().iter().all(Option::is_some));
    let drawn: u32 = Class::ALL.iter().map(|&c| POOL_COPIES - run.pool_remaining(c)).sum();
    assert_eq!(drawn, SHOP_SIZE as u32, "shop offers come out of the pool");
}

#[test]
fn same_seed_same_shop() {
    assert_eq!(Run::new(7).shop(), Run::new(7).shop());
}

#[test]
fn buying_costs_gold_and_fills_the_bench() {
    let mut run = run_with_shop(Class::Knight, 10);
    run.buy(0).unwrap();
    assert_eq!(run.gold(), 10 - UNIT_COST);
    assert_eq!(run.shop()[0], None);
    assert_eq!(run.bench().iter().flatten().count(), 1);
    assert_eq!(run.buy(0), Err(RunError::NoOffer(0)));
}

#[test]
fn buying_needs_enough_gold() {
    let mut run = run_with_shop(Class::Knight, 2);
    assert_eq!(run.buy(0), Err(RunError::NotEnoughGold { need: 3, have: 2 }));
}

#[test]
fn three_copies_merge_into_level_two() {
    let mut run = run_with_shop(Class::Mage, 100);
    for i in 0..3 {
        run.buy(i).unwrap();
    }
    let units: Vec<_> = run.bench().iter().flatten().collect();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].level, Level::Two);
    assert_eq!(run.copies_owned(Class::Mage), 3);
}

#[test]
fn nine_copies_cascade_to_level_three() {
    let mut run = run_with_shop(Class::Rogue, 100);
    for _ in 0..9 {
        let slot = run.shop.iter().position(Option::is_some).unwrap_or_else(|| {
            run.shop = [Some(Class::Rogue); SHOP_SIZE];
            0
        });
        run.buy(slot).unwrap();
    }
    let units: Vec<_> = run.bench().iter().flatten().collect();
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].level, Level::Three);
}

#[test]
fn merge_keeps_the_board_units_program_and_place() {
    let mut run = run_with_shop(Class::Knight, 100);
    run.buy(0).unwrap();
    let spot = Location::Board(Pos::new(2, 2));
    run.move_unit(first_bench(&run), spot).unwrap();
    let program = Program::new([Node::new(Condition::EnemyAdjacent, Action::Taunt)]);
    run.set_program(spot, program.clone()).unwrap();

    run.buy(1).unwrap();
    run.buy(2).unwrap();
    let knight = run.unit_at(spot).expect("merged unit stays on the board");
    assert_eq!(knight.level, Level::Two);
    assert_eq!(knight.program, program);
    assert!(run.bench().iter().all(Option::is_none));
}

#[test]
fn a_full_bench_still_accepts_a_merging_copy() {
    let mut run = Run::new(5);
    run.gold = 100;
    // Fill the bench: two Clerics plus seven other units.
    let fillers = [Class::Knight, Class::Ranger, Class::Mage, Class::Rogue, Class::Berserker];
    let mut classes = vec![Class::Cleric, Class::Cleric];
    classes.extend(fillers.iter().cycle().take(7));
    for class in classes {
        run.shop = [Some(class); SHOP_SIZE];
        run.buy(0).unwrap();
    }
    assert!(run.bench().iter().all(Option::is_some));

    run.shop = [Some(Class::Mage); SHOP_SIZE];
    assert_eq!(run.buy(0), Err(RunError::BenchFull), "a Mage wouldn't merge... yet");
    run.shop = [Some(Class::Cleric); SHOP_SIZE];
    run.buy(0).expect("third Cleric merges, so it fits");
    assert_eq!(run.bench().len(), BENCH_SIZE);
    assert!(
        run.bench().iter().flatten().any(|u| u.class == Class::Cleric && u.level == Level::Two)
    );
}

#[test]
fn selling_refunds_and_returns_copies_to_the_pool() {
    let mut run = run_with_shop(Class::Ranger, 9);
    let before = run.pool_remaining(Class::Ranger);
    run.buy(0).unwrap();
    let refund = run.sell(first_bench(&run)).unwrap();
    assert_eq!(refund, UNIT_COST);
    assert_eq!(run.gold(), 9);
    assert_eq!(run.pool_remaining(Class::Ranger), before + 1);
}

#[test]
fn reroll_costs_gold_and_returns_offers() {
    let mut run = Run::new(3);
    let in_pool = |r: &Run| Class::ALL.iter().map(|&c| r.pool_remaining(c)).sum::<u32>();
    let before = in_pool(&run);
    run.reroll().unwrap();
    assert_eq!(run.gold(), START_GOLD - REROLL_COST);
    assert_eq!(in_pool(&run), before, "old offers go back, new ones come out");
}

#[test]
fn board_rules_are_enforced() {
    let mut run = Run::new(2);
    run.gold = 100;
    for class in [Class::Knight, Class::Knight, Class::Ranger, Class::Mage, Class::Cleric] {
        run.shop = [Some(class); SHOP_SIZE];
        run.buy(0).unwrap();
    }
    let bench_of = |run: &Run, class: Class| {
        Location::Bench(
            run.bench().iter().position(|s| s.as_ref().is_some_and(|u| u.class == class)).unwrap(),
        )
    };
    let b = |x, y| Location::Board(Pos::new(x, y));

    assert_eq!(
        run.move_unit(bench_of(&run, Class::Knight), b(5, 0)),
        Err(RunError::NotPlayerSide(Pos::new(5, 0)))
    );
    run.move_unit(bench_of(&run, Class::Knight), b(0, 0)).unwrap();
    assert_eq!(
        run.move_unit(bench_of(&run, Class::Knight), b(1, 0)),
        Err(RunError::ClassAlreadyOnBoard(Class::Knight))
    );
    run.move_unit(bench_of(&run, Class::Ranger), b(1, 0)).unwrap();
    run.move_unit(bench_of(&run, Class::Mage), b(2, 0)).unwrap();
    assert_eq!(run.board_limit(), 3);
    assert_eq!(
        run.move_unit(bench_of(&run, Class::Cleric), b(3, 0)),
        Err(RunError::BoardFull { max: 3 })
    );
    // Swapping a bench unit with a board unit is fine even when the board is full.
    run.move_unit(bench_of(&run, Class::Cleric), b(2, 0)).unwrap();
    assert_eq!(run.unit_at(b(2, 0)).unwrap().class, Class::Cleric);
    assert!(run.bench().iter().flatten().any(|u| u.class == Class::Mage));
}

#[test]
fn programs_are_validated_for_class_and_level() {
    let mut run = run_with_shop(Class::Knight, 10);
    run.buy(0).unwrap();
    let loc = first_bench(&run);
    let fireball = Program::new([Node::new(Condition::Always, Action::Fireball(Target::Nearest))]);
    assert!(matches!(run.set_program(loc, fireball), Err(RunError::InvalidProgram(_))));
    let three = Program::new(vec![Node::new(Condition::Always, Action::Guard); 3]);
    assert!(run.set_program(loc, three).is_err(), "level 1 holds two cards");
}

fn play_round(run: &mut Run) -> RoundReport {
    let setup = run.start_battle().unwrap();
    let mut battle = setup.build();
    battle.run_to_end();
    run.resolve(&battle)
}

#[test]
fn an_empty_board_loses_and_takes_damage() {
    let mut run = Run::new(11);
    let report = play_round(&mut run);
    assert_eq!(report.outcome, Outcome::Won(Team::Enemy));
    assert_eq!(report.damage, 2 + 3, "three level 1 enemies survive");
    assert_eq!(run.hp(), START_HP - 5);
    assert_eq!(run.round(), 2);
    assert_eq!(report.streak, -1);
    assert_eq!(report.income.total(), BASE_INCOME + interest(START_GOLD));
    assert_eq!(run.gold(), START_GOLD + report.income.total());
}

#[test]
fn replaying_a_setup_reproduces_the_battle() {
    let mut run = Run::new(12);
    run.shop = [Some(Class::Berserker); SHOP_SIZE];
    run.buy(0).unwrap();
    run.move_unit(first_bench(&run), Location::Board(Pos::new(3, 2))).unwrap();
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
    assert_eq!(run.hp(), 0);
    assert_eq!(run.buy(0), Err(RunError::RunOver));
}

#[test]
fn surviving_the_final_round_is_victory() {
    let mut run = Run::new(14);
    run.round = FINAL_ROUND;
    run.hp = START_HP;
    // Even a loss in the final round is a victory if HP remains.
    let report = play_round(&mut run);
    assert_eq!(report.status, RunStatus::Victory);
    assert_eq!(run.round(), FINAL_ROUND);
}

#[test]
fn streaks_build_and_reset() {
    let mut run = Run::new(15);
    play_round(&mut run);
    play_round(&mut run);
    assert_eq!(run.streak(), -2);
    let report = play_round(&mut run);
    assert_eq!(report.income.streak, streak_bonus(3));
}

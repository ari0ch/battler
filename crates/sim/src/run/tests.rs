use super::economy::*;
use super::*;
use crate::{Action, Condition, Node, Outcome, Rarity, Target};
use Character::*;

const ME: PlayerId = PlayerId(0);

/// A lone player with its own pool, for shop and board rules.
fn solo(seed: u64) -> (Player, Pool) {
    let mut pool = Pool::new();
    let player = Player::new(ME, "You", seed, &mut pool);
    (player, pool)
}

/// A lone player with plenty of gold and a shop full of one character.
fn with_shop(character: Character, gold: u32) -> Player {
    let (mut p, _) = solo(1);
    p.set_gold(gold);
    p.set_shop(character);
    p
}

fn find(p: &Player, character: Character) -> Location {
    let id = p.all_units().find(|u| u.character == character).expect("owned").id;
    p.locate(id).unwrap()
}

// ----- Shop, bench and board ------------------------------------------------

#[test]
fn new_player_starts_at_level_three_with_a_shop() {
    let (p, pool) = solo(42);
    assert_eq!((p.hp(), p.gold(), p.level()), (START_HP, START_GOLD, 3));
    assert_eq!(p.board_limit(), 3);
    assert!(p.shop().iter().all(Option::is_some));
    assert_eq!(Pool::new().total() - pool.total(), SHOP_SIZE as u32, "offers leave the pool");
}

#[test]
fn level_three_shops_never_offer_epics() {
    let (mut p, mut pool) = solo(5);
    p.set_gold(1000);
    for _ in 0..100 {
        p.reroll(&mut pool).unwrap();
        assert!(p.shop().iter().flatten().all(|c| c.rarity() != Rarity::Epic));
    }
}

#[test]
fn same_seed_same_shop() {
    assert_eq!(solo(7).0.shop(), solo(7).0.shop());
}

#[test]
fn buying_costs_rarity_price_and_auto_places() {
    let mut p = with_shop(SirAldric, 10);
    p.buy(0).unwrap();
    assert_eq!(p.gold(), 10 - Rarity::Rare.cost());
    assert_eq!(p.shop()[0], None);
    assert_eq!(p.board().len(), 1, "room on the board: placed straight away");
    assert_eq!(p.board()[0].0.x, SIDE_DEPTH - 1, "melee goes to the front");
    assert_eq!(p.buy(0), Err(RunError::NoOffer(0)));

    p.set_shop(PipQuickfeather);
    p.buy(0).unwrap();
    assert!(
        p.board().iter().any(|(pos, u)| u.character == PipQuickfeather && pos.x == 0),
        "ranged at the back"
    );
}

#[test]
fn a_second_copy_waits_on_the_bench() {
    let mut p = with_shop(Nettle, 10);
    p.buy(0).unwrap();
    p.buy(1).unwrap();
    assert_eq!(p.board().len(), 1);
    assert_eq!(p.bench().iter().flatten().count(), 1);
}

#[test]
fn buying_needs_enough_gold() {
    let mut p = with_shop(Ysolde, 2);
    assert_eq!(p.buy(0), Err(RunError::NotEnoughGold { need: 3, have: 2 }));
}

#[test]
fn three_copies_merge_and_keep_the_board_program() {
    let mut p = with_shop(SirAldric, 100);
    p.buy(0).unwrap();
    let spot = find(&p, SirAldric);
    let program = Program::new([Node::new(
        Condition::AllyHpBelow(crate::Threshold::P50),
        Action::ShieldWall,
    )]);
    p.set_program(spot, program.clone()).unwrap();
    p.buy(1).unwrap();
    p.buy(2).unwrap();
    let aldric = p.unit_at(spot).expect("merged unit stays on the board");
    assert_eq!(aldric.level, Level::Two);
    assert_eq!(aldric.program, program);
    assert!(p.bench().iter().all(Option::is_none));
}

#[test]
fn nine_copies_cascade_to_level_three() {
    let mut p = with_shop(Grub, 100);
    for _ in 0..9 {
        if p.shop().iter().all(Option::is_none) {
            p.set_shop(Grub);
        }
        let slot = p.shop().iter().position(Option::is_some).unwrap();
        p.buy(slot).unwrap();
    }
    assert_eq!(p.copies_owned(Grub), 9);
    assert_eq!(p.unit_at(find(&p, Grub)).unwrap().level, Level::Three);
}

#[test]
fn a_full_bench_still_accepts_a_merging_copy() {
    let (mut p, _) = solo(5);
    p.set_gold(100);
    p.set_level(MAX_LEVEL, 0); // board space doesn't matter here
    let mut others: Vec<Character> =
        Character::ALL.iter().copied().filter(|&c| c != SisterAgnes && c != Tobble).collect();
    // Fill the board first so new buys stay on the bench.
    for _ in 0..MAX_LEVEL {
        p.set_shop(others.remove(0));
        p.buy(0).unwrap();
    }
    for c in [SisterAgnes, SisterAgnes].into_iter().chain(others.into_iter().take(7)) {
        p.set_shop(c);
        p.buy(0).unwrap();
    }
    assert!(p.bench().iter().all(Option::is_some));
    p.set_shop(Tobble);
    assert_eq!(p.buy(0), Err(RunError::BenchFull));
    p.set_shop(SisterAgnes);
    p.buy(0).expect("third Agnes merges, so it fits");
    assert_eq!(p.bench().len(), BENCH_SIZE);
}

#[test]
fn selling_refunds_rarity_price_and_returns_copies() {
    let mut p = with_shop(Kestrel, 9);
    let mut pool = Pool::new();
    let before = pool.remaining(Kestrel);
    p.buy(0).unwrap();
    let refund = p.sell(find(&p, Kestrel), &mut pool).unwrap();
    assert_eq!(refund, Rarity::Rare.cost());
    assert_eq!(p.gold(), 9);
    assert_eq!(pool.remaining(Kestrel), before + 1);
}

#[test]
fn reroll_costs_gold_and_returns_offers() {
    let (mut p, mut pool) = solo(3);
    let before = pool.total();
    p.reroll(&mut pool).unwrap();
    assert_eq!(p.gold(), START_GOLD - REROLL_COST);
    assert_eq!(pool.total(), before);
}

#[test]
fn buying_xp_levels_up_and_grows_the_board() {
    let (mut p, _) = solo(4);
    p.set_gold(100);
    assert_eq!(p.buy_xp(), Ok(vec![]));
    assert_eq!(p.xp(), 4);
    assert_eq!(p.buy_xp(), Ok(vec![4]), "6 XP for level 4");
    assert_eq!((p.level(), p.xp()), (4, 2));
    assert_eq!(p.board_limit(), 4);
    p.set_level(MAX_LEVEL, 0);
    assert_eq!(p.buy_xp(), Err(RunError::MaxLevel));
}

#[test]
fn board_rules_are_enforced() {
    let (mut p, _) = solo(2);
    p.set_gold(100);
    let b = |x, y| Location::Board(Pos::new(x, y));
    for c in [BramGatewarden, SirAldric, Tobble] {
        p.set_shop(c);
        p.buy(0).unwrap();
    }
    assert_eq!(p.board().len(), 3, "level 3: three on the board");
    p.set_shop(SisterAgnes);
    p.buy(0).unwrap();
    let agnes = find(&p, SisterAgnes);
    assert!(matches!(agnes, Location::Bench(_)), "board full: stays on the bench");
    assert_eq!(p.move_unit(agnes, b(0, 4)), Err(RunError::BoardFull { max: 3 }));
    assert_eq!(p.move_unit(agnes, b(5, 0)), Err(RunError::NotPlayerSide(Pos::new(5, 0))));

    // Swapping a bench unit with a board unit is fine even when the board is full.
    let tobble = find(&p, Tobble);
    p.move_unit(agnes, tobble).unwrap();
    assert_eq!(p.unit_at(tobble).unwrap().character, SisterAgnes);

    // Two different Knights are fine; the same character twice is not.
    p.set_shop(BramGatewarden);
    p.buy(0).unwrap();
    let second_bram = Location::Bench(
        p.bench()
            .iter()
            .position(|s| s.as_ref().is_some_and(|u| u.character == BramGatewarden))
            .unwrap(),
    );
    let aldric = find(&p, SirAldric);
    assert_eq!(p.move_unit(second_bram, aldric), Err(RunError::AlreadyOnBoard(BramGatewarden)));
}

#[test]
fn programs_respect_unlocks_and_signatures() {
    let mut p = with_shop(BramGatewarden, 10);
    p.buy(0).unwrap();
    let loc = find(&p, BramGatewarden);
    let fireball = Program::new([Node::new(Condition::Always, Action::Fireball(Target::Nearest))]);
    assert!(matches!(p.set_program(loc, fireball), Err(RunError::InvalidProgram(_))));
    let level_four = Program::new([Node::new(Condition::EnemyInRange, Action::Guard)]);
    assert!(p.set_program(loc, level_four.clone()).is_err());
    p.set_level(4, 0);
    assert_eq!(p.set_program(loc, level_four), Ok(()));
}

#[test]
fn synergy_previews() {
    let mut p = with_shop(BramGatewarden, 10);
    p.buy(0).unwrap();
    assert!(p.would_raise_synergy(SirAldric));
    assert!(!p.would_raise_synergy(BramGatewarden), "already on the board");
    assert!(!p.would_raise_synergy(Tobble));
}

// ----- The lobby ------------------------------------------------------------

/// Plays one round, returning the human's report.
fn play_round(lobby: &mut Lobby) -> RoundReport {
    lobby.start_round().unwrap();
    lobby.resolve();
    lobby.player(ME).history().last().unwrap().1.clone()
}

#[test]
fn rounds_follow_the_pve_schedule() {
    use crate::{Monster, Wave};
    assert_eq!(round_kind(1), RoundKind::Pve(Wave::Slimes));
    assert_eq!(round_kind(3), RoundKind::Pve(Wave::Goblins));
    assert_eq!(round_kind(4), RoundKind::Versus);
    let boss = |boss, tier| RoundKind::Pve(Wave::Boss { boss, tier });
    assert_eq!(round_kind(7), boss(Monster::OgreChieftain, 0));
    assert_eq!(round_kind(11), boss(Monster::FrostWyrm, 0));
    assert_eq!(round_kind(15), boss(Monster::Lich, 0));
    assert_eq!(round_kind(19), boss(Monster::OgreChieftain, 1));
    assert_eq!(round_kind(8), RoundKind::Versus);
}

#[test]
fn a_lobby_has_one_human_and_seven_bots() {
    let lobby = Lobby::new(1, 1);
    assert_eq!(lobby.players().len(), LOBBY_SIZE);
    assert!(!lobby.is_bot(ME));
    assert!((1..LOBBY_SIZE).all(|i| lobby.is_bot(PlayerId(i))));
    assert_eq!(lobby.player(ME).name(), "You");
}

#[test]
fn bots_build_boards_through_the_shop() {
    let lobby = Lobby::new(3, 1);
    for p in &lobby.players()[1..] {
        assert!(!p.board().is_empty(), "{} bought nothing", p.name());
        assert!(p.gold() < START_GOLD, "{} spent nothing", p.name());
        assert!(p.board().len() <= p.board_limit());
    }
    // Every copy bots hold came out of the shared pool.
    let held: u32 =
        lobby.players().iter().flat_map(|p| p.all_units()).map(|u| u.level.copies()).sum();
    let offered: u32 =
        lobby.players().iter().map(|p| p.shop().iter().flatten().count() as u32).sum();
    assert_eq!(lobby.pool().total() + held + offered, Pool::new().total());
}

#[test]
fn commands_are_refused_during_a_battle() {
    let mut lobby = Lobby::new(4, 1);
    lobby.start_round().unwrap();
    assert_eq!(lobby.apply(ME, Command::Reroll), Err(RunError::BattleInProgress));
    lobby.resolve();
    assert!(lobby.apply(ME, Command::Reroll).is_ok());
}

#[test]
fn an_empty_board_loses_to_monsters_for_a_flat_cost() {
    let mut lobby = Lobby::new(11, 1);
    let report = play_round(&mut lobby);
    assert_eq!(report.opponent, Opponent::Monsters(crate::Wave::Slimes));
    assert_eq!(report.result, Some(false));
    assert_eq!(report.damage, MONSTER_LOSS_DAMAGE);
    assert_eq!(report.streak, 0, "PvE rounds don't touch streaks");
    assert_eq!(lobby.player(ME).hp(), START_HP - MONSTER_LOSS_DAMAGE);
    assert_eq!(lobby.round(), 2);
    assert_eq!(lobby.player(ME).xp(), XP_PER_ROUND);
    assert_eq!(report.income.total(), BASE_INCOME + interest(START_GOLD));
}

#[test]
fn bots_beat_the_first_monster_rounds() {
    let mut lobby = Lobby::new(21, 0);
    for _ in 0..3 {
        lobby.start_round().unwrap();
        lobby.resolve();
    }
    for p in lobby.players() {
        let wins = p.history().iter().filter(|(_, r)| r.result == Some(true)).count();
        assert_eq!(wins, 3, "{} lost a monster round", p.name());
        assert_eq!(p.history()[0].1.income.win, MONSTER_REWARD);
    }
}

#[test]
fn versus_rounds_pair_everyone_once() {
    let mut lobby = Lobby::new(5, 0);
    for _ in 0..3 {
        lobby.start_round().unwrap();
        lobby.resolve();
    }
    assert_eq!(lobby.round_kind(), RoundKind::Versus);
    lobby.start_round().unwrap();
    for p in lobby.players() {
        let (_, side, opponent) = lobby.battle_for(p.id()).expect("everyone fights");
        let Opponent::Player(other) = opponent else { panic!("versus round") };
        let (_, other_side, back) = lobby.battle_for(other).unwrap();
        assert_eq!(back, Opponent::Player(p.id()), "pairs are symmetric");
        assert_ne!(side, other_side);
    }
}

#[test]
fn an_odd_player_out_fights_a_ghost_for_free() {
    let mut lobby = Lobby::new(6, 1);
    for _ in 0..3 {
        lobby.start_round().unwrap();
        lobby.resolve();
    }
    // Knock out one bot so seven remain.
    lobby.player_mut(PlayerId(7)).set_hp(1);
    while lobby.alive_count() == LOBBY_SIZE {
        lobby.start_round().unwrap();
        lobby.resolve();
    }
    if lobby.round_kind() != RoundKind::Versus {
        return; // landed on a boss round; the pairing test covers versus rounds
    }
    lobby.start_round().unwrap();
    let ghosts: Vec<_> = lobby
        .players()
        .iter()
        .filter(|p| p.is_alive())
        .filter(|p| matches!(lobby.battle_for(p.id()), Some((_, _, Opponent::Ghost(_)))))
        .collect();
    assert_eq!(ghosts.len(), 1);
    let ghost = ghosts[0].id();
    let hp = lobby.player(ghost).hp();
    lobby.resolve();
    assert_eq!(lobby.player(ghost).hp(), hp, "losing to a ghost costs nothing");
}

#[test]
fn replaying_a_setup_reproduces_the_battle() {
    let mut lobby = Lobby::new(8, 1);
    lobby.player_mut(ME).set_shop(Hilda);
    lobby.apply(ME, Command::Buy(0)).unwrap();
    play_round(&mut lobby);
    let (setup, report) = &lobby.player(ME).history()[0];
    let mut again = setup.build();
    assert_eq!(again.run_to_end(), Outcome::Won(Team::Player));
    assert_eq!(report.result, Some(true));
    assert_eq!(again.tick(), report.ticks);
}

#[test]
fn lobbies_are_deterministic() {
    let play = |seed| {
        let mut lobby = Lobby::new(seed, 0);
        for _ in 0..12 {
            if lobby.is_over() {
                break;
            }
            lobby.start_round().unwrap();
            lobby.resolve();
        }
        lobby.players().iter().map(|p| (p.hp(), p.gold(), p.level())).collect::<Vec<_>>()
    };
    assert_eq!(play(99), play(99));
    assert_ne!(play(99), play(100));
}

#[test]
fn an_idle_human_is_knocked_out_with_a_placement() {
    let mut lobby = Lobby::new(13, 1);
    let mut rounds = 0;
    while lobby.player(ME).is_alive() {
        play_round(&mut lobby);
        rounds += 1;
        assert!(rounds < 100, "never knocked out");
    }
    let me = lobby.player(ME);
    assert_eq!(me.status(), RunStatus::Defeat);
    let placement = me.placement().expect("placed");
    assert!((2..=LOBBY_SIZE as u32).contains(&placement));
    assert_eq!(me.history().last().unwrap().1.placement, Some(placement));
    assert!(me.board().is_empty() && me.all_units().next().is_none(), "units return to the pool");
    assert_eq!(lobby.apply(ME, Command::Reroll), Err(RunError::RunOver));
}

#[test]
fn a_bot_lobby_ends_with_one_winner_and_every_placement() {
    let mut lobby = Lobby::new(17, 0);
    while !lobby.is_over() {
        lobby.start_round().unwrap();
        lobby.resolve();
        assert!(lobby.round() < 200, "lobby never ended");
    }
    let mut placements: Vec<u32> =
        lobby.players().iter().map(|p| p.placement().expect("placed")).collect();
    placements.sort();
    assert_eq!(placements, (1..=LOBBY_SIZE as u32).collect::<Vec<_>>());
    let winners: Vec<_> =
        lobby.players().iter().filter(|p| p.status() == RunStatus::Victory).collect();
    assert_eq!(winners.len(), 1);
    assert_eq!(winners[0].placement(), Some(1));
    assert_eq!(lobby.start_round(), Err(RunError::RunOver));
}

// ----- Items -------------------------------------------------------------------

#[test]
fn opening_waves_always_drop_two_commons() {
    let mut lobby = Lobby::new(5, 1);
    let report = play_round(&mut lobby);
    assert_eq!(report.loot.len(), 2);
    assert!(report.loot.iter().all(|i| i.rarity() == Rarity::Common));
    for p in lobby.players() {
        // Bots may have equipped theirs; either way each player got two.
        let equipped: usize = p.all_units().map(|u| u.items.iter().flatten().count()).sum();
        assert_eq!(p.stash().len() + equipped, 2, "{}", p.name());
    }
}

#[test]
fn bosses_drop_by_tier_and_result() {
    use crate::{Monster, Wave};
    let mut rng = crate::Rng::new(1);
    let boss = |boss, tier| Wave::Boss { boss, tier };
    let mut drop = |w, result| {
        super::lobby::loot(w, result, &mut rng).iter().map(|i| i.rarity()).collect::<Vec<_>>()
    };
    let (ogre, wyrm, lich) = (Monster::OgreChieftain, Monster::FrostWyrm, Monster::Lich);
    // The first boss of the run (the Ogre, round 7) drops a rare...
    assert_eq!(drop(boss(ogre, 0), Some(true)), vec![Rarity::Rare]);
    // ...every later one a rare and an epic.
    assert_eq!(drop(boss(wyrm, 0), Some(true)), vec![Rarity::Rare, Rarity::Epic]);
    assert_eq!(drop(boss(lich, 0), Some(true)), vec![Rarity::Rare, Rarity::Epic]);
    assert_eq!(drop(boss(ogre, 1), Some(true)), vec![Rarity::Rare, Rarity::Epic]);
    // Losing (or drawing) gets a consolation common.
    assert_eq!(drop(boss(lich, 2), Some(false)), vec![Rarity::Common]);
    assert_eq!(drop(boss(ogre, 0), None), vec![Rarity::Common]);
}

/// A player owning one Bram and these stashed items.
fn armed(items: &[Item]) -> Player {
    let mut p = with_shop(BramGatewarden, 10);
    p.buy(0).unwrap();
    p.add_to_stash(items);
    p
}

#[test]
fn equip_unequip_and_swap() {
    let mut p = armed(&[Item::IronSword, Item::Aegis, Item::Spyglass]);
    let bram = find(&p, BramGatewarden);
    p.equip(0, bram, None).unwrap();
    p.equip(0, bram, None).unwrap();
    assert_eq!(p.unit_at(bram).unwrap().items, [Some(Item::IronSword), Some(Item::Aegis)]);
    assert_eq!(p.equip(0, bram, None), Err(RunError::ItemSlotsFull));
    // Dropping onto a slot swaps the old item back into the stash.
    p.equip(0, bram, Some(1)).unwrap();
    assert_eq!(p.unit_at(bram).unwrap().items[1], Some(Item::Spyglass));
    assert_eq!(p.stash(), &[Item::Aegis]);
    p.unequip(bram, 0).unwrap();
    assert_eq!(p.stash(), &[Item::Aegis, Item::IronSword]);
    assert_eq!(p.unequip(bram, 0), Err(RunError::EmptyItemSlot));
    assert_eq!(p.equip(9, bram, None), Err(RunError::NoItem(9)));
}

#[test]
fn selling_returns_items_to_the_stash() {
    let mut pool = Pool::new();
    let mut p = armed(&[Item::Whetstone]);
    let bram = find(&p, BramGatewarden);
    p.equip(0, bram, None).unwrap();
    p.sell(bram, &mut pool).unwrap();
    assert_eq!(p.stash(), &[Item::Whetstone]);
}

#[test]
fn merging_keeps_items_and_stashes_the_overflow() {
    let mut p = with_shop(BramGatewarden, 10);
    p.add_to_stash(&[Item::IronSword, Item::Aegis, Item::Spyglass]);
    p.buy(0).unwrap();
    p.buy(1).unwrap();
    // Two copies, one item each, plus one more on the first.
    let units: Vec<Location> = p
        .all_units()
        .map(|u| u.id)
        .collect::<Vec<_>>()
        .iter()
        .map(|&id| p.locate(id).unwrap())
        .collect();
    p.equip(0, units[0], None).unwrap();
    p.equip(0, units[0], None).unwrap();
    p.equip(0, units[1], None).unwrap();
    p.buy(2).unwrap();
    let merged: Vec<&OwnedUnit> = p.all_units().collect();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].level, crate::Level::Two);
    assert_eq!(merged[0].items.iter().flatten().count(), 2);
    assert_eq!(p.stash().len(), 1, "the third item didn't fit");
}

#[test]
fn equip_goes_through_the_lobby() {
    let mut lobby = Lobby::new(5, 1);
    play_round(&mut lobby);
    let me = lobby.player(ME);
    let Some(unit) = me.all_units().next().map(|u| u.id) else { return };
    let loc = me.locate(unit).unwrap();
    let before = me.stash().len();
    lobby.apply(ME, Command::Equip { item: 0, to: loc, slot: None }).unwrap();
    assert_eq!(lobby.player(ME).stash().len(), before - 1);
    lobby.apply(ME, Command::Unequip { from: loc, slot: 0 }).unwrap();
    assert_eq!(lobby.player(ME).stash().len(), before);
}

// ----- Seats, steps and replays (multiplayer) ---------------------------------

/// Plays a lobby with two humans for a few rounds through [`Step`]s, the way
/// the server does, and returns the log.
fn played_log(seed: u64, seats: &[Seat], rounds: u32) -> (Lobby, Vec<Step>) {
    let mut lobby = Lobby::with_seats(seed, seats);
    let mut log = Vec::new();
    let mut push = |lobby: &mut Lobby, step: Step| {
        if lobby.step(&step).is_ok() {
            log.push(step);
        }
    };
    for _ in 0..rounds {
        if lobby.is_over() {
            break;
        }
        for p in 0..seats.len() {
            let player = PlayerId(p);
            for command in [Command::Buy(0), Command::Buy(1), Command::Reroll, Command::Buy(2)] {
                push(&mut lobby, Step::Command { player, command });
            }
        }
        push(&mut lobby, Step::StartRound);
        push(&mut lobby, Step::Resolve);
    }
    (lobby, log)
}

#[test]
fn seats_name_the_players_and_bots_fill_the_rest() {
    let lobby = Lobby::with_seats(1, &[Seat::human("Ari"), Seat::human("Bea")]);
    assert_eq!(lobby.player(PlayerId(0)).name(), "Ari");
    assert_eq!(lobby.player(PlayerId(1)).name(), "Bea");
    assert!(!lobby.is_bot(PlayerId(0)) && !lobby.is_bot(PlayerId(1)));
    assert!((2..LOBBY_SIZE).all(|i| lobby.is_bot(PlayerId(i))));
    assert_eq!(lobby.player(PlayerId(2)).name(), "Mossa");
}

#[test]
fn replaying_the_log_rebuilds_the_same_lobby() {
    let seats = [Seat::human("Ari"), Seat::human("Bea")];
    let (played, log) = played_log(42, &seats, 6);
    assert!(log.len() > 12, "the humans did things");
    let mut copy = Lobby::with_seats(42, &seats);
    copy.replay(&log);
    assert_eq!(copy.state_hash(), played.state_hash());
    assert_eq!(copy.round(), played.round());
    for i in 0..LOBBY_SIZE {
        let (a, b) = (copy.player(PlayerId(i)), played.player(PlayerId(i)));
        assert_eq!((a.hp(), a.gold(), a.board().len()), (b.hp(), b.gold(), b.board().len()));
    }
}

#[test]
fn the_state_hash_notices_any_difference() {
    let seats = [Seat::human("Ari")];
    let a = Lobby::with_seats(7, &seats);
    let mut b = Lobby::with_seats(7, &seats);
    assert_eq!(a.state_hash(), b.state_hash());
    b.apply(PlayerId(0), Command::Reroll).unwrap();
    assert_ne!(a.state_hash(), b.state_hash());
    assert_ne!(a.state_hash(), Lobby::with_seats(8, &seats).state_hash(), "seed matters");
}

#[test]
fn a_bot_takes_over_a_seat_and_plans_from_the_next_round() {
    let seats = [Seat::human("Ari"), Seat::human("Bea")];
    let mut lobby = Lobby::with_seats(5, &seats);
    let bea = PlayerId(1);
    let bea_units =
        |l: &Lobby| l.player(bea).board().len() + l.player(bea).bench().iter().flatten().count();
    assert_eq!(bea_units(&lobby), 0);
    lobby.step(&Step::SetBot { player: bea, bot: true }).unwrap();
    assert!(lobby.is_bot(bea));
    assert!(bea_units(&lobby) > 0, "the bot plans straight away");
    lobby.step(&Step::StartRound).unwrap();
    lobby.step(&Step::Resolve).unwrap();
    lobby.step(&Step::SetBot { player: bea, bot: false }).unwrap();
    assert!(!lobby.is_bot(bea));
}

#[test]
fn resolving_without_a_round_is_refused() {
    let mut lobby = Lobby::with_seats(3, &[Seat::human("Ari")]);
    assert_eq!(lobby.step(&Step::Resolve), Err(RunError::NoBattle));
    lobby.step(&Step::StartRound).unwrap();
    let ticks = lobby.longest_battle_ticks().expect("round locked in");
    assert!(ticks > 0);
    assert_eq!(lobby.step(&Step::Resolve), Ok(None));
}

// ----- Shop lock ---------------------------------------------------------------

#[test]
fn a_locked_shop_keeps_its_offers_into_the_next_round() {
    let mut lobby = Lobby::with_seats(21, &[Seat::human("Ari")]);
    lobby.apply(ME, Command::Buy(0)).unwrap();
    lobby.apply(ME, Command::LockShop(true)).unwrap();
    let before = lobby.player(ME).shop().to_vec();
    assert!(before[0].is_none(), "the bought slot is empty");
    lobby.start_round().unwrap();
    lobby.resolve();
    assert_eq!(lobby.player(ME).shop(), &before[..], "same offers, same empty slot");
    assert!(lobby.player(ME).shop_locked(), "the lock stays on");
}

#[test]
fn an_unlocked_shop_refreshes_and_rerolling_still_works_while_locked() {
    let mut lobby = Lobby::with_seats(22, &[Seat::human("Ari")]);
    lobby.apply(ME, Command::Buy(0)).unwrap();
    lobby.start_round().unwrap();
    lobby.resolve();
    assert!(lobby.player(ME).shop().iter().all(Option::is_some), "a fresh shop");
    // Locked, a reroll still replaces the offers, and the lock stays.
    lobby.apply(ME, Command::LockShop(true)).unwrap();
    let before = lobby.player(ME).shop().to_vec();
    lobby.apply(ME, Command::Reroll).unwrap();
    assert!(lobby.player(ME).shop_locked());
    assert_eq!(lobby.player(ME).shop().len(), before.len());
    lobby.apply(ME, Command::LockShop(false)).unwrap();
    assert!(!lobby.player(ME).shop_locked());
}

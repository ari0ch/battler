//! Behaviour tests for the interpreter and abilities.
//!
//! Most tests call `set_crit_chance(0)` so damage numbers are exact.

use crate::*;
use Action::*;
use Condition::*;

fn card(condition: Condition, action: Action) -> Node {
    Node::new(condition, action)
}

/// A battle with crits off.
fn battle(width: i32, height: i32) -> Battle {
    let mut b = Battle::new(width, height, 100, 0);
    b.set_crit_chance(0);
    b
}

fn add(b: &mut Battle, team: Team, x: i32, y: i32, class: Class, nodes: &[Node]) -> UnitId {
    let spec = UnitSpec::new(class, Level::Three, Program::new(nodes.to_vec()));
    b.spawn(team, Pos::new(x, y), spec).expect("valid test unit")
}

/// A unit that never does anything.
fn dummy(b: &mut Battle, team: Team, x: i32, y: i32, class: Class) -> UnitId {
    add(b, team, x, y, class, &[card(Always, Hold)])
}

fn fired(events: &[Event], unit: UnitId) -> Option<CardRef> {
    events.iter().find_map(|e| match e {
        Event::CardFired { unit: u, card } if *u == unit => Some(*card),
        _ => None,
    })
}

fn damage_to(events: &[Event], target: UnitId) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Attacked { target: t, damage, .. } if *t == target => Some(*damage),
            _ => None,
        })
        .collect()
}

fn attack_of(class: Class) -> u32 {
    Level::Three.scale(class.base_stats()).attack
}

// ----- Reproducibility ---------------------------------------------------

fn melee_brawl(seed: u64) -> Battle {
    let mut b = Battle::new(8, 4, 300, seed);
    let p = Team::Player;
    let e = Team::Enemy;
    add(&mut b, p, 1, 1, Class::Knight, &[card(EnemyAdjacent, Taunt), card(Always, ShieldWall)]);
    add(&mut b, p, 0, 0, Class::Ranger, &[card(EnemyInRange, AimedShot(Target::LowestHp))]);
    add(
        &mut b,
        p,
        0,
        2,
        Class::Cleric,
        &[card(AllyHpBelow(Threshold::P75), Heal(Target::LowestHp))],
    );
    add(&mut b, e, 6, 1, Class::Berserker, &[card(EnemyAdjacent, Frenzy), card(Always, Cleave)]);
    add(
        &mut b,
        e,
        7,
        0,
        Class::Mage,
        &[card(EnemyAdjacent, Blink), card(Always, Fireball(Target::Nearest))],
    );
    add(
        &mut b,
        e,
        7,
        3,
        Class::Rogue,
        &[card(Always, Shadowstep(Target::Ranged)), card(Always, Backstab(Target::Nearest))],
    );
    b
}

#[test]
fn same_seed_replays_identically() {
    let (mut a, mut b) = (melee_brawl(1234), melee_brawl(1234));
    assert_eq!(a.run_to_end(), b.run_to_end());
    assert_eq!(a.log(), b.log());
    assert_ne!(a.outcome(), Outcome::Ongoing);
}

#[test]
fn different_seeds_can_play_out_differently() {
    let reference = {
        let mut b = melee_brawl(0);
        b.run_to_end();
        b.log().to_vec()
    };
    let differs = (1..=20).any(|seed| {
        let mut b = melee_brawl(seed);
        b.run_to_end();
        b.log() != reference.as_slice()
    });
    assert!(differs, "crits should make at least one seed diverge");
}

#[test]
fn crits_deal_extra_damage() {
    let mut b = battle(2, 1);
    let target = dummy(&mut b, Team::Player, 0, 0, Class::Knight);
    add(&mut b, Team::Enemy, 1, 0, Class::Berserker, &[card(Always, Attack(Target::Nearest))]);
    b.set_crit_chance(100);
    let events = b.step();
    let base = attack_of(Class::Berserker);
    assert_eq!(damage_to(&events, target), vec![base * CRIT_DAMAGE_PERCENT / 100]);
    assert!(events.iter().any(|e| matches!(e, Event::Attacked { crit: true, .. })));
}

// ----- Programs and validation ---------------------------------------------

#[test]
fn node_count_is_capped_by_level() {
    let program = Program::new(vec![card(Always, Hold); 3]);
    assert_eq!(
        program.validate(Class::Knight, Level::One),
        Err(ProgramError::TooManyNodes { max: 2, got: 3 })
    );
    assert_eq!(program.validate(Class::Knight, Level::Two), Ok(()));
}

#[test]
fn classes_can_only_use_their_own_signatures() {
    let fireball = Program::new([card(Always, Fireball(Target::Nearest))]);
    assert!(matches!(
        fireball.validate(Class::Knight, Level::One),
        Err(ProgramError::AbilityNotAvailable { ability: Ability::Fireball, .. })
    ));
    assert_eq!(fireball.validate(Class::Mage, Level::One), Ok(()));

    let ready = Program::new([card(AbilityReady(Ability::Heal), Hold)]);
    assert!(ready.validate(Class::Knight, Level::One).is_err());
}

#[test]
fn only_rangers_focus_the_weakest_enemy() {
    let focus = Program::new([card(Always, Attack(Target::LowestHp))]);
    assert!(matches!(
        focus.validate(Class::Rogue, Level::One),
        Err(ProgramError::TargetNotAvailable { target: Target::LowestHp, .. })
    ));
    assert_eq!(focus.validate(Class::Ranger, Level::One), Ok(()));
    // Picking the most hurt *ally* is fine for anyone who can heal.
    let heal = Program::new([card(Always, Heal(Target::LowestHp))]);
    assert_eq!(heal.validate(Class::Cleric, Level::One), Ok(()));
}

#[test]
fn spawn_rejects_invalid_programs_and_positions() {
    let mut b = battle(3, 1);
    let bad = UnitSpec::new(Class::Knight, Level::One, Program::new([card(Always, Blink)]));
    assert!(matches!(
        b.spawn(Team::Player, Pos::new(0, 0), bad),
        Err(SpawnError::InvalidProgram(_))
    ));
    dummy(&mut b, Team::Player, 0, 0, Class::Knight);
    let ok = UnitSpec::new(Class::Knight, Level::One, Program::default());
    assert_eq!(
        b.spawn(Team::Enemy, Pos::new(0, 0), ok.clone()),
        Err(SpawnError::Occupied(Pos::new(0, 0)))
    );
    assert_eq!(
        b.spawn(Team::Enemy, Pos::new(5, 0), ok),
        Err(SpawnError::OutOfBounds(Pos::new(5, 0)))
    );
}

#[test]
fn levels_scale_stats() {
    let knight = Class::Knight.base_stats();
    assert_eq!(Level::Two.scale(knight).max_hp, knight.max_hp * 180 / 100);
    assert_eq!(Level::Three.scale(knight).range, knight.range);
}

// ----- Interpreter ---------------------------------------------------------

#[test]
fn falls_through_when_an_action_cannot_run() {
    let mut b = battle(4, 1);
    let knight = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Knight,
        &[
            card(Always, Attack(Target::Nearest)), // nothing in range
            card(Always, Guard),
        ],
    );
    dummy(&mut b, Team::Enemy, 3, 0, Class::Knight);
    let events = b.step();
    assert_eq!(fired(&events, knight), Some(CardRef::Node(1)));
}

#[test]
fn false_conditions_are_skipped() {
    let mut b = battle(4, 1);
    let knight = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Knight,
        &[card(MyHpBelow(Threshold::P50), Guard), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 3, 0, Class::Knight);
    assert_eq!(fired(&b.step(), knight), Some(CardRef::Node(1)));
}

#[test]
fn locked_default_rule_advances_then_attacks() {
    let mut b = battle(3, 1);
    let knight = add(&mut b, Team::Player, 0, 0, Class::Knight, &[]);
    let target = dummy(&mut b, Team::Enemy, 2, 0, Class::Knight);

    let first = b.step();
    assert_eq!(fired(&first, knight), Some(CardRef::Default));
    assert!(first.contains(&Event::Moved {
        unit: knight,
        from: Pos::new(0, 0),
        to: Pos::new(1, 0)
    }));

    let second = b.step();
    assert_eq!(fired(&second, knight), Some(CardRef::Default));
    assert_eq!(damage_to(&second, target), vec![attack_of(Class::Knight)]);
}

#[test]
fn card_fired_comes_before_its_effects() {
    let mut b = battle(2, 1);
    let ranger =
        add(&mut b, Team::Player, 0, 0, Class::Ranger, &[card(Always, Attack(Target::Nearest))]);
    dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    let events = b.step();
    assert_eq!(events[0], Event::CardFired { unit: ranger, card: CardRef::Node(0) });
    assert!(matches!(events[1], Event::Attacked { .. }));
}

#[test]
fn abilities_go_on_cooldown() {
    let mut b = battle(4, 1);
    let ranger = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Ranger,
        &[card(Always, AimedShot(Target::Nearest)), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 3, 0, Class::Knight);
    let cooldown = Ability::AimedShot.cooldown();
    let mut cards = Vec::new();
    for _ in 0..=cooldown {
        cards.push(fired(&b.step(), ranger).unwrap());
    }
    // Fires on tick 1, falls through while cooling down, fires again once ready.
    assert_eq!(cards[0], CardRef::Node(0));
    assert!(cards[1..cooldown as usize].iter().all(|&c| c == CardRef::Node(1)));
    assert_eq!(cards[cooldown as usize], CardRef::Node(0));
}

#[test]
fn ability_ready_condition_tracks_cooldowns() {
    let mut b = battle(3, 1);
    let knight = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Knight,
        &[card(AbilityReady(Ability::ShieldWall), ShieldWall), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 2, 0, Class::Knight);
    assert_eq!(fired(&b.step(), knight), Some(CardRef::Node(0)));
    assert_eq!(fired(&b.step(), knight), Some(CardRef::Node(1)));
}

#[test]
fn signals_arrive_on_the_next_tick() {
    let mut b = battle(6, 1);
    add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Knight,
        &[card(EveryNTicks(2), SendSignal(Signal::Red)), card(Always, Hold)],
    );
    let listener = add(
        &mut b,
        Team::Player,
        1,
        0,
        Class::Knight,
        &[card(SignalReceived(Signal::Red), Guard), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 5, 0, Class::Knight);

    let cards: Vec<_> = (0..4).map(|_| fired(&b.step(), listener).unwrap()).collect();
    // Sent on tick 2 (after the listener can't see it yet), seen on tick 3 only.
    assert_eq!(cards, vec![CardRef::Node(1), CardRef::Node(1), CardRef::Node(0), CardRef::Node(1)]);
}

#[test]
fn outnumbered_counts_units_within_two_cells() {
    let mut b = battle(4, 4);
    let knight = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Knight,
        &[card(Outnumbered, Guard), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    dummy(&mut b, Team::Enemy, 0, 1, Class::Knight);
    assert_eq!(fired(&b.step(), knight), Some(CardRef::Node(0)));
}

// ----- Targeting -------------------------------------------------------------

#[test]
fn rangers_focus_the_lowest_hp_enemy() {
    let mut b = battle(4, 3);
    add(&mut b, Team::Player, 0, 1, Class::Ranger, &[card(Always, Attack(Target::LowestHp))]);
    dummy(&mut b, Team::Enemy, 1, 1, Class::Knight); // nearer, but tougher
    let weak = dummy(&mut b, Team::Enemy, 3, 1, Class::Mage);
    let events = b.step();
    assert_eq!(damage_to(&events, weak), vec![attack_of(Class::Ranger)]);
}

#[test]
fn ranged_target_picks_backliners() {
    let mut b = battle(5, 1);
    add(&mut b, Team::Player, 0, 0, Class::Ranger, &[card(Always, Attack(Target::Ranged))]);
    dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    let cleric = dummy(&mut b, Team::Enemy, 3, 0, Class::Cleric);
    assert_eq!(damage_to(&b.step(), cleric).len(), 1);
}

#[test]
fn taunt_overrides_target_choice() {
    let mut b = battle(4, 2);
    let knight = add(&mut b, Team::Player, 0, 0, Class::Knight, &[card(Always, Taunt)]);
    let cleric = dummy(&mut b, Team::Player, 0, 1, Class::Cleric); // lower HP than the knight
    add(&mut b, Team::Enemy, 2, 0, Class::Ranger, &[card(Always, Attack(Target::LowestHp))]);
    let events = b.step();
    assert_eq!(damage_to(&events, knight), vec![attack_of(Class::Ranger)]);
    assert!(damage_to(&events, cleric).is_empty());
}

// ----- Defences --------------------------------------------------------------

#[test]
fn guarding_halves_damage_rounding_up() {
    let mut b = battle(2, 1);
    let knight = add(&mut b, Team::Player, 0, 0, Class::Knight, &[card(Always, Guard)]);
    add(&mut b, Team::Enemy, 1, 0, Class::Berserker, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), knight), vec![attack_of(Class::Berserker).div_ceil(2)]);
}

#[test]
fn shield_wall_protects_adjacent_allies() {
    let mut b = battle(3, 2);
    add(&mut b, Team::Player, 0, 0, Class::Knight, &[card(Always, ShieldWall)]);
    let cleric = dummy(&mut b, Team::Player, 0, 1, Class::Cleric);
    add(&mut b, Team::Enemy, 1, 1, Class::Rogue, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), cleric), vec![attack_of(Class::Rogue).div_ceil(2)]);
}

#[test]
fn sanctuary_blocks_all_damage() {
    let mut b = battle(2, 1);
    let cleric =
        add(&mut b, Team::Player, 0, 0, Class::Cleric, &[card(Always, Sanctuary(Target::Nearest))]);
    add(&mut b, Team::Enemy, 1, 0, Class::Rogue, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), cleric), vec![0]);
}

// ----- Signature abilities -----------------------------------------------------

#[test]
fn heal_skips_full_health_and_heals_the_most_hurt() {
    let mut b = battle(4, 1);
    let cleric = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Cleric,
        &[card(Always, Heal(Target::LowestHp)), card(Always, Hold)],
    );
    let knight = dummy(&mut b, Team::Player, 1, 0, Class::Knight);
    dummy(&mut b, Team::Enemy, 3, 0, Class::Knight);

    assert_eq!(fired(&b.step(), cleric), Some(CardRef::Node(1)), "nobody is hurt yet");

    let max = b.units[1].stats.max_hp;
    b.units[1].hp = max / 2;
    let events = b.step();
    let amount = attack_of(Class::Cleric) * 4;
    assert!(events.contains(&Event::Healed { healer: cleric, target: knight, amount }));
}

#[test]
fn kite_attacks_then_steps_back() {
    let mut b = battle(5, 1);
    let ranger =
        add(&mut b, Team::Player, 1, 0, Class::Ranger, &[card(Always, Kite(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 2, 0, Class::Knight);
    let events = b.step();
    assert_eq!(damage_to(&events, target).len(), 1);
    assert!(events.contains(&Event::Moved {
        unit: ranger,
        from: Pos::new(1, 0),
        to: Pos::new(0, 0)
    }));
}

#[test]
fn fireball_splashes_neighbours_of_the_target() {
    let mut b = battle(5, 3);
    add(&mut b, Team::Player, 0, 1, Class::Mage, &[card(Always, Fireball(Target::Nearest))]);
    let main = dummy(&mut b, Team::Enemy, 2, 1, Class::Knight);
    let next_to = dummy(&mut b, Team::Enemy, 3, 1, Class::Knight);
    let far = dummy(&mut b, Team::Enemy, 4, 2, Class::Knight);
    let events = b.step();
    let attack = attack_of(Class::Mage);
    assert_eq!(damage_to(&events, main), vec![attack]);
    assert_eq!(damage_to(&events, next_to), vec![attack / 2]);
    assert!(damage_to(&events, far).is_empty());
}

#[test]
fn blink_escapes_to_the_safest_cell() {
    let mut b = battle(7, 1);
    let mage = add(&mut b, Team::Player, 1, 0, Class::Mage, &[card(Always, Blink)]);
    dummy(&mut b, Team::Enemy, 0, 0, Class::Knight);
    let events = b.step();
    assert!(events.contains(&Event::Teleported {
        unit: mage,
        from: Pos::new(1, 0),
        to: Pos::new(4, 0)
    }));
}

#[test]
fn backstab_needs_the_target_engaged_by_an_ally() {
    let mut b = battle(3, 1);
    let rogue = add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Rogue,
        &[card(Always, Backstab(Target::Nearest)), card(Always, Hold)],
    );
    dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    assert_eq!(fired(&b.step(), rogue), Some(CardRef::Node(1)), "target isn't engaged");

    let mut b = battle(3, 1);
    add(&mut b, Team::Player, 0, 0, Class::Rogue, &[card(Always, Backstab(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    dummy(&mut b, Team::Player, 2, 0, Class::Knight);
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Class::Rogue) * 3]);
}

#[test]
fn shadowstep_lands_next_to_the_target() {
    let mut b = battle(5, 3);
    let rogue =
        add(&mut b, Team::Player, 0, 0, Class::Rogue, &[card(Always, Shadowstep(Target::Nearest))]);
    dummy(&mut b, Team::Enemy, 2, 1, Class::Knight);
    b.step();
    let pos = b.unit(rogue).unwrap().pos;
    assert!(pos.is_adjacent(Pos::new(2, 1)));
    assert_eq!(pos.distance(Pos::new(0, 0)), 4, "should land on the far side");
}

#[test]
fn frenzy_doubles_basic_attacks() {
    let mut b = battle(2, 1);
    add(
        &mut b,
        Team::Player,
        0,
        0,
        Class::Berserker,
        &[card(Always, Frenzy), card(Always, Attack(Target::Nearest))],
    );
    let target = dummy(&mut b, Team::Enemy, 1, 0, Class::Knight);
    b.step(); // frenzy
    assert_eq!(damage_to(&b.step(), target).len(), 2);
}

#[test]
fn cleave_hits_every_adjacent_enemy() {
    let mut b = battle(3, 3);
    add(&mut b, Team::Player, 1, 1, Class::Berserker, &[card(Always, Cleave)]);
    let foes = [
        dummy(&mut b, Team::Enemy, 0, 1, Class::Knight),
        dummy(&mut b, Team::Enemy, 2, 1, Class::Knight),
        dummy(&mut b, Team::Enemy, 1, 0, Class::Knight),
    ];
    let events = b.step();
    for foe in foes {
        assert_eq!(damage_to(&events, foe), vec![attack_of(Class::Berserker)]);
    }
}

#[test]
fn stalemate_ends_in_a_draw_at_the_tick_limit() {
    let mut b = Battle::new(4, 1, 5, 0);
    dummy(&mut b, Team::Player, 0, 0, Class::Knight);
    dummy(&mut b, Team::Enemy, 3, 0, Class::Knight);
    assert_eq!(b.run_to_end(), Outcome::Draw);
    assert_eq!(b.tick(), 5);
}

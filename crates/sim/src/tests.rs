//! Behaviour tests for the interpreter, abilities, passives and synergies.
//!
//! Most tests call `set_crit_chance(0)` so damage numbers are exact, and use
//! inert target dummies that never act.

use crate::*;
use Action::*;
use Character::*;
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

fn add(b: &mut Battle, team: Team, x: i32, y: i32, who: Character, nodes: &[Node]) -> UnitId {
    let spec = UnitSpec::new(who, Level::Three, Program::new(nodes.to_vec()));
    b.spawn(team, Pos::new(x, y), spec).expect("valid test unit")
}

/// A unit that never takes a turn.
fn dummy(b: &mut Battle, team: Team, x: i32, y: i32, who: Character) -> UnitId {
    let id = add(b, team, x, y, who, &[]);
    b.units.last_mut().unwrap().inert = true;
    id
}

fn unit_mut(b: &mut Battle, id: UnitId) -> &mut Unit {
    b.units.iter_mut().find(|u| u.id == id).unwrap()
}

fn fired(events: &[Event], unit: UnitId) -> Vec<CardRef> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::CardFired { unit: u, card } if *u == unit => Some(*card),
            _ => None,
        })
        .collect()
}

fn first_fired(events: &[Event], unit: UnitId) -> Option<CardRef> {
    fired(events, unit).first().copied()
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

fn healed(events: &[Event], target: UnitId) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Healed { target: t, amount, .. } if *t == target => Some(*amount),
            _ => None,
        })
        .sum()
}

fn attack_of(who: Character) -> u32 {
    Level::Three.scale(who.base_stats()).attack
}

fn max_hp_of(who: Character) -> u32 {
    Level::Three.scale(who.base_stats()).max_hp
}

// ----- Reproducibility ---------------------------------------------------

fn brawl(seed: u64) -> Battle {
    let mut b = Battle::new(8, 4, 300, seed);
    let (p, e) = (Team::Player, Team::Enemy);
    add(&mut b, p, 1, 1, SirAldric, &[card(AllyHpBelow(Threshold::P50), ShieldWall)]);
    add(&mut b, p, 0, 0, Kestrel, &[card(EnemyAdjacent, Kite(Target::Nearest))]);
    add(&mut b, p, 0, 2, Seraphine, &[card(AllyHpBelow(Threshold::P75), MassHeal)]);
    add(&mut b, e, 6, 1, Korgath, &[card(EnemyAdjacent, Rampage)]);
    add(
        &mut b,
        e,
        7,
        0,
        Vorn,
        &[card(EnemyAdjacent, Blink), card(Always, Fireball(Target::Nearest))],
    );
    add(
        &mut b,
        e,
        7,
        3,
        GreyWidow,
        &[card(Always, Assassinate), card(Always, Shadowstep(Target::Weakest))],
    );
    b
}

#[test]
fn same_seed_replays_identically() {
    let (mut a, mut b) = (brawl(1234), brawl(1234));
    assert_eq!(a.run_to_end(), b.run_to_end());
    assert_eq!(a.log(), b.log());
    assert_ne!(a.outcome(), Outcome::Ongoing);
}

#[test]
fn different_seeds_can_play_out_differently() {
    let reference = {
        let mut b = brawl(0);
        b.run_to_end();
        b.log().to_vec()
    };
    let differs = (1..=20).any(|seed| {
        let mut b = brawl(seed);
        b.run_to_end();
        b.log() != reference.as_slice()
    });
    assert!(differs, "crits should make at least one seed diverge");
}

#[test]
fn crits_deal_extra_damage() {
    let mut b = battle(2, 1);
    let target = dummy(&mut b, Team::Player, 0, 0, BramGatewarden);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    b.set_crit_chance(100);
    let events = b.step();
    assert_eq!(damage_to(&events, target), vec![attack_of(Grub) * CRIT_DAMAGE_PERCENT / 100]);
}

// ----- Characters and programs -------------------------------------------

#[test]
fn rarity_scales_stats_and_eagle_eye_adds_range() {
    assert_eq!(SirAldric.base_stats().max_hp, Class::Knight.base_stats().max_hp * 110 / 100);
    assert_eq!(Ysolde.base_stats().attack, Class::Knight.base_stats().attack * 120 / 100);
    assert_eq!(Elowen.base_stats().range, 4);
    assert_eq!(PipQuickfeather.base_stats().range, 3);
}

#[test]
fn roster_has_three_rarities_per_class() {
    for class in Class::ALL {
        let mut rarities: Vec<Rarity> =
            Character::ALL.iter().filter(|c| c.class() == class).map(|c| c.rarity()).collect();
        rarities.sort();
        assert_eq!(rarities, Rarity::ALL.to_vec(), "{class:?}");
    }
    for c in Character::ALL {
        let expected = if c.rarity() == Rarity::Epic { 2 } else { 1 };
        assert_eq!(c.abilities().len(), expected, "{c:?}");
        assert_eq!(c.passive().is_some(), c.rarity() != Rarity::Common, "{c:?}");
    }
}

#[test]
fn characters_can_only_use_their_own_signatures() {
    let fireball = Program::new([card(Always, Fireball(Target::Nearest))]);
    assert!(matches!(
        fireball.validate_basic(BramGatewarden),
        Err(ProgramError::NotThisCharacter { ability: Ability::Fireball, .. })
    ));
    assert_eq!(fireball.validate_basic(Vorn), Ok(()));
}

fn unlocks(level: u32, board: &[Character]) -> Unlocks {
    Unlocks { player_level: level, synergies: Synergies::from_characters(board.iter().copied()) }
}

#[test]
fn blocks_unlock_with_player_level() {
    let p = Program::new([card(EnemyInRange, Attack(Target::Strongest))]);
    assert!(matches!(
        p.validate(PipQuickfeather, Level::One, &unlocks(3, &[])),
        Err(ProgramError::Locked { requirement: Requirement::PlayerLevel(4), .. })
    ));
    assert_eq!(p.validate(PipQuickfeather, Level::One, &unlocks(4, &[])), Ok(()));
}

#[test]
fn synergy_blocks_need_the_synergy_and_sometimes_the_class() {
    let surrounded = Program::new([card(Surrounded, Cleave)]);
    assert!(surrounded.validate(Grub, Level::One, &unlocks(3, &[Grub])).is_err());
    assert_eq!(surrounded.validate(Grub, Level::One, &unlocks(3, &[Grub, Hilda])), Ok(()));
    let class_only = Program::new([card(Surrounded, Guard)]);
    assert!(class_only.validate(Nettle, Level::One, &unlocks(3, &[Grub, Hilda])).is_err());

    let marked = Program::new([card(Always, Attack(Target::Marked))]);
    assert_eq!(marked.validate(Nettle, Level::One, &unlocks(3, &[Kestrel, Elowen])), Ok(()));
}

#[test]
fn card_slots_follow_level_and_knight_synergy() {
    let three = Program::new(vec![card(Always, Guard); 3]);
    assert!(matches!(
        three.validate(BramGatewarden, Level::One, &unlocks(3, &[])),
        Err(ProgramError::TooManyCards { max: 2, got: 3 })
    ));
    let five = Program::new(vec![card(Always, Guard); 5]);
    let knights = [BramGatewarden, SirAldric, Ysolde];
    assert_eq!(five.validate(Ysolde, Level::Three, &unlocks(3, &knights)), Ok(()));
    assert!(five.validate(Ysolde, Level::Three, &unlocks(3, &knights[..2])).is_err());
}

#[test]
fn spawn_rejects_bad_signatures_and_positions() {
    let mut b = battle(3, 1);
    let bad = UnitSpec::new(BramGatewarden, Level::One, Program::new([card(Always, Blink)]));
    assert!(matches!(
        b.spawn(Team::Player, Pos::new(0, 0), bad),
        Err(SpawnError::InvalidProgram(_))
    ));
    dummy(&mut b, Team::Player, 0, 0, BramGatewarden);
    let ok = UnitSpec::new(BramGatewarden, Level::One, Program::default());
    assert_eq!(
        b.spawn(Team::Enemy, Pos::new(0, 0), ok.clone()),
        Err(SpawnError::Occupied(Pos::new(0, 0)))
    );
    assert_eq!(
        b.spawn(Team::Enemy, Pos::new(5, 0), ok),
        Err(SpawnError::OutOfBounds(Pos::new(5, 0)))
    );
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
        BramGatewarden,
        &[
            card(Always, Attack(Target::Nearest)), // nothing in range
            card(Always, Guard),
        ],
    );
    dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), knight), Some(CardRef::Node(1)));
}

#[test]
fn false_conditions_are_skipped() {
    let mut b = battle(4, 1);
    let knight = add(
        &mut b,
        Team::Player,
        0,
        0,
        BramGatewarden,
        &[card(MyHpBelow(Threshold::P50), Taunt), card(Always, Guard)],
    );
    dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), knight), Some(CardRef::Node(1)));
}

#[test]
fn locked_default_rule_advances_then_attacks() {
    let mut b = battle(3, 1);
    let knight = add(&mut b, Team::Player, 0, 0, BramGatewarden, &[]);
    let target = dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    let first = b.step();
    assert_eq!(first_fired(&first, knight), Some(CardRef::Default));
    assert!(first.contains(&Event::Moved {
        unit: knight,
        from: Pos::new(0, 0),
        to: Pos::new(1, 0)
    }));
    let second = b.step();
    assert_eq!(damage_to(&second, target), vec![attack_of(BramGatewarden)]);
}

#[test]
fn card_fired_comes_before_its_effects() {
    let mut b = battle(2, 1);
    let pip =
        add(&mut b, Team::Player, 0, 0, PipQuickfeather, &[card(Always, Attack(Target::Nearest))]);
    dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    let events = b.step();
    assert_eq!(events[0], Event::CardFired { unit: pip, card: CardRef::Node(0) });
    assert!(matches!(events[1], Event::Attacked { .. }));
}

#[test]
fn abilities_go_on_cooldown() {
    let mut b = battle(4, 1);
    let pip = add(
        &mut b,
        Team::Player,
        0,
        0,
        PipQuickfeather,
        &[card(Always, AimedShot(Target::Nearest)), card(Always, Guard)],
    );
    let enemy = dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    unit_mut(&mut b, enemy).hp = 100_000;
    unit_mut(&mut b, enemy).stats.max_hp = 100_000;
    let cooldown = Ability::AimedShot.cooldown() as usize;
    let cards: Vec<_> = (0..=cooldown).map(|_| first_fired(&b.step(), pip).unwrap()).collect();
    assert_eq!(cards[0], CardRef::Node(0));
    assert!(cards[1..cooldown].iter().all(|&c| c == CardRef::Node(1)));
    assert_eq!(cards[cooldown], CardRef::Node(0));
}

#[test]
fn outnumbered_counts_units_within_two_cells() {
    let mut b = battle(4, 4);
    let knight = add(&mut b, Team::Player, 0, 0, BramGatewarden, &[card(Outnumbered, Guard)]);
    dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 0, 1, BramGatewarden);
    assert_eq!(first_fired(&b.step(), knight), Some(CardRef::Node(0)));
}

#[test]
fn enemy_weak_and_was_hit_conditions() {
    let mut b = battle(3, 1);
    let pip = add(
        &mut b,
        Team::Player,
        0,
        0,
        PipQuickfeather,
        &[card(EnemyWeak, Guard), card(WasHit, Guard)],
    );
    let enemy = dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), pip), Some(CardRef::Default));
    let max = max_hp_of(BramGatewarden);
    unit_mut(&mut b, enemy).hp = max / 5;
    assert_eq!(first_fired(&b.step(), pip), Some(CardRef::Node(0)));

    let mut b = battle(2, 1);
    let bram = add(&mut b, Team::Player, 0, 0, BramGatewarden, &[card(WasHit, Guard)]);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[]);
    b.step(); // Bram attacks first, then Grub hits him
    assert_eq!(first_fired(&b.step(), bram), Some(CardRef::Node(0)));
}

// ----- Targeting -------------------------------------------------------------

#[test]
fn weakest_target_picks_lowest_hp() {
    let mut b = battle(4, 3);
    add(&mut b, Team::Player, 0, 1, PipQuickfeather, &[card(Always, Attack(Target::Weakest))]);
    dummy(&mut b, Team::Enemy, 1, 1, BramGatewarden);
    let weak = dummy(&mut b, Team::Enemy, 3, 1, Tobble);
    assert_eq!(damage_to(&b.step(), weak), vec![attack_of(PipQuickfeather)]);
}

#[test]
fn ranged_target_picks_backliners() {
    let mut b = battle(5, 1);
    add(&mut b, Team::Player, 0, 0, PipQuickfeather, &[card(Always, Attack(Target::Ranged))]);
    dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    let cleric = dummy(&mut b, Team::Enemy, 3, 0, SisterAgnes);
    assert_eq!(damage_to(&b.step(), cleric).len(), 1);
}

#[test]
fn taunt_overrides_target_choice() {
    let mut b = battle(4, 2);
    let knight = add(&mut b, Team::Player, 0, 0, BramGatewarden, &[card(Always, Taunt)]);
    let cleric = dummy(&mut b, Team::Player, 0, 1, SisterAgnes);
    add(&mut b, Team::Enemy, 2, 0, PipQuickfeather, &[card(Always, Attack(Target::Weakest))]);
    let events = b.step();
    assert_eq!(damage_to(&events, knight), vec![attack_of(PipQuickfeather)]);
    assert!(damage_to(&events, cleric).is_empty());
}

#[test]
fn marked_target_follows_the_teams_rangers() {
    let mut b = battle(3, 4);
    add(&mut b, Team::Player, 0, 0, PipQuickfeather, &[card(Always, Attack(Target::Nearest))]);
    add(&mut b, Team::Player, 0, 3, Elowen, &[card(Always, Attack(Target::Marked))]);
    let bram = dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    let grub = dummy(&mut b, Team::Enemy, 1, 3, Grub);
    let events = b.step();
    assert_eq!(damage_to(&events, bram).len(), 2, "both Rangers hit the marked enemy");
    assert!(damage_to(&events, grub).is_empty());
}

#[test]
fn synergy_cards_are_skipped_without_the_synergy() {
    let mut b = battle(3, 4);
    let elowen = add(&mut b, Team::Player, 0, 3, Elowen, &[card(Always, Attack(Target::Marked))]);
    dummy(&mut b, Team::Enemy, 1, 3, Grub);
    assert_eq!(first_fired(&b.step(), elowen), Some(CardRef::Default));
}

#[test]
fn vanish_makes_the_widow_untargetable_at_first() {
    let mut b = battle(3, 1);
    add(&mut b, Team::Player, 0, 0, PipQuickfeather, &[card(Always, Attack(Target::Nearest))]);
    let widow = dummy(&mut b, Team::Enemy, 1, 0, GreyWidow);
    let bram = dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    let first = b.step();
    assert_eq!(damage_to(&first, bram).len(), 1);
    assert!(damage_to(&first, widow).is_empty());
    b.step();
    assert_eq!(damage_to(&b.step(), widow).len(), 1, "targetable from tick 3");
}

// ----- Defences --------------------------------------------------------------

#[test]
fn guarding_halves_damage_rounding_up() {
    let mut b = battle(2, 1);
    let knight = add(&mut b, Team::Player, 0, 0, BramGatewarden, &[card(Always, Guard)]);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), knight), vec![attack_of(Grub).div_ceil(2)]);
}

#[test]
fn shield_wall_protects_adjacent_allies() {
    let mut b = battle(3, 2);
    add(&mut b, Team::Player, 0, 0, SirAldric, &[card(Always, ShieldWall)]);
    let cleric = dummy(&mut b, Team::Player, 0, 1, SisterAgnes);
    add(&mut b, Team::Enemy, 1, 1, Nettle, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), cleric), vec![attack_of(Nettle).div_ceil(2)]);
}

#[test]
fn sanctuary_blocks_all_damage() {
    let mut b = battle(2, 1);
    let calloway = add(
        &mut b,
        Team::Player,
        0,
        0,
        BrotherCalloway,
        &[card(Always, Sanctuary(Target::Nearest))],
    );
    add(&mut b, Team::Enemy, 1, 0, Nettle, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), calloway), vec![0]);
}

// ----- Signature abilities -----------------------------------------------------

#[test]
fn heal_skips_full_health_and_heals_the_most_hurt() {
    let mut b = battle(4, 1);
    let agnes = add(
        &mut b,
        Team::Player,
        0,
        0,
        SisterAgnes,
        &[card(Always, Heal(Target::Weakest)), card(Always, Guard)],
    );
    let knight = dummy(&mut b, Team::Player, 1, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), agnes), Some(CardRef::Node(1)), "nobody is hurt yet");
    unit_mut(&mut b, knight).hp = max_hp_of(BramGatewarden) / 2;
    assert_eq!(healed(&b.step(), knight), attack_of(SisterAgnes) * 4);
}

#[test]
fn kite_attacks_then_steps_back() {
    let mut b = battle(5, 1);
    let kestrel = add(&mut b, Team::Player, 1, 0, Kestrel, &[card(Always, Kite(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    let events = b.step();
    assert_eq!(damage_to(&events, target).len(), 1);
    assert!(events.contains(&Event::Moved {
        unit: kestrel,
        from: Pos::new(1, 0),
        to: Pos::new(0, 0)
    }));
}

#[test]
fn volley_hits_every_enemy_in_range() {
    let mut b = battle(6, 3);
    add(&mut b, Team::Player, 0, 1, Elowen, &[card(Always, Volley)]);
    let near = dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    let edge = dummy(&mut b, Team::Enemy, 4, 1, BramGatewarden);
    let far = dummy(&mut b, Team::Enemy, 5, 2, BramGatewarden);
    let events = b.step();
    let half = attack_of(Elowen) / 2;
    assert_eq!(damage_to(&events, near), vec![half]);
    assert_eq!(damage_to(&events, edge), vec![half]);
    assert!(damage_to(&events, far).is_empty());
}

#[test]
fn firebolt_hits_hard() {
    let mut b = battle(4, 1);
    add(&mut b, Team::Player, 0, 0, Tobble, &[card(Always, Firebolt(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Tobble) * 5 / 2]);
}

#[test]
fn frost_nova_freezes_and_chill_adds_damage() {
    let mut b = battle(3, 1);
    add(
        &mut b,
        Team::Player,
        0,
        0,
        Mirelle,
        &[card(EnemyAdjacent, FrostNova), card(Always, Attack(Target::Nearest))],
    );
    let grub = add(&mut b, Team::Enemy, 1, 0, Grub, &[]);
    let first = b.step();
    assert_eq!(damage_to(&first, grub), vec![attack_of(Mirelle) / 2]);
    let second = b.step();
    assert!(second.contains(&Event::TurnSkipped { unit: grub }));
    assert_eq!(damage_to(&second, grub), vec![attack_of(Mirelle) * 120 / 100], "frozen: +20%");
}

#[test]
fn fireball_splashes_neighbours_of_the_target() {
    let mut b = battle(5, 3);
    add(&mut b, Team::Player, 0, 1, Vorn, &[card(Always, Fireball(Target::Nearest))]);
    let main = dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    let next_to = dummy(&mut b, Team::Enemy, 3, 1, BramGatewarden);
    let far = dummy(&mut b, Team::Enemy, 4, 2, BramGatewarden);
    let events = b.step();
    let attack = attack_of(Vorn);
    assert_eq!(damage_to(&events, main), vec![attack]);
    assert_eq!(damage_to(&events, next_to), vec![attack / 2]);
    assert!(damage_to(&events, far).is_empty());
}

#[test]
fn blink_escapes_to_the_safest_cell() {
    let mut b = battle(7, 1);
    let vorn = add(&mut b, Team::Player, 1, 0, Vorn, &[card(Always, Blink)]);
    dummy(&mut b, Team::Enemy, 0, 0, BramGatewarden);
    let events = b.step();
    assert!(events.contains(&Event::Teleported {
        unit: vorn,
        from: Pos::new(1, 0),
        to: Pos::new(4, 0)
    }));
}

#[test]
fn second_wind_heals_only_when_hurt() {
    let mut b = battle(3, 1);
    let ysolde =
        add(&mut b, Team::Player, 0, 0, Ysolde, &[card(Always, SecondWind), card(Always, Guard)]);
    dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), ysolde), Some(CardRef::Node(1)));
    let max = max_hp_of(Ysolde);
    unit_mut(&mut b, ysolde).hp = max / 2;
    assert_eq!(healed(&b.step(), ysolde), max * 30 / 100);
}

#[test]
fn mass_heal_heals_every_hurt_ally() {
    let mut b = battle(5, 1);
    add(&mut b, Team::Player, 0, 0, Seraphine, &[card(Always, MassHeal)]);
    let a = dummy(&mut b, Team::Player, 1, 0, BramGatewarden);
    let c = dummy(&mut b, Team::Player, 2, 0, PipQuickfeather);
    dummy(&mut b, Team::Enemy, 4, 0, BramGatewarden);
    unit_mut(&mut b, a).hp = 10;
    unit_mut(&mut b, c).hp = 10;
    let events = b.step();
    let amount = attack_of(Seraphine) * 2;
    assert_eq!(healed(&events, a), amount);
    assert_eq!(healed(&events, c), amount);
}

#[test]
fn backstab_needs_the_target_engaged_by_an_ally() {
    let mut b = battle(3, 1);
    let nettle = add(
        &mut b,
        Team::Player,
        0,
        0,
        Nettle,
        &[card(Always, Backstab(Target::Nearest)), card(Always, Guard)],
    );
    dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), nettle), Some(CardRef::Node(1)), "target isn't engaged");

    let mut b = battle(3, 1);
    add(&mut b, Team::Player, 0, 0, Nettle, &[card(Always, Backstab(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    dummy(&mut b, Team::Player, 2, 0, BramGatewarden);
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Nettle) * 3]);
}

#[test]
fn shadowstep_lands_next_to_the_target() {
    let mut b = battle(5, 3);
    let vex = add(&mut b, Team::Player, 0, 0, Vex, &[card(Always, Shadowstep(Target::Nearest))]);
    dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    b.step();
    let pos = b.unit(vex).unwrap().pos;
    assert!(pos.is_adjacent(Pos::new(2, 1)));
    assert_eq!(pos.distance(Pos::new(0, 0)), 4, "should land on the far side");
}

#[test]
fn assassinate_finishes_a_weak_enemy() {
    let mut b = battle(5, 1);
    let widow = add(
        &mut b,
        Team::Player,
        0,
        0,
        GreyWidow,
        &[card(Always, Assassinate), card(Always, Guard)],
    );
    let target = dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), widow), Some(CardRef::Node(1)), "nobody weak yet");
    unit_mut(&mut b, target).hp = max_hp_of(BramGatewarden) / 5;
    let events = b.step();
    assert!(events.iter().any(|e| matches!(e, Event::Teleported { unit, .. } if *unit == widow)));
    assert_eq!(damage_to(&events, target), vec![attack_of(GreyWidow) * 3]);
}

#[test]
fn frenzy_doubles_basic_attacks() {
    let mut b = battle(2, 1);
    add(
        &mut b,
        Team::Player,
        0,
        0,
        Hilda,
        &[card(Always, Frenzy), card(Always, Attack(Target::Nearest))],
    );
    let target = dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    b.step();
    assert_eq!(damage_to(&b.step(), target).len(), 2);
}

#[test]
fn cleave_and_rampage_hit_every_adjacent_enemy() {
    let mut b = battle(3, 3);
    add(&mut b, Team::Player, 1, 1, Grub, &[card(Always, Cleave)]);
    let foes = [
        dummy(&mut b, Team::Enemy, 0, 1, BramGatewarden),
        dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden),
        dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden),
    ];
    let events = b.step();
    for foe in foes {
        assert_eq!(damage_to(&events, foe), vec![attack_of(Grub)]);
    }

    let mut b = battle(3, 1);
    let korgath = add(&mut b, Team::Player, 1, 0, Korgath, &[card(Always, Rampage)]);
    dummy(&mut b, Team::Enemy, 0, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    let events = b.step();
    assert_eq!(events.iter().filter(|e| matches!(e, Event::Attacked { .. })).count(), 2);
    assert!(b.unit(korgath).unwrap().statuses.frenzied_until > 0);
}

// ----- Passives ----------------------------------------------------------------

#[test]
fn bulwark_reduces_damage_per_adjacent_ally() {
    let mut b = battle(3, 2);
    let aldric = dummy(&mut b, Team::Player, 1, 1, SirAldric);
    dummy(&mut b, Team::Player, 0, 1, BramGatewarden);
    dummy(&mut b, Team::Player, 1, 0, SisterAgnes);
    add(&mut b, Team::Enemy, 2, 1, Grub, &[card(Always, Attack(Target::Nearest))]);
    assert_eq!(damage_to(&b.step(), aldric), vec![attack_of(Grub) * 80 / 100]);
}

#[test]
fn last_stand_halves_damage_when_low() {
    let mut b = battle(2, 1);
    let ysolde = dummy(&mut b, Team::Player, 0, 0, Ysolde);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    unit_mut(&mut b, ysolde).hp = max_hp_of(Ysolde) / 5;
    assert_eq!(damage_to(&b.step(), ysolde), vec![attack_of(Grub).div_ceil(2)]);
}

#[test]
fn hunters_mark_makes_allies_hit_harder() {
    let mut b = battle(4, 2);
    add(&mut b, Team::Player, 0, 0, Kestrel, &[card(Always, Attack(Target::Nearest))]);
    add(&mut b, Team::Player, 0, 1, PipQuickfeather, &[card(Always, Attack(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    let events = b.step();
    let hits = damage_to(&events, target);
    assert_eq!(hits[0], attack_of(Kestrel));
    assert_eq!(hits[1], attack_of(PipQuickfeather) * 115 / 100);
}

#[test]
fn opportunist_punishes_wounded_enemies() {
    let mut b = battle(2, 1);
    add(&mut b, Team::Player, 0, 0, Vex, &[card(Always, Attack(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    unit_mut(&mut b, target).hp = max_hp_of(BramGatewarden) / 3;
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Vex) * 130 / 100]);
}

#[test]
fn arcane_surge_makes_every_third_ability_crit() {
    let mut b = battle(4, 1);
    let vorn = add(&mut b, Team::Player, 0, 0, Vorn, &[card(Always, Fireball(Target::Nearest))]);
    let target = dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    unit_mut(&mut b, target).stats.max_hp = 1_000_000;
    unit_mut(&mut b, target).hp = 1_000_000;
    let mut fireball_crits = Vec::new();
    while fireball_crits.len() < 3 {
        let events = b.step();
        if events.iter().any(|e| matches!(e, Event::AbilityUsed { unit, .. } if *unit == vorn)) {
            fireball_crits
                .push(events.iter().any(|e| matches!(e, Event::Attacked { crit: true, .. })));
        }
    }
    assert_eq!(fireball_crits, vec![false, false, true]);
}

#[test]
fn bloodlust_heals_when_wounded() {
    let mut b = battle(2, 1);
    let hilda = add(&mut b, Team::Player, 0, 0, Hilda, &[card(Always, Attack(Target::Nearest))]);
    dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    unit_mut(&mut b, hilda).hp = 10;
    assert_eq!(healed(&b.step(), hilda), attack_of(Hilda) / 5);
}

#[test]
fn undying_survives_one_killing_blow() {
    let mut b = battle(2, 1);
    let korgath = dummy(&mut b, Team::Player, 0, 0, Korgath);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    unit_mut(&mut b, korgath).hp = 5;
    let events = b.step();
    assert!(events.contains(&Event::CheatedDeath { unit: korgath }));
    assert_eq!(b.unit(korgath).unwrap().hp, 1);
    let events = b.step();
    assert!(events.contains(&Event::Died { unit: korgath }));
}

#[test]
fn resurrection_revives_the_first_ally_to_fall_once() {
    let mut b = battle(3, 2);
    dummy(&mut b, Team::Player, 0, 1, Seraphine);
    let bram = dummy(&mut b, Team::Player, 0, 0, BramGatewarden);
    let pip = dummy(&mut b, Team::Player, 2, 1, PipQuickfeather);
    add(&mut b, Team::Enemy, 1, 0, Grub, &[card(Always, Cleave)]);
    unit_mut(&mut b, bram).hp = 1;
    unit_mut(&mut b, pip).hp = 1;
    // Grub at (1,0) is adjacent to Bram (0,0) only; move Pip next to him too.
    unit_mut(&mut b, pip).pos = Pos::new(2, 0);
    let events = b.step();
    let revived: Vec<_> = events.iter().filter(|e| matches!(e, Event::Revived { .. })).collect();
    assert_eq!(revived.len(), 1, "only the first to fall comes back");
    assert!(b.unit(bram).unwrap().is_alive());
    assert!(!b.unit(pip).unwrap().is_alive());
}

// ----- Synergies -----------------------------------------------------------------

#[test]
fn three_knights_unlock_a_fifth_card() {
    let program = [
        card(MyHpBelow(Threshold::P25), Taunt),
        card(MyHpBelow(Threshold::P25), Guard),
        card(MyHpBelow(Threshold::P25), Guard),
        card(MyHpBelow(Threshold::P25), Guard),
        card(Always, Guard),
    ];
    let mut b = battle(5, 2);
    let bram = add(&mut b, Team::Player, 0, 0, BramGatewarden, &program);
    dummy(&mut b, Team::Player, 1, 0, SirAldric);
    dummy(&mut b, Team::Player, 2, 0, Ysolde);
    dummy(&mut b, Team::Enemy, 4, 1, Grub);
    assert_eq!(first_fired(&b.step(), bram), Some(CardRef::Node(4)));

    let mut b = battle(5, 2);
    let bram = add(&mut b, Team::Player, 0, 0, BramGatewarden, &program);
    dummy(&mut b, Team::Enemy, 4, 1, Grub);
    assert_eq!(first_fired(&b.step(), bram), Some(CardRef::Default), "fifth card is skipped");
}

#[test]
fn three_rangers_gain_range() {
    let mut b = battle(6, 3);
    let pip = dummy(&mut b, Team::Player, 0, 0, PipQuickfeather);
    dummy(&mut b, Team::Player, 0, 1, Kestrel);
    dummy(&mut b, Team::Player, 0, 2, Elowen);
    dummy(&mut b, Team::Enemy, 5, 0, BramGatewarden);
    b.step();
    assert_eq!(b.unit(pip).unwrap().stats.range, 4);
}

#[test]
fn three_mages_cool_down_faster() {
    let mut b = battle(6, 3);
    let tobble =
        add(&mut b, Team::Player, 0, 0, Tobble, &[card(Always, Firebolt(Target::Nearest))]);
    dummy(&mut b, Team::Player, 0, 1, Mirelle);
    dummy(&mut b, Team::Player, 0, 2, Vorn);
    dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    b.step();
    let ready = b.unit(tobble).unwrap().ready_at[Ability::Firebolt.index()];
    assert_eq!(ready, 1 + Ability::Firebolt.cooldown() - 2);
}

#[test]
fn clerics_unlock_signals_and_three_heal_on_receipt() {
    let mut b = battle(6, 3);
    let agnes =
        add(&mut b, Team::Player, 0, 0, SisterAgnes, &[card(Always, SendSignal(Signal::Red))]);
    let calloway = add(
        &mut b,
        Team::Player,
        0,
        1,
        BrotherCalloway,
        &[card(SignalReceived(Signal::Red), Guard)],
    );
    dummy(&mut b, Team::Enemy, 5, 0, BramGatewarden);
    assert_eq!(first_fired(&b.step(), agnes), Some(CardRef::Node(0)));
    assert_eq!(first_fired(&b.step(), calloway), Some(CardRef::Node(0)), "signal seen next tick");

    let mut b = battle(6, 3);
    add(&mut b, Team::Player, 0, 0, SisterAgnes, &[card(Always, SendSignal(Signal::Red))]);
    dummy(&mut b, Team::Player, 0, 1, BrotherCalloway);
    let sera = dummy(&mut b, Team::Player, 0, 2, Seraphine);
    dummy(&mut b, Team::Enemy, 5, 0, BramGatewarden);
    unit_mut(&mut b, sera).hp = 10;
    b.step();
    assert_eq!(healed(&b.step(), sera), max_hp_of(Seraphine) / 10);
}

#[test]
fn three_rogues_double_crit_chance() {
    let mut b = Battle::new(6, 3, 100, 0);
    let nettle = dummy(&mut b, Team::Player, 0, 0, Nettle);
    dummy(&mut b, Team::Player, 0, 1, Vex);
    dummy(&mut b, Team::Player, 0, 2, GreyWidow);
    dummy(&mut b, Team::Enemy, 5, 0, BramGatewarden);
    b.step();
    assert_eq!(b.unit(nettle).unwrap().crit_chance, CRIT_CHANCE_PERCENT * 2);
}

#[test]
fn fury_chains_the_next_card() {
    let program = [card(Surrounded, Cleave), card(Always, Attack(Target::Weakest))];
    let mut b = battle(3, 3);
    let grub = add(&mut b, Team::Player, 1, 1, Grub, &program);
    dummy(&mut b, Team::Player, 0, 0, Hilda);
    dummy(&mut b, Team::Player, 2, 2, Korgath);
    dummy(&mut b, Team::Enemy, 0, 1, BramGatewarden);
    dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    let events = b.step();
    assert_eq!(fired(&events, grub), vec![CardRef::Node(0), CardRef::Node(1)]);

    let mut b = battle(3, 3);
    let grub = add(&mut b, Team::Player, 1, 1, Grub, &program);
    dummy(&mut b, Team::Player, 0, 0, Hilda);
    dummy(&mut b, Team::Enemy, 0, 1, BramGatewarden);
    dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    assert_eq!(fired(&b.step(), grub), vec![CardRef::Node(0)], "two Berserkers: no Fury");
}

#[test]
fn class_only_synergy_blocks_are_skipped_for_other_classes() {
    let mut b = battle(3, 3);
    let nettle = add(&mut b, Team::Player, 1, 1, Nettle, &[card(Surrounded, Guard)]);
    dummy(&mut b, Team::Player, 0, 0, Grub);
    dummy(&mut b, Team::Player, 2, 2, Hilda);
    dummy(&mut b, Team::Enemy, 0, 1, BramGatewarden);
    dummy(&mut b, Team::Enemy, 2, 1, BramGatewarden);
    assert_eq!(first_fired(&b.step(), nettle), Some(CardRef::Default));
}

#[test]
fn stalemate_ends_in_a_draw_at_the_tick_limit() {
    let mut b = Battle::new(4, 1, 5, 0);
    dummy(&mut b, Team::Player, 0, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    assert_eq!(b.run_to_end(), Outcome::Draw);
    assert_eq!(b.tick(), 5);
}

// ----- The `it` target ---------------------------------------------------

#[test]
fn it_targets_the_enemy_the_condition_spotted() {
    // A healthy enemy is nearer, but `enemy weak -> Attack it` hits the weak one.
    let mut b = battle(4, 1);
    let pip =
        add(&mut b, Team::Player, 0, 0, PipQuickfeather, &[card(EnemyWeak, Attack(Target::It))]);
    let near = dummy(&mut b, Team::Enemy, 1, 0, BramGatewarden);
    let weak = dummy(&mut b, Team::Enemy, 3, 0, BramGatewarden);
    unit_mut(&mut b, weak).hp = max_hp_of(BramGatewarden) / 5;
    let events = b.step();
    assert_eq!(first_fired(&events, pip), Some(CardRef::Node(0)));
    assert!(damage_to(&events, near).is_empty());
    assert_eq!(damage_to(&events, weak).len(), 1);
}

#[test]
fn heal_it_heals_the_ally_the_condition_spotted() {
    let mut b = battle(3, 1);
    let agnes = add(
        &mut b,
        Team::Player,
        0,
        0,
        SisterAgnes,
        &[card(AllyHpBelow(Threshold::P50), Heal(Target::It))],
    );
    let hurt = dummy(&mut b, Team::Player, 1, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    unit_mut(&mut b, hurt).hp = max_hp_of(BramGatewarden) / 3;
    let events = b.step();
    assert_eq!(first_fired(&events, agnes), Some(CardRef::Node(0)));
    assert!(healed(&events, hurt) > 0);
}

#[test]
fn was_hit_it_strikes_back_at_the_attacker() {
    // Both enemies are adjacent; the dummy has the lower id, so `nearest`
    // would pick it. `it` picks Grub, who did the hitting.
    let mut b = battle(3, 2);
    let bram = add(&mut b, Team::Player, 1, 0, BramGatewarden, &[card(WasHit, Attack(Target::It))]);
    let bystander = dummy(&mut b, Team::Enemy, 0, 0, BramGatewarden);
    let grub = add(&mut b, Team::Enemy, 2, 0, Grub, &[]);
    unit_mut(&mut b, bystander).hp = 100_000;
    unit_mut(&mut b, grub).hp = 100_000;
    b.step(); // Bram isn't hit yet, so he attacks the nearest; then Grub hits him.
    let events = b.step();
    assert_eq!(first_fired(&events, bram), Some(CardRef::Node(0)));
    assert_eq!(damage_to(&events, grub).len(), 1);
    assert!(damage_to(&events, bystander).is_empty());
}

#[test]
fn it_needs_a_condition_that_spots_the_right_side() {
    let no_subject = Program::new([card(Always, Attack(Target::It))]);
    assert_eq!(
        no_subject.validate_basic(PipQuickfeather),
        Err(ProgramError::NoSubject { index: 0 })
    );
    let wrong_side = Program::new([card(EnemyWeak, Heal(Target::It))]);
    assert_eq!(wrong_side.validate_basic(SisterAgnes), Err(ProgramError::NoSubject { index: 0 }));
    let fine = Program::new([
        card(AllyHpBelow(Threshold::P50), Heal(Target::It)),
        card(EnemyAdjacent, Attack(Target::It)),
    ]);
    assert_eq!(fine.validate_basic(SisterAgnes), Ok(()));
}

// ----- Items ----------------------------------------------------------------

fn add_with(
    b: &mut Battle,
    team: Team,
    x: i32,
    y: i32,
    who: Character,
    nodes: &[Node],
    items: &[Item],
) -> UnitId {
    let mut slots = [None; ITEM_SLOTS];
    for (slot, item) in slots.iter_mut().zip(items) {
        *slot = Some(*item);
    }
    let spec = UnitSpec::new(who, Level::Three, Program::new(nodes.to_vec())).with_items(slots);
    b.spawn(team, Pos::new(x, y), spec).expect("valid test unit")
}

fn unit_of(b: &Battle, id: UnitId) -> &Unit {
    b.units().iter().find(|u| u.id == id).unwrap()
}

/// A tough dummy that soaks hits without dying.
fn soak(b: &mut Battle, team: Team, x: i32, y: i32) -> UnitId {
    let id = dummy(b, team, x, y, BramGatewarden);
    unit_mut(b, id).stats.max_hp = 100_000;
    unit_mut(b, id).hp = 100_000;
    id
}

#[test]
fn stat_items_change_stats_at_spawn() {
    let mut b = Battle::new(4, 2, 50, 0);
    let plain = add(&mut b, Team::Player, 0, 0, Grub, &[]);
    let geared =
        add_with(&mut b, Team::Player, 0, 1, Grub, &[], &[Item::IronSword, Item::LeatherJerkin]);
    let ranged =
        add_with(&mut b, Team::Player, 1, 0, Kestrel, &[], &[Item::Spyglass, Item::LuckyCoin]);
    let (p, g) = (unit_of(&b, plain).stats, unit_of(&b, geared).stats);
    assert_eq!(g.attack, p.attack * 115 / 100);
    assert_eq!(g.max_hp, p.max_hp * 120 / 100);
    assert_eq!(unit_of(&b, geared).hp, g.max_hp);
    let k = unit_of(&b, ranged);
    assert_eq!(k.stats.range, Level::Three.scale(Kestrel.base_stats()).range + 1);
    assert_eq!(k.crit_chance, CRIT_CHANCE_PERCENT + 10);
}

#[test]
fn twin_blades_hit_twice_at_sixty_percent() {
    let mut b = battle(2, 1);
    add_with(
        &mut b,
        Team::Player,
        0,
        0,
        Grub,
        &[card(Always, Attack(Target::Nearest))],
        &[Item::TwinBlades],
    );
    let target = soak(&mut b, Team::Enemy, 1, 0);
    let hit = attack_of(Grub) * 60 / 100;
    assert_eq!(damage_to(&b.step(), target), vec![hit, hit]);
}

#[test]
fn whetstone_doubles_only_the_first_hit() {
    let mut b = battle(2, 1);
    add_with(
        &mut b,
        Team::Player,
        0,
        0,
        Grub,
        &[card(Always, Attack(Target::Nearest))],
        &[Item::Whetstone],
    );
    let target = soak(&mut b, Team::Enemy, 1, 0);
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Grub) * 2]);
    assert_eq!(damage_to(&b.step(), target), vec![attack_of(Grub)]);
}

#[test]
fn vampire_fang_heals_a_fifth_of_damage_dealt() {
    let mut b = battle(2, 1);
    let grub = add_with(
        &mut b,
        Team::Player,
        0,
        0,
        Grub,
        &[card(Always, Attack(Target::Nearest))],
        &[Item::VampireFang],
    );
    unit_mut(&mut b, grub).hp = 10;
    soak(&mut b, Team::Enemy, 1, 0);
    assert_eq!(healed(&b.step(), grub), attack_of(Grub) / 5);
}

#[test]
fn spiked_buckler_reflects_melee_damage_only() {
    let mut b = battle(3, 2);
    let grub = add(&mut b, Team::Player, 0, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    let wall = add_with(&mut b, Team::Enemy, 1, 0, BramGatewarden, &[], &[Item::SpikedBuckler]);
    unit_mut(&mut b, wall).inert = true;
    unit_mut(&mut b, wall).hp = 100_000;
    let events = b.step();
    let dealt = damage_to(&events, wall)[0];
    assert_eq!(damage_to(&events, grub), vec![dealt / 5]);

    // A ranged attacker two cells away takes nothing back.
    let mut b = battle(3, 1);
    let archer = add(&mut b, Team::Player, 0, 0, Kestrel, &[card(Always, Attack(Target::Nearest))]);
    let wall = add_with(&mut b, Team::Enemy, 2, 0, BramGatewarden, &[], &[Item::SpikedBuckler]);
    unit_mut(&mut b, wall).inert = true;
    assert!(damage_to(&b.step(), archer).is_empty());
}

#[test]
fn phoenix_feather_revives_once_at_forty_percent() {
    let mut b = battle(2, 1);
    add(&mut b, Team::Player, 0, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    let bird = add_with(&mut b, Team::Enemy, 1, 0, BramGatewarden, &[], &[Item::PhoenixFeather]);
    unit_mut(&mut b, bird).inert = true;
    unit_mut(&mut b, bird).hp = 1;
    let events = b.step();
    let max = unit_of(&b, bird).stats.max_hp;
    assert!(events.contains(&Event::Died { unit: bird }));
    assert!(events.contains(&Event::Revived { unit: bird, hp: max * 40 / 100 }));
    // The second death is final.
    unit_mut(&mut b, bird).hp = 1;
    let events = b.step();
    assert!(events.contains(&Event::Died { unit: bird }));
    assert!(!events.iter().any(|e| matches!(e, Event::Revived { .. })));
}

#[test]
fn healing_draught_heals_once_below_forty_percent() {
    let mut b = battle(2, 1);
    add(&mut b, Team::Player, 0, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    let drinker = add_with(&mut b, Team::Enemy, 1, 0, BramGatewarden, &[], &[Item::HealingDraught]);
    unit_mut(&mut b, drinker).inert = true;
    let max = unit_of(&b, drinker).stats.max_hp;
    // Just above 40%: one hit takes it below.
    unit_mut(&mut b, drinker).hp = max * 40 / 100 + 1;
    let events = b.step();
    assert_eq!(healed(&events, drinker), max * 30 / 100);
    // Already drunk.
    unit_mut(&mut b, drinker).hp = max * 40 / 100 + 1;
    assert_eq!(healed(&b.step(), drinker), 0);
}

#[test]
fn aegis_blocks_the_first_two_ticks_then_softens_hits() {
    let mut b = battle(2, 1);
    add(&mut b, Team::Player, 0, 0, Grub, &[card(Always, Attack(Target::Nearest))]);
    let guard = add_with(&mut b, Team::Enemy, 1, 0, BramGatewarden, &[], &[Item::Aegis]);
    unit_mut(&mut b, guard).inert = true;
    unit_mut(&mut b, guard).hp = 100_000;
    assert_eq!(damage_to(&b.step(), guard), vec![0]);
    assert_eq!(damage_to(&b.step(), guard), vec![0]);
    assert_eq!(damage_to(&b.step(), guard), vec![attack_of(Grub) * 85 / 100]);
}

#[test]
fn hourglass_shortens_signature_cooldowns() {
    let mut b = battle(4, 1);
    let pip = add_with(
        &mut b,
        Team::Player,
        0,
        0,
        PipQuickfeather,
        &[card(Always, AimedShot(Target::Nearest)), card(Always, Guard)],
        &[Item::Hourglass],
    );
    soak(&mut b, Team::Enemy, 3, 0);
    let cooldown = Ability::AimedShot.cooldown() as usize - 1;
    let cards: Vec<_> = (0..=cooldown).map(|_| first_fired(&b.step(), pip).unwrap()).collect();
    assert_eq!(cards[0], CardRef::Node(0));
    assert_eq!(cards[cooldown], CardRef::Node(0), "ready a tick sooner");
}

#[test]
fn banners_count_their_class_for_synergies() {
    let mut b = Battle::new(4, 3, 50, 0);
    add(&mut b, Team::Player, 0, 0, BramGatewarden, &[]);
    assert!(!b.synergies(Team::Player).active(Class::Knight));
    add_with(&mut b, Team::Player, 0, 1, Grub, &[], &[Item::Banner(Class::Knight)]);
    let syn = b.synergies(Team::Player);
    assert_eq!(syn.count(Class::Knight), 2);
    assert_eq!(syn.count(Class::Berserker), 1, "still counts as its own class");
}

// ----- Pathfinding ------------------------------------------------------------

fn moved_to(events: &[Event], unit: UnitId) -> Option<Pos> {
    events.iter().find_map(|e| match e {
        Event::Moved { unit: u, to, .. } if *u == unit => Some(*to),
        _ => None,
    })
}

#[test]
fn advancing_walks_around_a_blocking_ally() {
    // Grub, an ally standing in front of him, and the enemy behind that ally.
    let mut b = battle(4, 3);
    let grub = add(&mut b, Team::Player, 0, 1, Grub, &[]);
    dummy(&mut b, Team::Player, 1, 1, BramGatewarden);
    dummy(&mut b, Team::Enemy, 3, 1, BramGatewarden);
    let to = moved_to(&b.step(), grub).expect("finds a way round instead of standing still");
    assert_eq!(to.x, 0, "steps sideways first");
    assert_ne!(to.y, 1);
}

#[test]
fn advancing_heads_for_a_reachable_enemy_when_the_nearest_is_walled_in() {
    // The nearest enemy (top right) is boxed in by its allies; a farther one
    // (bottom) can be reached.
    let mut b = battle(5, 4);
    let grub = add(&mut b, Team::Player, 0, 0, Grub, &[]);
    dummy(&mut b, Team::Enemy, 3, 0, Nettle);
    dummy(&mut b, Team::Enemy, 2, 0, BramGatewarden);
    dummy(&mut b, Team::Enemy, 3, 1, BramGatewarden);
    dummy(&mut b, Team::Enemy, 4, 0, BramGatewarden);
    // Grub can always reach someone, so he should never stand still.
    for _ in 0..3 {
        let events = b.step();
        let attacked = events
            .iter()
            .any(|e| matches!(e, Event::Attacked { attacker, .. } if *attacker == grub));
        assert!(attacked || moved_to(&events, grub).is_some());
    }
}

#[test]
fn pathfinding_is_deterministic() {
    let run = || {
        let mut b = battle(6, 4);
        let grub = add(&mut b, Team::Player, 0, 1, Grub, &[]);
        dummy(&mut b, Team::Player, 1, 1, BramGatewarden);
        dummy(&mut b, Team::Player, 1, 2, SirAldric);
        dummy(&mut b, Team::Enemy, 5, 2, BramGatewarden);
        (0..4).map(|_| moved_to(&b.step(), grub)).collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

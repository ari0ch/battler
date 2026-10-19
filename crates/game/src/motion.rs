//! Free-looking movement for battle playback.
//!
//! The sim moves units cell to cell on a grid, and that grid stays the source
//! of truth: every rule and replay is unchanged. This module only decides how
//! units are *drawn*. Each unit gets a [`Body`] with a free position and
//! velocity on the field. Every frame the body steers towards its unit's cell
//! (at a small, per-unit offset so formations don't look tiled), bodies that
//! overlap push each other apart, and attacks add a lunge towards the target
//! or a knock-back away from the attacker.
//!
//! All of this is presentation, so it uses `f32` and a `HashMap` freely; the
//! determinism rules apply to the sim, not to how it is animated.

use std::collections::HashMap;

use battler_sim::{Battle, Pos, Team, UnitId};
use bevy::math::Vec2;

/// How far a unit stands from its cell's centre, at most, in pixels.
const SCATTER: f32 = 10.0;
/// Top speed at 1x playback, in pixels per second (a cell in about 0.35 s).
const MAX_SPEED: f32 = 230.0;
/// How strongly a body is pulled towards where it wants to be.
const PULL: f32 = 9.0;
/// How quickly velocity catches up with the desired velocity (per second).
const RESPONSE: f32 = 14.0;
/// A melee lunge: how far towards the target, and how long it lasts.
const LUNGE_PX: f32 = 22.0;
const LUNGE_SECS: f32 = 0.28;
/// Below this speed a unit counts as standing still (no walk bob, team facing).
const STILL: f32 = 18.0;

/// One unit as drawn: where it is, how it moves and which way it faces.
#[derive(Debug, Clone)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    /// Collision radius: bodies closer than the sum of their radii are pushed apart.
    pub radius: f32,
    /// The unit's personal spot within its cell.
    offset: Vec2,
    /// A melee lunge in progress: towards whom, and seconds left.
    lunge: Option<(UnitId, f32)>,
    pub facing_left: bool,
    /// Distance walked, which drives the walking bob.
    pub stride: f32,
}

/// A small, stable offset for a unit, so the same unit always stands in the
/// same spot of its cell. A few rounds of integer hashing turn the id into two
/// numbers in -1..1 (`wrapping_mul` keeps the overflow defined).
fn scatter(id: UnitId) -> Vec2 {
    let mut h = id.0.wrapping_mul(0x9E37_79B9) ^ 0x85EB_CA6B;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    let a = (h & 0xFFFF) as f32 / 65535.0 * 2.0 - 1.0;
    let b = (h >> 16) as f32 / 65535.0 * 2.0 - 1.0;
    Vec2::new(a, b) * SCATTER
}

/// Bodies for every unit in a battle, standing at their cells.
pub fn spawn(
    battle: &Battle,
    centre: impl Fn(Pos) -> Vec2,
    radius: impl Fn(&battler_sim::Unit) -> f32,
) -> HashMap<UnitId, Body> {
    battle
        .units()
        .iter()
        .map(|u| {
            let offset = scatter(u.id);
            let body = Body {
                pos: centre(u.pos) + offset,
                vel: Vec2::ZERO,
                radius: radius(u),
                offset,
                lunge: None,
                facing_left: u.team == Team::Enemy,
                stride: 0.0,
            };
            (u.id, body)
        })
        .collect()
}

/// Starts a melee lunge from `attacker` towards `target`.
pub fn lunge(bodies: &mut HashMap<UnitId, Body>, attacker: UnitId, target: UnitId) {
    if let Some(b) = bodies.get_mut(&attacker) {
        b.lunge = Some((target, LUNGE_SECS));
    }
}

/// Knocks `target` away from `from` with an instant push of `speed` px/s.
pub fn knock(bodies: &mut HashMap<UnitId, Body>, target: UnitId, from: UnitId, speed: f32) {
    let Some(from_pos) = bodies.get(&from).map(|b| b.pos) else { return };
    if let Some(b) = bodies.get_mut(&target) {
        b.vel += (b.pos - from_pos).normalize_or_zero() * speed;
    }
}

/// Advances every living unit's body by `dt` seconds of playback time.
///
/// `centre` maps a sim cell to its centre on the field, and `size` is the
/// field's width and height; bodies stay inside it.
pub fn step(
    bodies: &mut HashMap<UnitId, Body>,
    battle: &Battle,
    centre: impl Fn(Pos) -> Vec2,
    size: Vec2,
    dt: f32,
) {
    if dt <= 0.0 {
        return;
    }
    let alive: Vec<&battler_sim::Unit> = battle.units().iter().filter(|u| u.is_alive()).collect();
    // Copy the positions first: a lunge needs to read another body's position
    // while this one is being changed, which the borrow checker (rightly)
    // won't allow on the same map.
    let positions: HashMap<UnitId, Vec2> = bodies.iter().map(|(id, b)| (*id, b.pos)).collect();

    for u in &alive {
        let Some(b) = bodies.get_mut(&u.id) else { continue };
        let mut target = centre(u.pos) + b.offset;
        let mut facing_target = None;
        if let Some((victim, left)) = b.lunge.as_mut() {
            *left -= dt;
            if let Some(&v) = positions.get(victim) {
                facing_target = Some(v);
                let reach = (std::f32::consts::PI * left.max(0.0) / LUNGE_SECS).sin().max(0.0);
                target += (v - b.pos).normalize_or_zero() * LUNGE_PX * reach;
            }
        }
        if b.lunge.is_some_and(|(_, left)| left <= 0.0) {
            b.lunge = None;
        }
        // Steer: the desired velocity points at the target, capped at top
        // speed, and the real velocity eases towards it (no instant turns).
        let desired = ((target - b.pos) * PULL).clamp_length_max(MAX_SPEED);
        let blend = 1.0 - (-RESPONSE * dt).exp();
        b.vel = b.vel.lerp(desired, blend);
        b.pos += b.vel * dt;
        b.stride += b.vel.length() * dt;
        // Face the enemy being attacked, else the way it's walking, else the
        // team's default (towards the other side).
        b.facing_left = match facing_target {
            Some(v) => v.x < b.pos.x,
            None if b.vel.x.abs() > STILL => b.vel.x < 0.0,
            None => u.team == Team::Enemy,
        };
    }

    separate(bodies, &alive.iter().map(|u| u.id).collect::<Vec<_>>());

    for id in alive.iter().map(|u| u.id) {
        if let Some(b) = bodies.get_mut(&id) {
            let r = Vec2::splat(b.radius.min(size.min_element() / 2.0));
            b.pos = b.pos.clamp(r, size - r);
        }
    }
}

/// Pushes overlapping bodies apart, each by half the overlap. A couple of
/// passes settle small crowds without jitter.
fn separate(bodies: &mut HashMap<UnitId, Body>, ids: &[UnitId]) {
    for _ in 0..2 {
        for (i, &a) in ids.iter().enumerate() {
            for &b in &ids[i + 1..] {
                let (Some(pa), Some(pb)) = (bodies.get(&a), bodies.get(&b)) else { continue };
                let min = pa.radius + pb.radius;
                let mut d = pb.pos - pa.pos;
                let len = d.length();
                if len >= min {
                    continue;
                }
                if len < 0.01 {
                    // Exactly on top of each other: split them along a fixed
                    // direction picked by id, so it is the same every time.
                    d = if a < b { Vec2::X } else { Vec2::NEG_X };
                }
                let push = d.normalize_or_zero() * (min - len) / 2.0;
                if let Some(x) = bodies.get_mut(&a) {
                    x.pos -= push;
                }
                if let Some(x) = bodies.get_mut(&b) {
                    x.pos += push;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use battler_sim::{Character, Level, Program, UnitSpec};

    const CELL: f32 = 72.0;
    fn centre(p: Pos) -> Vec2 {
        Vec2::new(p.y as f32 * CELL + CELL / 2.0, p.x as f32 * CELL + CELL / 2.0)
    }
    fn battle(units: &[(Team, i32, i32)]) -> Battle {
        let mut b = Battle::new(8, 7, 50, 1);
        for &(team, x, y) in units {
            let spec = UnitSpec::new(Character::Nettle, Level::One, Program::new(Vec::new()));
            b.spawn(team, Pos::new(x, y), spec).expect("free cell");
        }
        b
    }
    const SIZE: Vec2 = Vec2::new(7.0 * CELL, 8.0 * CELL);

    #[test]
    fn bodies_walk_to_their_cell_and_stop() {
        let b = battle(&[(Team::Player, 0, 0)]);
        let mut bodies = spawn(&b, centre, |_| 22.0);
        let id = b.units()[0].id;
        bodies.get_mut(&id).unwrap().pos += Vec2::new(60.0, 0.0);
        for _ in 0..120 {
            step(&mut bodies, &b, centre, SIZE, 1.0 / 60.0);
        }
        let body = &bodies[&id];
        let home = centre(Pos::new(0, 0)) + scatter(id);
        assert!(body.pos.distance(home) < 1.0, "{:?} vs {home:?}", body.pos);
        assert!(body.vel.length() < 5.0);
    }

    #[test]
    fn overlapping_bodies_are_pushed_apart() {
        let b = battle(&[(Team::Player, 0, 0), (Team::Player, 0, 1)]);
        let mut bodies = spawn(&b, centre, |_| 22.0);
        let (a, c) = (b.units()[0].id, b.units()[1].id);
        // Put both on the same spot.
        let p = bodies[&a].pos;
        bodies.get_mut(&c).unwrap().pos = p;
        separate(&mut bodies, &[a, c]);
        separate(&mut bodies, &[a, c]);
        assert!(bodies[&a].pos.distance(bodies[&c].pos) >= 44.0 - 0.01);
    }

    #[test]
    fn scatter_is_small_and_stable() {
        for i in 0..50 {
            let s = scatter(UnitId(i));
            assert!(s.x.abs() <= SCATTER && s.y.abs() <= SCATTER);
            assert_eq!(s, scatter(UnitId(i)));
        }
    }

    #[test]
    fn a_lunge_moves_towards_the_target_then_back() {
        let b = battle(&[(Team::Player, 3, 3), (Team::Enemy, 4, 3)]);
        let mut bodies = spawn(&b, centre, |_| 22.0);
        let (me, foe) = (b.units()[0].id, b.units()[1].id);
        let start = bodies[&me].pos;
        lunge(&mut bodies, me, foe);
        let mut closest = f32::MAX;
        for _ in 0..60 {
            step(&mut bodies, &b, centre, SIZE, 1.0 / 60.0);
            closest = closest.min(bodies[&me].pos.distance(bodies[&foe].pos));
        }
        assert!(closest < start.distance(bodies[&foe].pos) - 8.0, "lunged forward");
        assert!(bodies[&me].pos.distance(start) < 3.0, "came back");
    }
}

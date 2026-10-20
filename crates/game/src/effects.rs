//! Spell and hit effects on the battlefield: expanding rings, bursts, bubbles
//! and particles, built from plain UI nodes that grow, drift and fade.
//!
//! Effects are presentation only. Each is spawned as a child of the battle
//! field (so it is cleaned up with it) with an [`Effect`] component that
//! [`animate`] advances every frame in playback time, so they speed up with the
//! 2x and 4x buttons. Positions are pixels on the isometric field; an effect
//! can follow a unit (relative to its feet) as it moves. Effects on the ground
//! are `flat`: circles squashed to half height so they lie on the diamond.

use std::collections::HashMap;

use battler_sim::UnitId;
use bevy::prelude::*;

/// Effects draw above units (whose stacking order tops out around 150) but
/// below projectiles and floating numbers.
pub const EFFECT_Z: i32 = 300;

/// What an effect looks like over its lifetime.
#[derive(Debug, Clone, Copy)]
pub enum Kind {
    /// A circle outline growing from `from` to `to` pixels in radius.
    Ring { from: f32, to: f32, width: f32 },
    /// A filled circle that grows to `to` pixels in radius while fading.
    Burst { to: f32 },
    /// A translucent filled circle of fixed radius (shields, bubbles).
    Bubble { radius: f32 },
    /// A small square drifting with velocity `vel` (px/s), pulled by `gravity`.
    Particle { vel: Vec2, gravity: f32, size: f32 },
}

#[derive(Component, Debug, Clone)]
pub struct Effect {
    kind: Kind,
    color: Color,
    /// Where it is, or its offset from the followed unit.
    at: Vec2,
    follow: Option<UnitId>,
    /// Lies on the ground: drawn half as tall as wide.
    flat: bool,
    /// Seconds before it appears (e.g. an explosion waits for the projectile).
    delay: f32,
    age: f32,
    life: f32,
}

/// Builder for effects, so call sites read as a sentence:
/// `effects::ring(at, 10.0, 90.0).color(ICE).life(0.45).spawn(..)`.
#[derive(Debug, Clone)]
pub struct EffectBuilder(Effect);

fn new(kind: Kind, at: Vec2) -> EffectBuilder {
    EffectBuilder(Effect {
        kind,
        color: Color::WHITE,
        at,
        follow: None,
        flat: false,
        delay: 0.0,
        age: 0.0,
        life: 0.4,
    })
}

pub fn ring(at: Vec2, from: f32, to: f32) -> EffectBuilder {
    new(Kind::Ring { from, to, width: 4.0 }, at)
}
pub fn burst(at: Vec2, to: f32) -> EffectBuilder {
    new(Kind::Burst { to }, at)
}
pub fn bubble(at: Vec2, radius: f32) -> EffectBuilder {
    new(Kind::Bubble { radius }, at)
}
pub fn particle(at: Vec2, vel: Vec2, gravity: f32, size: f32) -> EffectBuilder {
    new(Kind::Particle { vel, gravity, size }, at)
}

impl EffectBuilder {
    pub fn color(mut self, c: Color) -> Self {
        self.0.color = c;
        self
    }
    pub fn life(mut self, secs: f32) -> Self {
        self.0.life = secs;
        self
    }
    pub fn delay(mut self, secs: f32) -> Self {
        self.0.delay = secs;
        self
    }
    /// Lies on the ground (rings and novas around a unit's feet).
    pub fn flat(mut self) -> Self {
        self.0.flat = true;
        self
    }
    /// Moves with a unit: `at` becomes an offset from that unit's body.
    pub fn follow(mut self, unit: UnitId) -> Self {
        self.0.follow = Some(unit);
        self
    }

    pub fn spawn(self, commands: &mut Commands, board: Entity) {
        let effect = self.0;
        commands.entity(board).with_children(|b| {
            b.spawn((
                Node { position_type: PositionType::Absolute, display: Display::None, ..default() },
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                ZIndex(EFFECT_Z),
                Pickable::IGNORE,
                effect,
            ));
        });
    }
}

/// A spray of `n` particles bursting out from `at`, evenly spread around a
/// circle with a little variety in speed.
pub fn spray(commands: &mut Commands, board: Entity, at: Vec2, n: usize, speed: f32, color: Color) {
    for i in 0..n {
        let angle = std::f32::consts::TAU * (i as f32 + 0.3 * (i % 3) as f32) / n as f32;
        let s = speed * (0.7 + 0.15 * (i % 3) as f32);
        let vel = Vec2::from_angle(angle) * s;
        let size = 4.0 + (i % 2) as f32 * 2.0;
        particle(at, vel, 0.0, size).color(color).life(0.45).spawn(commands, board);
    }
}

/// Plus-shaped sparkles rising from a unit (heals).
pub fn sparkles(commands: &mut Commands, board: Entity, unit: UnitId, color: Color) {
    for i in 0..6 {
        let x = (i as f32 - 2.5) * 7.0;
        let vel = Vec2::new(x * 0.6, -50.0 - (i % 3) as f32 * 14.0);
        particle(Vec2::new(x, -40.0), vel, 0.0, 5.0)
            .color(color)
            .life(0.6)
            .delay(i as f32 * 0.04)
            .follow(unit)
            .spawn(commands, board);
    }
}

/// Grows, moves and fades every effect; removes it when its life is over.
pub fn animate(
    commands: &mut Commands,
    dt: f32,
    feet: &HashMap<UnitId, Vec2>,
    q: &mut Query<(Entity, &mut Effect, &mut Node, &mut BackgroundColor, &mut BorderColor)>,
) {
    for (e, mut fx, mut node, mut bg, mut border) in q.iter_mut() {
        if fx.delay > 0.0 {
            fx.delay -= dt;
            continue;
        }
        fx.age += dt;
        let t = (fx.age / fx.life).min(1.0);
        if t >= 1.0 {
            commands.entity(e).despawn();
            continue;
        }
        let origin = match fx.follow {
            Some(id) => feet.get(&id).map_or(fx.at, |f| *f + fx.at),
            None => fx.at,
        };
        // Ease out: fast at first, settling at the end.
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let fade = 1.0 - t * t;
        node.display = Display::Flex;
        match fx.kind {
            Kind::Ring { from, to, width } => {
                let r = from + (to - from) * ease;
                place_circle(&mut node, origin, r, fx.flat);
                node.border = UiRect::all(px(width));
                bg.0 = Color::NONE;
                border.set_all(fx.color.with_alpha(fade));
            }
            Kind::Burst { to } => {
                place_circle(&mut node, origin, to * ease, fx.flat);
                bg.0 = fx.color.with_alpha(0.75 * fade);
            }
            Kind::Bubble { radius } => {
                // Pops in, holds, then fades.
                let r = radius * (0.6 + 0.4 * (t * 4.0).min(1.0));
                place_circle(&mut node, origin, r, fx.flat);
                node.border = UiRect::all(px(2));
                bg.0 = fx.color.with_alpha(0.25 * fade);
                border.set_all(fx.color.with_alpha(0.9 * fade));
            }
            Kind::Particle { vel, gravity, size } => {
                let a = fx.age;
                let p = origin + vel * a + Vec2::new(0.0, 0.5 * gravity * a * a);
                node.left = px(p.x - size / 2.0);
                node.top = px(p.y - size / 2.0);
                node.width = px(size);
                node.height = px(size);
                bg.0 = fx.color.with_alpha(fade);
            }
        }
    }
}

/// Sizes a node as a circle of radius `r` around `centre`, or for a `flat`
/// effect an ellipse half as tall (a circle on the ground, seen at an angle).
fn place_circle(node: &mut Node, centre: Vec2, r: f32, flat: bool) {
    let r = r.max(1.0);
    let ry = if flat { r / 2.0 } else { r };
    node.left = px(centre.x - r);
    node.top = px(centre.y - ry);
    node.width = px(r * 2.0);
    node.height = px(ry * 2.0);
    node.border_radius = BorderRadius::all(percent(50));
}

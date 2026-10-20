//! The battle meters: a pop-out panel with two tabs, Damage and Healing, that
//! ranks your units by how much they have dealt or healed in the current
//! battle (live, as it plays) or in the last one (while planning).
//!
//! Totals are gathered in `battle.rs` from the sim's events into
//! [`Playback`]; this module only draws them. The panel is built with the
//! round screen, and during a battle its rows are rebuilt whenever the totals
//! change (the same pattern as the card trace).

use battler_sim::{Team, UnitId};
use bevy::prelude::*;

use crate::battle::Playback;
use crate::kit;
use crate::sprites::{Sprites, sprite_node};
use crate::state::{MeterTab, RunPhase, Settings};
use crate::ui::{self, SECONDARY, TOGGLED, UiAction, color, column, heading, row, text};

pub struct MetersPlugin;

impl Plugin for MetersPlugin {
    fn build(&self, app: &mut App) {
        // `PostUpdate`, not `Update`: toggling the panel bumps `Rev`, and the
        // round screen's rebuild then despawns the old panel. Commands are
        // deferred, so a refresh in `Update` could queue rows for a panel that
        // is despawned before they apply, and a command on a missing entity
        // panics. By `PostUpdate` the rebuild has been applied, so the query
        // only finds panels that still exist.
        app.add_systems(
            PostUpdate,
            refresh_meters
                .run_if(in_state(RunPhase::Battle).and_then(resource_exists::<Playback>))
                .before(bevy::ui::UiSystems::Layout),
        );
    }
}

/// The panel's list of rows, rebuilt when the totals change.
#[derive(Component)]
struct MeterContent;

pub const PANEL_W: f32 = 200.0;

/// The meters panel: a tab per meter, then one row per unit.
pub fn spawn_meters(
    parent: &mut ChildSpawnerCommands,
    pb: Option<&Playback>,
    settings: &Settings,
    sprites: &Sprites,
) {
    parent
        .spawn(ui::panel(Node {
            width: px(PANEL_W),
            min_height: px(0),
            flex_shrink: 1.0,
            overflow: Overflow::clip(),
            ..column(8.0)
        }))
        .with_children(|p| {
            p.spawn(row(4.0)).with_children(|tabs| {
                for (tab, label) in [(MeterTab::Damage, "Damage"), (MeterTab::Healing, "Healing")] {
                    let style = if settings.meter_tab == tab { TOGGLED } else { SECONDARY };
                    tabs.spawn(ui::button(
                        UiAction::MeterTab(tab),
                        style,
                        Node { flex_grow: 1.0, height: px(30), ..default() },
                    ))
                    .with_child(heading(label, 12.0, color::TEXT));
                }
            });
            p.spawn((column(6.0), MeterContent))
                .with_children(|c| meter_rows(c, pb, settings.meter_tab, sprites));
        });
}

/// Your units ranked by the chosen total, with a bar relative to the best.
fn meter_rows(
    c: &mut ChildSpawnerCommands,
    pb: Option<&Playback>,
    tab: MeterTab,
    sprites: &Sprites,
) {
    let Some(pb) = pb else {
        c.spawn(text("No battle yet.", 12.0, color::MUTED));
        return;
    };
    let totals = match tab {
        MeterTab::Damage => &pb.damage,
        MeterTab::Healing => &pb.healing,
    };
    // Every unit on your side, dead or alive, with its total (0 if none).
    let mut rows: Vec<(UnitId, u32)> = pb
        .battle
        .units()
        .iter()
        .filter(|u| u.team == Team::Player)
        .map(|u| (u.id, totals.get(&u.id).copied().unwrap_or(0)))
        .collect();
    // Highest first; ties keep the id order, so rows don't jitter.
    rows.sort_by_key(|&(id, v)| (std::cmp::Reverse(v), id));
    let best = rows.first().map_or(0, |r| r.1).max(1);
    let total: u32 = rows.iter().map(|r| r.1).sum();
    if total == 0 {
        let none = match tab {
            MeterTab::Damage => "No damage dealt yet.",
            MeterTab::Healing => "No healing done yet.",
        };
        c.spawn(text(none, 12.0, color::MUTED));
    }
    let fill = match tab {
        MeterTab::Damage => kit::img::FILL_ENEMY,
        MeterTab::Healing => kit::img::FILL_HP,
    };
    for (id, value) in rows {
        let Some(unit) = pb.battle.unit(id) else { continue };
        let name = match unit.kind.character() {
            Some(ch) => ch.short_name().to_string(),
            None => "Monster".to_string(),
        };
        let ink = if unit.is_alive() { color::TEXT } else { color::FAINT };
        c.spawn(Node { align_items: AlignItems::Center, ..row(6.0) }).with_children(|r| {
            r.spawn(sprite_node(sprites.unit(unit.kind), 1.0, false));
            r.spawn(Node { flex_grow: 1.0, min_width: px(0), ..column(2.0) }).with_children(
                |col| {
                    col.spawn(Node { justify_content: JustifyContent::SpaceBetween, ..row(4.0) })
                        .with_children(|line| {
                            line.spawn(heading(name, 12.0, ink));
                            line.spawn(heading(value.to_string(), 12.0, ink));
                        });
                    let pct = 100.0 * value as f32 / best as f32;
                    kit::spawn_bar(col, fill.clone(), pct, percent(100), 10.0, ());
                },
            );
        });
    }
    if total > 0 {
        c.spawn(Node { justify_content: JustifyContent::FlexEnd, ..row(0.0) }).with_child(text(
            format!("Total {total}"),
            12.0,
            color::MUTED,
        ));
    }
}

/// Rebuilds the rows when a tick changed the totals (or the tab changed).
fn refresh_meters(
    mut commands: Commands,
    mut pb: ResMut<Playback>,
    settings: Res<Settings>,
    sprites: Res<Sprites>,
    contents: Query<Entity, With<MeterContent>>,
) {
    if !pb.meters_dirty && !settings.is_changed() {
        return;
    }
    pb.meters_dirty = false;
    for e in &contents {
        commands.entity(e).despawn_related::<Children>();
        let (pb, settings, sprites) = (&*pb, &*settings, &*sprites);
        commands.entity(e).with_children(|c| meter_rows(c, Some(pb), settings.meter_tab, sprites));
    }
}

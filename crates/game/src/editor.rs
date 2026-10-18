//! The card editor: a side drawer on the planning screen. Blocks are dragged
//! (or clicked) from the palette into rule cards; clicking a placed block cycles
//! its setting. Every change is written straight into the run, so a card only
//! "counts" once it has an action.

use battler_sim::run::{Location, OwnedUnit, Run};
use battler_sim::{Ability, Action, Class, Condition};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::cards::{self, action_label, cond_label};
use crate::planning::class_letter;
use crate::state::{DraftCard, Planning, RunPhase};
use crate::ui::{
    self, Block, Clickable, DragSource, DropTarget, Slot, UiAction, color, column, row,
    spawn_button, text,
};

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HoveredAbility>()
            .add_systems(Update, track_hover.run_if(in_state(RunPhase::Planning)));
    }
}

/// The signature ability whose palette block the pointer is over, if any.
/// Used to highlight that ability's reach on the board.
#[derive(Resource, Default)]
pub struct HoveredAbility(pub Option<Ability>);

#[derive(Component)]
struct PaletteAbility(Ability);

fn track_hover(chips: Query<(&PaletteAbility, &Hovered)>, mut hovered: ResMut<HoveredAbility>) {
    let now = chips.iter().find(|(_, h)| h.get()).map(|(a, _)| a.0);
    if hovered.0 != now {
        hovered.0 = now;
    }
}

// ----- Editing logic (called from `actions.rs`) ------------------------

/// The selected unit and where it is, if the selection is still valid.
pub fn selected_unit<'a>(run: &'a Run, planning: &Planning) -> Option<(Location, &'a OwnedUnit)> {
    let loc = run.locate(planning.selected?)?;
    Some((loc, run.unit_at(loc)?))
}

/// Loads the selected unit's program into the draft.
pub fn load_draft(run: &Run, planning: &mut Planning) {
    planning.draft = selected_unit(run, planning)
        .map(|(_, u)| cards::program_to_draft(&u.program))
        .unwrap_or_default();
}

/// Tries a new draft: if the program it makes is legal, keep it and save it
/// to the unit; otherwise leave everything unchanged and explain why.
fn try_draft(run: &mut Run, planning: &mut Planning, draft: Vec<DraftCard>) {
    let Some((loc, _)) = selected_unit(run, planning) else { return };
    match run.set_program(loc, cards::draft_to_program(&draft)) {
        Ok(()) => planning.draft = draft,
        Err(e) => planning.toast(e.to_string()),
    }
}

fn block_fits(block: Block, slot: Slot) -> bool {
    matches!((block, slot), (Block::Cond(_), Slot::Cond) | (Block::Act(_), Slot::Act))
}

fn put(card: &mut DraftCard, block: Block) {
    match block {
        Block::Cond(c) => card.cond = Some(c),
        Block::Act(a) => card.act = Some(a),
    }
}

/// Drops a block into a specific card (an index one past the end starts a new card).
pub fn drop_block(run: &mut Run, planning: &mut Planning, card: usize, slot: Slot, block: Block) {
    if !block_fits(block, slot) {
        planning.toast(match slot {
            Slot::Cond => "That slot takes a condition",
            Slot::Act => "That slot takes an action",
        });
        return;
    }
    let Some((_, unit)) = selected_unit(run, planning) else { return };
    let max = unit.level.max_nodes();
    let mut draft = planning.draft.clone();
    if card >= draft.len() {
        if draft.len() >= max {
            planning.toast("No free card: level this unit up for more");
            return;
        }
        draft.push(DraftCard::default());
    }
    let index = card.min(draft.len() - 1);
    put(&mut draft[index], block);
    try_draft(run, planning, draft);
}

/// Clicking a palette block: fill the first card missing that kind of block,
/// or start a new card.
pub fn click_block(run: &mut Run, planning: &mut Planning, block: Block) {
    let first_gap = planning.draft.iter().position(|c| match block {
        Block::Cond(_) => c.cond.is_none(),
        Block::Act(_) => c.act.is_none(),
    });
    let card = first_gap.unwrap_or(planning.draft.len());
    let slot = match block {
        Block::Cond(_) => Slot::Cond,
        Block::Act(_) => Slot::Act,
    };
    drop_block(run, planning, card, slot, block);
}

pub fn cycle_slot(run: &mut Run, planning: &mut Planning, card: usize, slot: Slot) {
    let Some((_, unit)) = selected_unit(run, planning) else { return };
    let class = unit.class;
    let mut draft = planning.draft.clone();
    let Some(c) = draft.get_mut(card) else { return };
    match slot {
        Slot::Cond => c.cond = c.cond.map(|x| cards::cycle_condition(x, class)),
        Slot::Act => c.act = c.act.map(|x| cards::cycle_action(x, class)),
    }
    try_draft(run, planning, draft);
}

pub fn clear_card(run: &mut Run, planning: &mut Planning, card: usize) {
    let mut draft = planning.draft.clone();
    if card < draft.len() {
        draft.remove(card);
        try_draft(run, planning, draft);
    }
}

pub fn card_up(run: &mut Run, planning: &mut Planning, card: usize) {
    let mut draft = planning.draft.clone();
    if card > 0 && card < draft.len() {
        draft.swap(card, card - 1);
        try_draft(run, planning, draft);
    }
}

// ----- Drawer UI -------------------------------------------------------

pub fn spawn_drawer(parent: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    let Some((_, unit)) = selected_unit(run, planning) else { return };
    let class = unit.class;
    let max = unit.level.max_nodes();
    let used = planning.draft.iter().filter(|c| c.act.is_some()).count();

    parent
        .spawn((
            Node {
                width: px(600),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                border: UiRect::left(px(1)),
                ..default()
            },
            BackgroundColor(color::PANEL),
            BorderColor::all(color::LINE_STRONG),
        ))
        .with_children(|drawer| {
            drawer
                .spawn((
                    Node {
                        height: px(64),
                        flex_shrink: 0.0,
                        padding: UiRect::horizontal(px(18)),
                        border: UiRect::bottom(px(1)),
                        ..row(12.0)
                    },
                    BorderColor::all(color::LINE),
                ))
                .with_children(|h| {
                    h.spawn((
                        Node {
                            width: px(40),
                            height: px(40),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border: UiRect::all(px(2)),
                            border_radius: BorderRadius::all(px(8)),
                            ..default()
                        },
                        BackgroundColor(color::BLUE_FILL),
                        BorderColor::all(color::GOLD),
                    ))
                    .with_child(text(
                        class_letter(class),
                        17.0,
                        color::BLUE_TEXT,
                    ));
                    h.spawn(Node { flex_grow: 1.0, ..column(2.0) }).with_children(|c| {
                        c.spawn(text(format!("{} - cards", class.name()), 16.0, color::TEXT));
                        c.spawn(text(
                            format!("Level {} - {used} of {max} cards used", unit.level.number()),
                            12.0,
                            color::MUTED,
                        ));
                    });
                    spawn_button(h, "Done", UiAction::CloseEditor, ui::PRIMARY, px(90), px(44));
                });

            drawer
                .spawn(Node {
                    flex_grow: 1.0,
                    padding: UiRect::axes(px(18), px(16)),
                    min_height: px(0),
                    align_items: AlignItems::Stretch,
                    ..row(16.0)
                })
                .with_children(|body| {
                    palette(body, class);
                    program(body, planning, max);
                });
        });
}

fn chip_colors(block: Block) -> (Color, Color, Color) {
    match block {
        Block::Cond(_) => (color::COND_FILL, color::COND_LINE, color::COND_TEXT),
        Block::Act(a) if a.ability().is_some() => {
            (color::SIG_FILL, color::SIG_LINE, color::SIG_TEXT)
        }
        Block::Act(_) => (color::ACT_FILL, color::ACT_LINE, color::ACT_TEXT),
    }
}

fn chip(parent: &mut ChildSpawnerCommands, block: Block) {
    let (fill, line, ink) = chip_colors(block);
    let mut e = parent.spawn((
        Button,
        Hovered::default(),
        Clickable(UiAction::PaletteBlock(block)),
        DragSource::Block(block),
        Pickable { should_block_lower: false, is_hoverable: true },
        GlobalZIndex::default(),
        Node {
            padding: UiRect::axes(px(8), px(4)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(6)),
            ..default()
        },
        BackgroundColor(fill),
        BorderColor::all(line),
        ui::HoverColors { normal: fill, hover: line },
    ));
    if let Block::Act(a) = block
        && let Some(ability) = a.ability()
    {
        e.insert(PaletteAbility(ability));
    }
    e.with_child(text(cards::block_label(block), 12.0, ink));
}

fn chip_group(parent: &mut ChildSpawnerCommands, title: &str, blocks: Vec<Block>) {
    parent.spawn(column(6.0)).with_children(|g| {
        g.spawn(text(title, 12.0, color::MUTED));
        g.spawn(Node { flex_wrap: FlexWrap::Wrap, row_gap: px(5), column_gap: px(5), ..default() })
            .with_children(|wrap| {
                for b in blocks {
                    chip(wrap, b);
                }
            });
    });
}

fn palette(parent: &mut ChildSpawnerCommands, class: Class) {
    parent.spawn(Node { width: px(214), flex_shrink: 0.0, ..column(12.0) }).with_children(|p| {
        chip_group(
            p,
            "Conditions",
            cards::shared_conditions(class).into_iter().map(Block::Cond).collect(),
        );
        chip_group(p, "Actions", cards::shared_actions().into_iter().map(Block::Act).collect());
        chip_group(
            p,
            class.name(),
            cards::signature_actions(class).into_iter().map(Block::Act).collect(),
        );
        p.spawn((
            Node {
                margin: UiRect::top(Val::Auto),
                padding: UiRect::all(px(10)),
                border_radius: BorderRadius::all(px(8)),
                ..column(6.0)
            },
            BackgroundColor(color::PANEL_2),
        ))
        .with_children(|help| {
            for ability in class.abilities() {
                help.spawn(text(cards::ability_label(ability), 13.0, color::SIG_TEXT));
                help.spawn(text(cards::ability_help(ability), 12.0, color::MUTED));
            }
        });
    });
}

fn slot_box(parent: &mut ChildSpawnerCommands, card: usize, slot: Slot, label: Option<String>) {
    let (fill, line, ink) = match (slot, &label) {
        (_, None) => (color::BG, color::LINE_STRONG, color::FAINT),
        (Slot::Cond, Some(_)) => (color::COND_FILL, color::COND_LINE, color::COND_TEXT),
        (Slot::Act, Some(_)) => (color::SIG_FILL, color::SIG_LINE, color::SIG_TEXT),
    };
    let placeholder = match slot {
        Slot::Cond => "always (drop a condition)",
        Slot::Act => "drop an action",
    };
    parent
        .spawn(ui::button(
            UiAction::CycleSlot(card, slot),
            ui::ButtonStyle { fill, hover: line, border: line },
            Node {
                flex_grow: 1.0,
                justify_content: JustifyContent::FlexStart,
                padding: UiRect::axes(px(10), px(7)),
                border_radius: BorderRadius::all(px(7)),
                ..default()
            },
        ))
        .insert(DropTarget::Card(card, slot))
        .with_child(text(label.unwrap_or_else(|| placeholder.to_string()), 13.0, ink));
}

fn card_row(parent: &mut ChildSpawnerCommands, index: usize, card: Option<DraftCard>) {
    let filled = card.is_some();
    let (fill, line) =
        if filled { (color::PANEL_2, color::LINE_STRONG) } else { (color::BG, color::LINE) };
    parent
        .spawn((
            Node {
                padding: UiRect::axes(px(10), px(8)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                align_items: AlignItems::Stretch,
                ..row(10.0)
            },
            BackgroundColor(fill),
            BorderColor::all(line),
        ))
        .with_children(|c| {
            c.spawn(Node { width: px(26), align_items: AlignItems::Center, ..column(4.0) })
                .with_children(|n| {
                    n.spawn(text((index + 1).to_string(), 13.0, color::FAINT));
                    if filled {
                        if index > 0 {
                            small_button(n, "up", UiAction::CardUp(index));
                        }
                        small_button(n, "x", UiAction::ClearCard(index));
                    }
                });
            c.spawn(Node { flex_grow: 1.0, ..column(6.0) }).with_children(|slots| {
                let cond: Option<Condition> = card.and_then(|c| c.cond);
                let act: Option<Action> = card.and_then(|c| c.act);
                slots.spawn(row(8.0)).with_children(|r| {
                    r.spawn(Node { width: px(30), ..default() }).with_child(text(
                        "if",
                        12.0,
                        color::FAINT,
                    ));
                    slot_box(r, index, Slot::Cond, cond.map(cond_label));
                });
                slots.spawn(row(8.0)).with_children(|r| {
                    r.spawn(Node { width: px(30), ..default() }).with_child(text(
                        "then",
                        12.0,
                        color::FAINT,
                    ));
                    slot_box(r, index, Slot::Act, act.map(action_label));
                });
            });
        });
}

fn small_button(parent: &mut ChildSpawnerCommands, label: &str, action: UiAction) {
    parent
        .spawn(ui::button(
            action,
            ui::SECONDARY,
            Node {
                width: px(26),
                height: px(22),
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
        ))
        .with_child(text(label, 11.0, color::MUTED));
}

fn program(parent: &mut ChildSpawnerCommands, planning: &Planning, max: usize) {
    parent.spawn(Node { flex_grow: 1.0, min_width: px(0), ..column(10.0) }).with_children(|p| {
        p.spawn(text("Top to bottom - the first card that can run wins", 12.0, color::FAINT));
        for (i, card) in planning.draft.iter().enumerate() {
            card_row(p, i, Some(*card));
        }
        if planning.draft.len() < max {
            card_row(p, planning.draft.len(), None);
        }
        p.spawn((
            Node {
                padding: UiRect::axes(px(12), px(10)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(10)),
                ..row(10.0)
            },
            BackgroundColor(color::BG_DEEP),
            BorderColor::all(color::LINE),
        ))
        .with_child(text(
            "Locked: otherwise attack nearest, advancing if needed",
            13.0,
            color::FAINT,
        ));
        if planning.draft.len() >= max {
            p.spawn(text(
                "All card slots used - merge copies to level up for more",
                12.0,
                color::FAINT,
            ));
        }
        p.spawn((Node { margin: UiRect::top(Val::Auto), ..default() }, Pickable::IGNORE))
            .with_child(text(
                "Drag or click blocks into cards - click a placed block to change it",
                12.0,
                color::FAINT,
            ));
    });
}

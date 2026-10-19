//! The program column on the planning screen: the selected unit's cards, always
//! editable, with the block palette underneath. Blocks are dragged (or clicked)
//! into cards; clicking a placed block cycles its setting. Every change is
//! written straight into the run, so a card only "counts" once it has an action.

use battler_sim::run::{Location, OwnedUnit, Run};
use battler_sim::{Ability, Action, Character, Condition, Unlocks};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::cards::{self, action_label, cond_label};
use crate::state::{DraftCard, Planning, RunPhase};
use crate::ui::{
    self, Block, Clickable, DragSource, DropTarget, Slot, UiAction, color, column, row, text,
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

fn slot_of(block: Block) -> Slot {
    match block {
        Block::Cond(_) => Slot::Cond,
        Block::Act(_) => Slot::Act,
    }
}

fn put(card: &mut DraftCard, block: Block) {
    match block {
        Block::Cond(c) => card.cond = Some(c),
        Block::Act(a) => card.act = Some(a),
    }
}

/// Drops a block into a specific card (an index one past the end starts a new card).
pub fn drop_block(run: &mut Run, planning: &mut Planning, card: usize, slot: Slot, block: Block) {
    if slot_of(block) != slot {
        planning.toast(match slot {
            Slot::Cond => "That slot takes a condition",
            Slot::Act => "That slot takes an action",
        });
        return;
    }
    let Some((_, unit)) = selected_unit(run, planning) else { return };
    let max = run.synergies().card_slots(unit.character, unit.level);
    let mut draft = planning.draft.clone();
    if card >= draft.len() {
        if draft.len() >= max {
            planning.toast("No free card: merge copies to level this unit up");
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
    drop_block(run, planning, card, slot_of(block), block);
}

pub fn cycle_slot(run: &mut Run, planning: &mut Planning, card: usize, slot: Slot) {
    let Some((_, unit)) = selected_unit(run, planning) else { return };
    let character = unit.character;
    let unlocks = run.unlocks();
    let mut draft = planning.draft.clone();
    let Some(c) = draft.get_mut(card) else { return };
    match slot {
        Slot::Cond => c.cond = c.cond.map(cards::cycle_condition),
        Slot::Act => c.act = c.act.map(|a| cards::cycle_action(a, &unlocks, character)),
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

// ----- Program column UI -----------------------------------------------

pub fn spawn_program_panel(parent: &mut ChildSpawnerCommands, run: &Run, planning: &Planning) {
    parent
        .spawn(ui::panel(Node {
            flex_grow: 1.0,
            min_width: px(0),
            padding: UiRect::all(px(12)),
            ..column(9.0)
        }))
        .with_children(|p| {
            let Some((loc, unit)) = selected_unit(run, planning) else {
                p.spawn(text("No unit selected", 16.0, color::TEXT));
                for line in [
                    "Buy units in the shop below. They go straight",
                    "onto the board when there's room.",
                    "Click a unit to program it here.",
                    "Drag units to move them; drop one on the shop",
                    "to sell it. Hover anything for details.",
                    "Keys: D reroll, F buy XP, E sell hovered unit.",
                ] {
                    p.spawn(text(line, 13.0, color::MUTED));
                }
                return;
            };
            let unlocks = run.unlocks();
            header(p, unit, loc);
            cards_list(p, run, planning, unit);
            p.spawn((Node { height: px(1), ..default() }, BackgroundColor(color::LINE)));
            palette(p, unit.character, &unlocks);
            p.spawn((Node { margin: UiRect::top(Val::Auto), ..default() }, Pickable::IGNORE))
                .with_child(text(
                    "Drag or click blocks into cards - click a placed block to change it",
                    11.0,
                    color::FAINT,
                ));
        });
}

fn header(p: &mut ChildSpawnerCommands, unit: &OwnedUnit, loc: Location) {
    let c = unit.character;
    let stats = unit.level.scale(c.base_stats());
    p.spawn(row(10.0)).with_children(|r| {
        r.spawn((
            Node {
                width: px(36),
                height: px(36),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(color::BLUE_FILL),
            BorderColor::all(cards::rarity_color(c.rarity())),
            Hovered::default(),
            cards::unit_tooltip(c, Some(unit.level), None),
        ))
        .with_child(text(cards::initials(c), 14.0, color::BLUE_TEXT));
        r.spawn(Node { flex_grow: 1.0, min_width: px(0), ..column(1.0) }).with_children(|col| {
            col.spawn(row(6.0)).with_children(|n| {
                n.spawn(text(c.name(), 15.0, color::TEXT));
                n.spawn(text(
                    format!("{} {}", c.rarity().name(), c.class().name()),
                    11.0,
                    cards::rarity_color(c.rarity()),
                ));
            });
            let place = if matches!(loc, Location::Bench(_)) { " - bench" } else { "" };
            col.spawn(text(
                format!(
                    "L{} - {} HP - {} atk - r{}{place}",
                    unit.level.number(),
                    stats.max_hp,
                    stats.attack,
                    stats.range
                ),
                11.0,
                color::MUTED,
            ));
        });
        r.spawn(ui::button(
            UiAction::Sell,
            ui::SECONDARY,
            Node {
                height: px(32),
                padding: UiRect::horizontal(px(10)),
                border_radius: BorderRadius::all(px(7)),
                ..default()
            },
        ))
        .with_child(text(format!("Sell {} g", unit.sell_value()), 12.0, color::TEXT));
    });
}

fn cards_list(p: &mut ChildSpawnerCommands, run: &Run, planning: &Planning, unit: &OwnedUnit) {
    let max = run.synergies().card_slots(unit.character, unit.level);
    let unlocks = run.unlocks();
    p.spawn(column(5.0)).with_children(|list| {
        for (i, card) in planning.draft.iter().enumerate() {
            card_row(list, i, Some(*card), &unlocks, unit.character);
        }
        if planning.draft.len() < max {
            card_row(list, planning.draft.len(), None, &unlocks, unit.character);
        }
        list.spawn((
            Node {
                padding: UiRect::axes(px(8), px(5)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(7)),
                ..row(6.0)
            },
            BackgroundColor(color::BG_DEEP),
            BorderColor::all(color::LINE),
        ))
        .with_child(text("locked: otherwise attack nearest", 11.0, color::FAINT));
        if planning.draft.len() >= max {
            list.spawn(text(
                format!("{max} of {max} cards - merge copies for more"),
                11.0,
                color::FAINT,
            ));
        }
    });
}

fn card_row(
    list: &mut ChildSpawnerCommands,
    index: usize,
    card: Option<DraftCard>,
    unlocks: &Unlocks,
    character: Character,
) {
    let filled = card.is_some();
    let (fill, line) =
        if filled { (color::PANEL_2, color::LINE_STRONG) } else { (color::BG, color::LINE) };
    list.spawn((
        Node {
            padding: UiRect::axes(px(7), px(5)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            ..row(6.0)
        },
        BackgroundColor(fill),
        BorderColor::all(line),
    ))
    .with_children(|r| {
        r.spawn(Node { width: px(12), ..default() }).with_child(text(
            (index + 1).to_string(),
            11.0,
            color::FAINT,
        ));
        let cond: Option<Condition> = card.and_then(|c| c.cond);
        let act: Option<Action> = card.and_then(|c| c.act);
        slot_box(r, index, Slot::Cond, cond.map(Block::Cond), unlocks, character);
        r.spawn(text("->", 11.0, color::FAINT));
        slot_box(r, index, Slot::Act, act.map(Block::Act), unlocks, character);
        if filled {
            if index > 0 {
                small_button(r, "up", UiAction::CardUp(index));
            }
            small_button(r, "x", UiAction::ClearCard(index));
        }
    });
}

fn slot_box(
    parent: &mut ChildSpawnerCommands,
    card: usize,
    slot: Slot,
    block: Option<Block>,
    unlocks: &Unlocks,
    character: Character,
) {
    let (fill, line, ink) = match block {
        None => (color::BG, color::LINE_STRONG, color::FAINT),
        Some(b) => chip_colors(b),
    };
    let label = match (slot, block) {
        (_, Some(Block::Cond(c))) => cond_label(c),
        (_, Some(Block::Act(a))) => action_label(a),
        (Slot::Cond, None) => "always".to_string(),
        (Slot::Act, None) => "drop an action".to_string(),
    };
    let mut e = parent.spawn(ui::button(
        UiAction::CycleSlot(card, slot),
        ui::ButtonStyle { fill, hover: line, border: line },
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            justify_content: JustifyContent::FlexStart,
            padding: UiRect::axes(px(8), px(5)),
            border_radius: BorderRadius::all(px(6)),
            overflow: Overflow::clip(),
            ..default()
        },
    ));
    e.insert(DropTarget::Card(card, slot));
    if let Some(b) = block {
        e.insert(cards::block_tooltip(b, unlocks, character));
    }
    e.with_child(text(label, 12.0, ink));
}

fn small_button(parent: &mut ChildSpawnerCommands, label: &str, action: UiAction) {
    parent
        .spawn(ui::button(
            action,
            ui::SECONDARY,
            Node {
                width: px(24),
                height: px(22),
                flex_shrink: 0.0,
                border_radius: BorderRadius::all(px(5)),
                ..default()
            },
        ))
        .with_child(text(label, 10.0, color::MUTED));
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

/// A palette chip: draggable and clickable when unlocked, greyed with its
/// requirement when locked. Both show a tooltip.
fn chip(parent: &mut ChildSpawnerCommands, block: Block, unlocks: &Unlocks, character: Character) {
    let req = cards::block_requirement(block);
    let unlocked = unlocks.allows(req, character);
    let tooltip = cards::block_tooltip(block, unlocks, character);
    let node = Node {
        padding: UiRect::axes(px(7), px(3)),
        border: UiRect::all(px(1)),
        border_radius: BorderRadius::all(px(6)),
        ..default()
    };
    if !unlocked {
        let label = match cards::requirement_label(req) {
            Some(r) => format!("{} - {r}", cards::block_label(block)),
            None => cards::block_label(block),
        };
        parent
            .spawn((node, BorderColor::all(color::LINE), Hovered::default(), tooltip))
            .with_child(text(label, 11.0, color::FAINT));
        return;
    }
    let (fill, line, ink) = chip_colors(block);
    let mut e = parent.spawn((
        Button,
        Hovered::default(),
        Clickable(UiAction::PaletteBlock(block)),
        DragSource::Block(block),
        Pickable { should_block_lower: false, is_hoverable: true },
        GlobalZIndex::default(),
        node,
        BackgroundColor(fill),
        BorderColor::all(line),
        ui::HoverColors { normal: fill, hover: line },
        tooltip,
    ));
    if let Block::Act(a) = block
        && let Some(ability) = a.ability()
    {
        e.insert(PaletteAbility(ability));
    }
    e.with_child(text(cards::block_label(block), 11.0, ink));
}

fn chip_group(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    blocks: Vec<Block>,
    unlocks: &Unlocks,
    character: Character,
) {
    parent.spawn(column(4.0)).with_children(|g| {
        g.spawn(text(title, 11.0, color::MUTED));
        g.spawn(Node { flex_wrap: FlexWrap::Wrap, row_gap: px(4), column_gap: px(4), ..default() })
            .with_children(|wrap| {
                for b in blocks {
                    chip(wrap, b, unlocks, character);
                }
            });
    });
}

fn palette(parent: &mut ChildSpawnerCommands, character: Character, unlocks: &Unlocks) {
    let (conds, acts) = cards::palette(character);
    parent.spawn(column(8.0)).with_children(|p| {
        chip_group(p, "Conditions", conds, unlocks, character);
        chip_group(p, "Actions", acts, unlocks, character);
    });
}

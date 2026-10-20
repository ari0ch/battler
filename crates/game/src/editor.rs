//! The program column on the planning screen: the selected unit's cards, always
//! editable, with the block palette underneath. Blocks are dragged (or clicked)
//! into cards; clicking a placed block cycles its setting. Every change is
//! written straight into the run, so a card only "counts" once it has an action.

use battler_sim::run::{Command, Lobby, Location, OwnedUnit, Player};
use battler_sim::{Ability, Action, Character, Condition, Unlocks};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Button;

use crate::cards::{self, action_label, cond_label};
use crate::kit;
use crate::sprites::{Sprites, sprite_node};
use crate::state::{DraftCard, LobbyExt, Planning, RunPhase};
use crate::ui::{
    self, Block, Clickable, DragSource, DropTarget, Slot, UiAction, color, column, heading, row,
    text,
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
pub fn selected_unit<'a>(
    run: &'a Player,
    planning: &Planning,
) -> Option<(Location, &'a OwnedUnit)> {
    let loc = run.locate(planning.selected?)?;
    Some((loc, run.unit_at(loc)?))
}

/// Loads the selected unit's program into the draft.
pub fn load_draft(run: &Player, planning: &mut Planning) {
    planning.draft = selected_unit(run, planning)
        .map(|(_, u)| cards::program_to_draft(&u.program))
        .unwrap_or_default();
}

/// Tries a new draft: if the program it makes is legal, keep it and save it
/// to the unit; otherwise leave everything unchanged and explain why.
fn try_draft(lobby: &mut Lobby, planning: &mut Planning, draft: Vec<DraftCard>) {
    let Some((loc, _)) = selected_unit(lobby.me(), planning) else { return };
    match lobby.act(Command::SetProgram(loc, cards::draft_to_program(&draft))) {
        Ok(_) => planning.draft = draft,
        Err(e) => planning.toast(e.to_string()),
    }
}

/// Puts a block into a card, then re-fits the action's target to the
/// condition (see [`cards::fit_target`]).
fn put(card: &mut DraftCard, block: Block) {
    match block {
        Block::Cond(c) => card.cond = Some(c),
        Block::Act(a) => card.act = Some(a),
    }
    card.act = card.act.map(|a| cards::fit_target(card.cond, a));
}

/// Drops a block into a specific card (an index one past the end starts a new card).
///
/// The block replaces whichever half of the card matches its kind, so dropping
/// an action onto a card's condition half (a near miss) still does what the
/// player meant, and dropping onto a filled half replaces it.
pub fn drop_block(lobby: &mut Lobby, planning: &mut Planning, card: usize, block: Block) {
    let Some((_, unit)) = selected_unit(lobby.me(), planning) else { return };
    let max = lobby.me().synergies().card_slots(unit.character, unit.level);
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
    try_draft(lobby, planning, draft);
}

/// Clicking a palette block: fill the first card missing that kind of block,
/// or start a new card.
pub fn click_block(lobby: &mut Lobby, planning: &mut Planning, block: Block) {
    let first_gap = planning.draft.iter().position(|c| match block {
        Block::Cond(_) => c.cond.is_none(),
        Block::Act(_) => c.act.is_none(),
    });
    let card = first_gap.unwrap_or(planning.draft.len());
    drop_block(lobby, planning, card, block);
}

pub fn cycle_slot(lobby: &mut Lobby, planning: &mut Planning, card: usize, slot: Slot) {
    let Some((_, unit)) = selected_unit(lobby.me(), planning) else { return };
    let character = unit.character;
    let unlocks = lobby.me().unlocks();
    let mut draft = planning.draft.clone();
    let Some(c) = draft.get_mut(card) else { return };
    match slot {
        Slot::Cond => c.cond = c.cond.map(cards::cycle_condition),
        Slot::Act => c.act = c.act.map(|a| cards::cycle_action(a, c.cond, &unlocks, character)),
    }
    try_draft(lobby, planning, draft);
}

/// Takes the block out of one half of a card. A card left with neither half
/// is removed.
pub fn remove_block(lobby: &mut Lobby, planning: &mut Planning, card: usize, slot: Slot) {
    let mut draft = planning.draft.clone();
    if take(&mut draft, card, slot).is_some() {
        try_draft(lobby, planning, draft);
    }
}

/// Moves a placed block onto another card (one past the end starts a new
/// card), where it replaces the matching half like a palette drop.
pub fn move_block(lobby: &mut Lobby, planning: &mut Planning, from: usize, slot: Slot, to: usize) {
    let Some((_, unit)) = selected_unit(lobby.me(), planning) else { return };
    let max = lobby.me().synergies().card_slots(unit.character, unit.level);
    let mut draft = planning.draft.clone();
    let Some(block) = draft.get(from).and_then(|c| match slot {
        Slot::Cond => c.cond.map(Block::Cond),
        Slot::Act => c.act.map(Block::Act),
    }) else {
        return;
    };
    if to >= draft.len() {
        if draft.len() >= max {
            planning.toast("No free card: merge copies to level this unit up");
            return;
        }
        draft.push(DraftCard::default());
    }
    let to = to.min(draft.len() - 1);
    // Put first, then take: taking may remove the source card and shift the
    // indices after it, but the destination has already been filled.
    put(&mut draft[to], block);
    take(&mut draft, from, slot);
    try_draft(lobby, planning, draft);
}

/// Empties one half of a card in a draft and returns what was there; drops the
/// card if both halves are now empty.
fn take(draft: &mut Vec<DraftCard>, card: usize, slot: Slot) -> Option<Block> {
    let c = draft.get_mut(card)?;
    let block = match slot {
        Slot::Cond => c.cond.take().map(Block::Cond),
        Slot::Act => c.act.take().map(Block::Act),
    };
    if c.cond.is_none() && c.act.is_none() {
        draft.remove(card);
    }
    block
}

pub fn clear_card(lobby: &mut Lobby, planning: &mut Planning, card: usize) {
    let mut draft = planning.draft.clone();
    if card < draft.len() {
        draft.remove(card);
        try_draft(lobby, planning, draft);
    }
}

pub fn card_up(lobby: &mut Lobby, planning: &mut Planning, card: usize) {
    let mut draft = planning.draft.clone();
    if card > 0 && card < draft.len() {
        draft.swap(card, card - 1);
        try_draft(lobby, planning, draft);
    }
}

// ----- Program column UI -----------------------------------------------

pub fn spawn_program_panel(
    parent: &mut ChildSpawnerCommands,
    run: &Player,
    planning: &Planning,
    sprites: &Sprites,
) {
    parent
        .spawn(kit::parchment_panel(Node {
            flex_grow: 1.0,
            min_width: px(0),
            padding: UiRect::axes(px(18), px(16)),
            ..column(8.0)
        }))
        .with_children(|p| {
            let Some((loc, unit)) = selected_unit(run, planning) else {
                p.spawn(heading("No unit selected", 17.0, color::INK));
                for line in [
                    "Buy units in the shop below. They go straight",
                    "onto the board when there's room.",
                    "Click a unit to program it here.",
                    "Drag units to move them; drop one on the shop",
                    "to sell it. Hover anything for details.",
                    "Keys: D reroll, F buy XP, E sell hovered unit.",
                ] {
                    p.spawn(text(line, 13.0, color::INK_MUTED));
                }
                return;
            };
            let unlocks = run.unlocks();
            header(p, unit, loc, sprites);
            cards_list(p, run, planning, unit);
            p.spawn((Node { height: px(2), ..default() }, BackgroundColor(DIVIDER)));
            palette(p, unit.character, &unlocks);
            p.spawn((Node { margin: UiRect::top(Val::Auto), ..default() }, Pickable::IGNORE))
                .with_child(text(
                    "Drag blocks into cards - click one to change it, drag it off to remove it",
                    11.0,
                    color::INK_FAINT,
                ));
        });
}

fn header(p: &mut ChildSpawnerCommands, unit: &OwnedUnit, loc: Location, sprites: &Sprites) {
    let c = unit.character;
    let stats = unit.level.scale(c.base_stats());
    p.spawn(row(10.0)).with_children(|r| {
        r.spawn((
            Node {
                width: px(48),
                height: px(48),
                flex_shrink: 0.0,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            kit::sliced(kit::rarity_frame(c.rarity()), 6.0),
            Hovered::default(),
            cards::unit_tooltip(c, Some(unit.level), None),
        ))
        .with_child(sprite_node(sprites.character(c), 1.0, false));
        r.spawn(Node { flex_grow: 1.0, min_width: px(0), ..column(1.0) }).with_children(|col| {
            col.spawn(row(6.0)).with_children(|n| {
                n.spawn(heading(c.short_name(), 18.0, color::INK));
                n.spawn(kit::icon(kit::class_icon(c.class()), 18.0));
                n.spawn(text(
                    format!("{} {}", c.rarity().name(), c.class().name()),
                    12.0,
                    color::INK_MUTED,
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
                12.0,
                color::INK_MUTED,
            ));
        });
        r.spawn(ui::button(
            UiAction::Sell,
            ui::SECONDARY,
            Node { height: px(36), padding: UiRect::horizontal(px(10)), ..row(4.0) },
        ))
        .with_children(|b| {
            b.spawn(heading("Sell", 13.0, color::TEXT));
            b.spawn(kit::icon(kit::img::ICON_COIN, 16.0));
            b.spawn(heading(unit.sell_value().to_string(), 13.0, color::TEXT));
        });
    });
}

fn cards_list(p: &mut ChildSpawnerCommands, run: &Player, planning: &Planning, unit: &OwnedUnit) {
    let max = run.synergies().card_slots(unit.character, unit.level);
    let unlocks = run.unlocks();
    p.spawn(column(5.0)).with_children(|list| {
        for (i, card) in planning.draft.iter().enumerate() {
            card_row(list, i, Some(*card), &unlocks, unit.character);
        }
        if planning.draft.len() < max {
            card_row(list, planning.draft.len(), None, &unlocks, unit.character);
        }
        list.spawn(text("Otherwise: attack nearest (always last, locked)", 12.0, color::INK_MUTED));
        if planning.draft.len() >= max {
            list.spawn(text(
                format!("{max} of {max} cards - merge copies for more"),
                11.0,
                color::INK_FAINT,
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
    // Filled cards sit on a parchment strip; the next empty one is dashed.
    let strip = if filled { kit::img::CARD_ROW } else { kit::img::TAG_EMPTY };
    list.spawn((Node { padding: UiRect::axes(px(6), px(5)), ..row(6.0) }, kit::sliced(strip, 4.0)))
        .with_children(|r| {
            // The card number on a red wax seal.
            r.spawn((
                Node {
                    width: px(26),
                    height: px(26),
                    flex_shrink: 0.0,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ImageNode::new(kit::img::WAX_SEAL),
                Pickable::IGNORE,
            ))
            .with_child(heading(
                (index + 1).to_string(),
                12.0,
                Color::srgb(1.0, 0.906, 0.886),
            ));
            let cond: Option<Condition> = card.and_then(|c| c.cond);
            let act: Option<Action> = card.and_then(|c| c.act);
            slot_box(r, index, Slot::Cond, cond.map(Block::Cond), unlocks, character);
            r.spawn(heading(">", 14.0, Color::srgb(0.549, 0.353, 0.165)));
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
    let ink = block.map_or(color::INK_FAINT, block_ink);
    let label = match (slot, block) {
        (_, Some(Block::Cond(c))) => cond_label(c),
        (_, Some(Block::Act(a))) => action_label(a),
        (Slot::Cond, None) => "always".to_string(),
        (Slot::Act, None) => "drop an action".to_string(),
    };
    let mut e = parent.spawn(ui::button(
        UiAction::CycleSlot(card, slot),
        ui::ButtonStyle::flat(Color::NONE, Color::NONE, Color::NONE),
        Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            height: px(30),
            justify_content: JustifyContent::FlexStart,
            padding: UiRect::axes(px(9), px(4)),
            overflow: Overflow::clip(),
            ..default()
        },
    ));
    // The block's coloured tag, or a dashed outline when empty.
    e.insert((DropTarget::Card(card), kit::sliced(tag_image(block), 4.0)))
        .remove::<ui::HoverSkin>();
    if let Some(b) = block {
        // A placed block can be dragged to another card, or off to remove it.
        // It lets the pointer through so the card underneath gets the drop.
        e.insert((
            cards::block_tooltip(b, unlocks, character),
            DragSource::Placed(card, slot),
            Pickable { should_block_lower: false, is_hoverable: true },
        ));
    }
    // An action with a target shows it as a pill, so it reads as a setting you
    // can change: clicking the slot cycles it.
    if let Some(Block::Act(a)) = block
        && let Some(t) = a.target()
    {
        e.insert(Node {
            flex_grow: 1.0,
            flex_basis: px(0),
            min_width: px(0),
            height: px(30),
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Center,
            column_gap: px(6),
            padding: UiRect::axes(px(9), px(4)),
            overflow: Overflow::clip(),
            ..default()
        });
        e.with_children(|slot| {
            slot.spawn(heading(cards::action_name(a), 13.0, ink));
            slot.spawn((
                Node {
                    padding: UiRect::axes(px(5), px(1)),
                    border: UiRect::all(px(2)),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.18)),
                BorderColor::all(Color::srgba(0.102, 0.078, 0.149, 0.55)),
                Pickable::IGNORE,
            ))
            .with_child(heading(format!("{} v", cards::target_label(t)), 12.0, ink));
        });
        return;
    }
    e.with_child(heading(label, 13.0, ink));
}

fn small_button(parent: &mut ChildSpawnerCommands, label: &str, action: UiAction) {
    parent
        .spawn(ui::button(
            action,
            ui::SECONDARY,
            Node { width: px(30), height: px(30), flex_shrink: 0.0, ..default() },
        ))
        .with_child(heading(label, 12.0, color::TEXT));
}

/// The divider colour between sections of the parchment panel.
const DIVIDER: Color = Color::srgb(0.788, 0.663, 0.431);

/// The coloured tag a block sits on: blue conditions, orange actions, purple
/// signature abilities. `None` is an empty, dashed slot.
fn tag_image(block: Option<Block>) -> Handle<Image> {
    match block {
        None => kit::img::TAG_EMPTY,
        Some(Block::Cond(_)) => kit::img::TAG_CONDITION,
        Some(Block::Act(a)) if a.ability().is_some() => kit::img::TAG_SIGNATURE,
        Some(Block::Act(_)) => kit::img::TAG_ACTION,
    }
}

/// Text colour that reads on a block's tag.
fn block_ink(block: Block) -> Color {
    match block {
        Block::Cond(_) => color::COND_TEXT,
        Block::Act(a) if a.ability().is_some() => color::SIG_TEXT,
        Block::Act(_) => color::ACT_TEXT,
    }
}

/// A palette chip: draggable and clickable, with a tooltip. Only unlocked
/// blocks get one; locked blocks stay hidden until they unlock (the level-up
/// notice announces them).
fn chip(parent: &mut ChildSpawnerCommands, block: Block, unlocks: &Unlocks, character: Character) {
    let tooltip = cards::block_tooltip(block, unlocks, character);
    let node = Node { height: px(28), padding: UiRect::axes(px(8), px(3)), ..row(0.0) };
    let ink = block_ink(block);
    // Dimmed until hovered, so the block under the pointer stands out.
    let dim = Color::srgb(0.9, 0.9, 0.9);
    let image = tag_image(Some(block));
    let mut e = parent.spawn((
        Button,
        Hovered::default(),
        Clickable(UiAction::PaletteBlock(block)),
        DragSource::Block(block),
        Pickable { should_block_lower: false, is_hoverable: true },
        node,
        kit::sliced(image.clone(), 4.0).with_color(dim),
        ui::HoverSkin { normal: (image.clone(), dim), hover: (image, Color::WHITE) },
        tooltip,
    ));
    if let Block::Act(a) = block
        && let Some(ability) = a.ability()
    {
        e.insert(PaletteAbility(ability));
    }
    e.with_child(heading(cards::block_label(block), 12.0, ink));
}

fn chip_group(
    parent: &mut ChildSpawnerCommands,
    title: &str,
    ink: Color,
    blocks: Vec<Block>,
    unlocks: &Unlocks,
    character: Character,
) {
    parent.spawn(column(4.0)).with_children(|g| {
        g.spawn(heading(title, 12.0, ink));
        g.spawn(Node { flex_wrap: FlexWrap::Wrap, row_gap: px(4), column_gap: px(4), ..default() })
            .with_children(|wrap| {
                for b in blocks {
                    chip(wrap, b, unlocks, character);
                }
            });
    });
}

fn palette(parent: &mut ChildSpawnerCommands, character: Character, unlocks: &Unlocks) {
    let (mut conds, mut acts) = cards::palette(character);
    // `retain` filters a Vec in place; the closure gets a reference to each item.
    let usable = |b: &Block| unlocks.allows(cards::block_requirement(*b), character);
    conds.retain(usable);
    acts.retain(usable);
    parent.spawn(column(8.0)).with_children(|p| {
        chip_group(p, "Conditions", Color::srgb(0.173, 0.373, 0.478), conds, unlocks, character);
        chip_group(p, "Actions", Color::srgb(0.541, 0.290, 0.063), acts, unlocks, character);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(cond: Option<Condition>, act: Option<Action>) -> DraftCard {
        DraftCard { cond, act }
    }

    #[test]
    fn taking_one_half_keeps_the_card() {
        let mut draft = vec![card(Some(Condition::EnemyAdjacent), Some(Action::Guard))];
        assert_eq!(take(&mut draft, 0, Slot::Cond), Some(Block::Cond(Condition::EnemyAdjacent)));
        assert_eq!(draft, vec![card(None, Some(Action::Guard))]);
    }

    #[test]
    fn taking_the_last_half_removes_the_card() {
        let mut draft =
            vec![card(None, Some(Action::Guard)), card(Some(Condition::EnemyInRange), None)];
        assert_eq!(take(&mut draft, 0, Slot::Act), Some(Block::Act(Action::Guard)));
        assert_eq!(draft, vec![card(Some(Condition::EnemyInRange), None)]);
    }

    #[test]
    fn taking_from_an_empty_half_or_missing_card_does_nothing() {
        let mut draft = vec![card(None, Some(Action::Guard))];
        assert_eq!(take(&mut draft, 0, Slot::Cond), None);
        assert_eq!(take(&mut draft, 5, Slot::Act), None);
        assert_eq!(draft.len(), 1);
    }
}

//! Shared UI plumbing: colours, text and button helpers, the fixed-size stage
//! that scales to the window, hover styling, and routing clicks and drops to
//! [`UiAction`]s.
//!
//! Every clickable element is a headless [`Button`] carrying a [`Clickable`]
//! with the action it performs. One global observer turns `Activate` events into
//! queued actions; `actions::apply_actions` then applies them. Drag and drop works
//! the same way via [`DragSource`] and [`DropTarget`].

use battler_sim::run::Location;
use battler_sim::{Action, Condition, Pos};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::{Activate, Button};
use bevy::window::PrimaryWindow;

use crate::kit;

/// Logical size the whole UI is designed for; it scales to fit the window.
pub const STAGE_W: f32 = 1280.0;
pub const STAGE_H: f32 = 720.0;

pub mod color {
    use bevy::prelude::Color;

    // Fantasy-cartoon palette: deep plum ground, warm wood, cream text and gold
    // trim, matching the pixel-art kit (`kit.rs`).
    pub const BG: Color = Color::srgb(0.114, 0.102, 0.169);
    pub const BG_DEEP: Color = Color::srgb(0.078, 0.067, 0.125);
    pub const PANEL_2: Color = Color::srgb(0.227, 0.192, 0.314);
    pub const LINE_STRONG: Color = Color::srgb(0.361, 0.227, 0.133);
    pub const CELL_HOVER: Color = Color::srgba(1.0, 0.95, 0.8, 0.18);
    pub const TEXT: Color = Color::srgb(0.953, 0.902, 0.769);
    pub const MUTED: Color = Color::srgb(0.804, 0.718, 0.561);
    pub const FAINT: Color = Color::srgb(0.612, 0.518, 0.400);
    /// Ink on parchment.
    pub const INK: Color = Color::srgb(0.231, 0.141, 0.086);
    pub const INK_MUTED: Color = Color::srgb(0.420, 0.302, 0.180);
    pub const INK_FAINT: Color = Color::srgb(0.560, 0.450, 0.330);
    pub const GOLD: Color = Color::srgb(0.957, 0.706, 0.106);
    pub const GOLD_LIGHT: Color = Color::srgb(1.0, 0.824, 0.353);
    pub const ON_GOLD: Color = Color::srgb(0.290, 0.165, 0.0);
    pub const BLUE: Color = Color::srgb(0.314, 0.588, 1.0);
    pub const BLUE_FILL: Color = Color::srgb(0.114, 0.200, 0.322);
    pub const BLUE_TEXT: Color = Color::srgb(0.612, 0.827, 1.0);
    pub const RED_TEXT: Color = Color::srgb(1.0, 0.604, 0.541);
    pub const GOOD: Color = Color::srgb(0.608, 0.878, 0.478);
    pub const BAD: Color = Color::srgb(1.0, 0.541, 0.478);
    pub const CRIT: Color = Color::srgb(1.0, 0.824, 0.353);
    pub const COND_LINE: Color = Color::srgb(0.486, 0.776, 0.863);
    pub const COND_TEXT: Color = Color::srgb(0.059, 0.165, 0.227);
    pub const ACT_LINE: Color = Color::srgb(1.0, 0.824, 0.353);
    pub const ACT_TEXT: Color = Color::srgb(0.227, 0.102, 0.016);
    pub const SIG_LINE: Color = Color::srgb(0.604, 0.478, 0.769);
    pub const SIG_TEXT: Color = Color::srgb(0.965, 0.933, 1.0);
    pub const COMMON: Color = Color::srgb(0.906, 0.827, 0.659);
    pub const RARE: Color = Color::srgb(0.486, 0.776, 0.878);
    pub const EPIC: Color = Color::srgb(0.851, 0.651, 1.0);
    pub const ACTIVE_FILL: Color = Color::srgba(0.957, 0.706, 0.106, 0.14);
    /// Outline drawn around pixel-art text and numbers.
    pub const OUTLINE: Color = Color::srgb(0.102, 0.078, 0.149);
}

/// A rule-card block: what the palette offers and what card slots hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Block {
    Cond(Condition),
    Act(Action),
}

/// Which half of a rule card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Cond,
    Act,
}

/// Something the player can drag.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragSource {
    Unit(Location),
    Block(Block),
}

/// Somewhere a drag can be dropped.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropTarget {
    Cell(Pos),
    /// A cell found from where the pointer is on the isometric field (see
    /// `field::FieldPicker`). A drop straight onto a unit (`Cell`) wins.
    Field(Pos),
    Bench(usize),
    Sell,
    /// A program card, by index (one past the end starts a new card).
    Card(usize),
}

/// Everything the player can do by clicking or dropping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiAction {
    NewRun,
    ToggleHowTo,
    MainMenu,
    Buy(usize),
    Reroll,
    Ready,
    BuyXp,
    SelectUnit(Location),
    ClickCell(Pos),
    ClickBench(usize),
    /// Sell the selected unit.
    Sell,
    /// Sell a specific unit (the E hotkey sells whatever is under the pointer).
    SellAt(Location),
    DismissNotice,
    /// Open the program panel, or dock it to its tab.
    ToggleEditor,
    PaletteBlock(Block),
    /// Clicking a filled slot cycles its setting (threshold, target...).
    CycleSlot(usize, Slot),
    ClearCard(usize),
    CardUp(usize),
    Speed(u32),
    Skip,
    ToggleTrace,
    /// Close the round result pop-up.
    Continue,
    Replay(usize),
    BackToSummary,
    Drop(DragSource, DropTarget),
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Clickable(pub UiAction);

/// Actions waiting to be applied this frame.
#[derive(Resource, Default)]
pub struct ActionQueue(pub Vec<UiAction>);

/// Hover text for any element that also has [`Hovered`].
#[derive(Component, Debug, Clone)]
pub struct Tooltip {
    pub title: String,
    pub accent: Color,
    pub lines: Vec<(String, Color)>,
}

/// Background colours for hover feedback.
#[derive(Component, Debug, Clone, Copy)]
pub struct HoverColors {
    pub normal: Color,
    pub hover: Color,
}

/// Image and tint for hover feedback on skinned (image) buttons.
#[derive(Component, Debug, Clone)]
pub struct HoverSkin {
    pub normal: (Handle<Image>, Color),
    pub hover: (Handle<Image>, Color),
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionQueue>()
            .add_observer(on_activate)
            .add_observer(on_drag_start)
            .add_observer(on_drag)
            .add_observer(on_drag_end)
            .add_observer(on_drop)
            .add_systems(Startup, spawn_tooltip)
            .add_systems(Update, (fit_ui_to_window, hover_colors, hover_skins, update_tooltip));
    }
}

fn on_activate(ev: On<Activate>, clickables: Query<&Clickable>, mut queue: ResMut<ActionQueue>) {
    if let Ok(c) = clickables.get(ev.entity) {
        queue.0.push(c.0);
    }
}

/// Lifts a dragged element above everything else while it moves.
///
/// `GlobalZIndex` takes a node out of its parent's stacking order and sorts it
/// among the top-level nodes instead, so it is only added for the drag: left
/// on at rest (even as 0), it would sink draggables under raised panels like
/// the program overlay and undo the depth order of units on the field.
fn on_drag_start(
    ev: On<bevy::picking::events::PointerDragStart>,
    q: Query<(), With<DragSource>>,
    mut commands: Commands,
) {
    if q.contains(ev.entity) {
        commands.entity(ev.entity).insert(GlobalZIndex(100));
    }
}

fn on_drag(
    ev: On<bevy::picking::events::PointerDrag>,
    scale: Res<UiScale>,
    mut q: Query<&mut UiTransform, With<DragSource>>,
) {
    if let Ok(mut t) = q.get_mut(ev.entity) {
        // Pointer distance is in window pixels; `Val::Px` is multiplied by `UiScale`.
        let s = scale.0.max(0.01);
        t.translation = Val2::px(ev.distance.x / s, ev.distance.y / s);
    }
}

fn on_drag_end(
    ev: On<bevy::picking::events::PointerDragEnd>,
    mut q: Query<&mut UiTransform, With<DragSource>>,
    mut commands: Commands,
) {
    if let Ok(mut t) = q.get_mut(ev.entity) {
        t.translation = Val2::ZERO;
        commands.entity(ev.entity).remove::<GlobalZIndex>();
    }
}

fn on_drop(
    ev: On<bevy::picking::events::PointerDragDrop>,
    targets: Query<&DropTarget>,
    sources: Query<&DragSource>,
    mut queue: ResMut<ActionQueue>,
) {
    // Bevy sends a drop to *every* element under the pointer, and each one
    // bubbles up through its ancestors. Dropping onto an occupied cell therefore
    // reaches the cell twice: directly, and via the unit standing on it. Applying
    // both would swap twice and undo the move, so queue each drop only once.
    if let (Ok(target), Ok(source)) = (targets.get(ev.entity), sources.get(ev.dropped)) {
        let action = UiAction::Drop(*source, *target);
        if !queue.0.contains(&action) {
            queue.0.push(action);
        }
    }
}

/// Scales the UI so the 1280x720 stage fits the window.
fn fit_ui_to_window(window: Single<&Window, With<PrimaryWindow>>, mut scale: ResMut<UiScale>) {
    let s = (window.width() / STAGE_W).min(window.height() / STAGE_H).max(0.1);
    if (scale.0 - s).abs() > 0.001 {
        scale.0 = s;
    }
}

fn hover_colors(mut q: Query<(&Hovered, &HoverColors, &mut BackgroundColor), Changed<Hovered>>) {
    for (hovered, colors, mut bg) in &mut q {
        bg.0 = if hovered.get() { colors.hover } else { colors.normal };
    }
}

fn hover_skins(mut q: Query<(&Hovered, &HoverSkin, &mut ImageNode), Changed<Hovered>>) {
    for (hovered, skin, mut image) in &mut q {
        let (handle, tint) = if hovered.get() { &skin.hover } else { &skin.normal };
        image.image = handle.clone();
        image.color = *tint;
    }
}

#[derive(Component)]
struct TooltipRoot;

/// One floating tooltip panel, shared by every [`Tooltip`] element.
fn spawn_tooltip(mut commands: Commands) {
    commands.spawn((
        TooltipRoot,
        Node {
            position_type: PositionType::Absolute,
            display: Display::None,
            width: px(TOOLTIP_W),
            padding: UiRect::all(px(18)),
            flex_direction: FlexDirection::Column,
            row_gap: px(5),
            ..default()
        },
        kit::sliced(kit::img::PANEL_WOOD, 10.0),
        GlobalZIndex(1000),
        Pickable::IGNORE,
    ));
}

const TOOLTIP_W: f32 = 300.0;

/// Shows the tooltip of whatever [`Tooltip`] element is hovered, next to the pointer.
fn update_tooltip(
    mut commands: Commands,
    window: Single<&Window, With<PrimaryWindow>>,
    scale: Res<UiScale>,
    hovered: Query<(Entity, &Hovered, &Tooltip)>,
    root: Single<(Entity, &mut Node), With<TooltipRoot>>,
    mut showing: Local<Option<Entity>>,
) {
    let (root_entity, mut node) = root.into_inner();
    let target = hovered.iter().find(|(_, h, _)| h.get());
    let (Some((entity, _, tip)), Some(cursor)) = (target, window.cursor_position()) else {
        node.display = Display::None;
        *showing = None;
        return;
    };
    let s = scale.0.max(0.01);
    // `Val::Px` is in UI units, which UiScale multiplies; the cursor is in window pixels.
    let (mut x, mut y) = (cursor.x / s + 16.0, cursor.y / s + 16.0);
    let (w, h) = (window.width() / s, window.height() / s);
    if x + TOOLTIP_W > w {
        x = (cursor.x / s - TOOLTIP_W - 12.0).max(0.0);
    }
    if y + 160.0 > h {
        y = (h - 170.0).max(0.0);
    }
    node.display = Display::Flex;
    node.left = px(x);
    node.top = px(y);
    if *showing != Some(entity) {
        *showing = Some(entity);
        commands.entity(root_entity).despawn_children().with_children(|t| {
            t.spawn(heading(tip.title.clone(), 16.0, tip.accent));
            for (line, ink) in &tip.lines {
                t.spawn(text(line.clone(), 12.0, *ink));
            }
        });
    }
}

// ----- Builders ---------------------------------------------------------

/// A full-window root that centres a fixed-size stage. Returns the stage entity.
pub fn spawn_stage(commands: &mut Commands, scope: impl Bundle, background: Color) -> Entity {
    let mut stage = Entity::PLACEHOLDER;
    commands
        .spawn((
            scope,
            Node {
                width: percent(100),
                height: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(color::BG_DEEP),
            Pickable::IGNORE,
        ))
        .with_children(|root| {
            stage = root
                .spawn((
                    Node {
                        width: px(STAGE_W),
                        height: px(STAGE_H),
                        flex_direction: FlexDirection::Column,
                        overflow: Overflow::clip(),
                        ..default()
                    },
                    BackgroundColor(background),
                ))
                .id();
        });
    stage
}

fn text_font(size: f32, weight: FontWeight) -> TextFont {
    TextFont { font: kit::FONT.into(), font_size: FontSize::Px(size), weight, ..default() }
}

/// Text in the kit's pixel font that never intercepts the pointer (so clicks
/// reach the button behind it).
pub fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(value), text_font(size, FontWeight::NORMAL), TextColor(color), Pickable::IGNORE)
}

/// Bold text for titles and numbers.
pub fn heading(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (Text::new(value), text_font(size, FontWeight::BOLD), TextColor(color), Pickable::IGNORE)
}

/// Bold text with a dark drop shadow, for numbers over the battlefield.
pub fn outlined(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (heading(value, size, color), TextShadow { offset: Vec2::new(2.0, 2.0), color: color::OUTLINE })
}

/// What a button is drawn with.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skin {
    /// A wooden plank (most buttons).
    Wood,
    /// Gold, for the main action on a screen.
    Gold,
    /// A pressed-in plank, for toggles that are on.
    Pressed,
    /// A parchment card in a wooden frame (shop offers).
    Parchment,
    /// A flat coloured box, using the style's colours.
    Flat,
}

/// Visual style for [`button`].
#[derive(Clone, Copy)]
pub struct ButtonStyle {
    pub skin: Skin,
    pub fill: Color,
    pub hover: Color,
    pub border: Color,
}

impl ButtonStyle {
    /// A flat box in the given colours.
    pub const fn flat(fill: Color, hover: Color, border: Color) -> Self {
        Self { skin: Skin::Flat, fill, hover, border }
    }

    const fn skinned(skin: Skin) -> Self {
        Self { skin, fill: Color::NONE, hover: Color::NONE, border: Color::NONE }
    }

    /// The text colour that reads well on this style.
    pub fn ink(&self) -> Color {
        match self.skin {
            Skin::Gold => color::ON_GOLD,
            Skin::Pressed => color::GOLD_LIGHT,
            Skin::Parchment => color::INK,
            Skin::Wood | Skin::Flat => color::TEXT,
        }
    }
}

pub const PRIMARY: ButtonStyle = ButtonStyle::skinned(Skin::Gold);
pub const SECONDARY: ButtonStyle = ButtonStyle::skinned(Skin::Wood);
pub const TOGGLED: ButtonStyle = ButtonStyle::skinned(Skin::Pressed);
pub const PARCHMENT: ButtonStyle = ButtonStyle::skinned(Skin::Parchment);

/// A clickable box. Add content as children.
///
/// `impl Bundle` must be one concrete type, so every skin produces the same
/// set of components and only their values differ: image skins get a sliced
/// image and no background colour; flat buttons get colours and a hidden image.
pub fn button(action: UiAction, style: ButtonStyle, node: Node) -> impl Bundle {
    // Gold and parchment brighten on hover; wood swaps to a lighter plank.
    let dim = Color::srgb(0.88, 0.88, 0.88);
    let (image, slice, normal_tint, hover_image, hover_tint) = match style.skin {
        Skin::Wood => (kit::img::BUTTON, 6.0, Color::WHITE, kit::img::BUTTON_HOVER, Color::WHITE),
        Skin::Gold => (kit::img::BUTTON_GOLD, 6.0, dim, kit::img::BUTTON_GOLD, Color::WHITE),
        Skin::Pressed => {
            (kit::img::BUTTON_PRESSED, 6.0, Color::WHITE, kit::img::BUTTON_PRESSED, Color::WHITE)
        }
        Skin::Parchment => {
            (kit::img::PANEL_PARCHMENT, 10.0, dim, kit::img::PANEL_PARCHMENT, Color::WHITE)
        }
        Skin::Flat => (Handle::default(), 1.0, Color::NONE, Handle::default(), Color::NONE),
    };
    (
        Button,
        Hovered::default(),
        Clickable(action),
        Node { justify_content: JustifyContent::Center, align_items: AlignItems::Center, ..node },
        BackgroundColor(style.fill),
        BorderColor::all(style.border),
        HoverColors { normal: style.fill, hover: style.hover },
        kit::sliced(image.clone(), slice).with_color(normal_tint),
        HoverSkin { normal: (image, normal_tint), hover: (hover_image, hover_tint) },
    )
}

/// A labelled button of a given size, in one call.
pub fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: UiAction,
    style: ButtonStyle,
    width: Val,
    height: Val,
) {
    parent
        .spawn(button(
            action,
            style,
            Node { width, height, padding: UiRect::horizontal(px(14)), ..default() },
        ))
        .with_child(heading(label, 16.0, style.ink()));
}

/// A dark wooden panel (light text goes on it). Its padding clears the frame.
pub fn panel(node: Node) -> impl Bundle {
    kit::wood_panel(node)
}

/// A row with a gap, for laying out children horizontally.
pub fn row(gap: f32) -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        column_gap: px(gap),
        align_items: AlignItems::Center,
        ..default()
    }
}

/// A column with a gap.
pub fn column(gap: f32) -> Node {
    Node { flex_direction: FlexDirection::Column, row_gap: px(gap), ..default() }
}

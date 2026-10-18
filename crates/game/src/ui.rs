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

/// Logical size the whole UI is designed for; it scales to fit the window.
pub const STAGE_W: f32 = 1280.0;
pub const STAGE_H: f32 = 720.0;

pub mod color {
    use bevy::prelude::Color;

    pub const BG: Color = Color::srgb(0.078, 0.071, 0.102);
    pub const BG_DEEP: Color = Color::srgb(0.063, 0.055, 0.082);
    pub const BAR: Color = Color::srgb(0.102, 0.090, 0.133);
    pub const PANEL: Color = Color::srgb(0.118, 0.106, 0.149);
    pub const PANEL_2: Color = Color::srgb(0.149, 0.133, 0.188);
    pub const LINE: Color = Color::srgb(0.180, 0.165, 0.227);
    pub const LINE_STRONG: Color = Color::srgb(0.227, 0.204, 0.282);
    pub const CELL: Color = Color::srgb(0.141, 0.125, 0.180);
    pub const CELL_ENEMY: Color = Color::srgb(0.102, 0.090, 0.133);
    pub const TEXT: Color = Color::srgb(0.925, 0.910, 0.949);
    pub const MUTED: Color = Color::srgb(0.643, 0.616, 0.702);
    pub const FAINT: Color = Color::srgb(0.557, 0.525, 0.627);
    pub const GOLD: Color = Color::srgb(0.890, 0.690, 0.294);
    pub const GOLD_LIGHT: Color = Color::srgb(0.949, 0.788, 0.416);
    pub const GOLD_DARK: Color = Color::srgb(0.227, 0.188, 0.098);
    pub const ON_GOLD: Color = Color::srgb(0.102, 0.078, 0.031);
    pub const BLUE: Color = Color::srgb(0.302, 0.549, 0.922);
    pub const BLUE_FILL: Color = Color::srgb(0.114, 0.200, 0.322);
    pub const BLUE_TEXT: Color = Color::srgb(0.812, 0.878, 0.980);
    pub const RED: Color = Color::srgb(0.878, 0.341, 0.247);
    pub const RED_FILL: Color = Color::srgb(0.290, 0.125, 0.098);
    pub const RED_TEXT: Color = Color::srgb(0.973, 0.831, 0.800);
    pub const GOOD: Color = Color::srgb(0.498, 0.831, 0.604);
    pub const BAD: Color = Color::srgb(0.941, 0.604, 0.541);
    pub const HP_FILL: Color = Color::srgb(0.373, 0.761, 0.494);
    pub const HP_LOW: Color = Color::srgb(0.910, 0.639, 0.235);
    pub const CRIT: Color = Color::srgb(0.961, 0.773, 0.259);
    pub const COND_FILL: Color = Color::srgb(0.176, 0.231, 0.165);
    pub const COND_LINE: Color = Color::srgb(0.247, 0.333, 0.224);
    pub const COND_TEXT: Color = Color::srgb(0.749, 0.890, 0.710);
    pub const ACT_FILL: Color = Color::srgb(0.165, 0.188, 0.275);
    pub const ACT_LINE: Color = Color::srgb(0.235, 0.267, 0.384);
    pub const ACT_TEXT: Color = Color::srgb(0.788, 0.827, 0.949);
    pub const SIG_FILL: Color = Color::srgb(0.227, 0.173, 0.290);
    pub const SIG_LINE: Color = Color::srgb(0.333, 0.251, 0.408);
    pub const SIG_TEXT: Color = Color::srgb(0.863, 0.796, 0.949);
    pub const REACH: Color = Color::srgb(0.173, 0.133, 0.251);
    pub const REACH_LINE: Color = Color::srgb(0.431, 0.333, 0.604);
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
    Bench(usize),
    Sell,
    Card(usize, Slot),
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
    SelectUnit(Location),
    ClickCell(Pos),
    ClickBench(usize),
    Sell,
    OpenEditor,
    CloseEditor,
    PaletteBlock(Block),
    /// Clicking a filled slot cycles its setting (threshold, target...).
    CycleSlot(usize, Slot),
    ClearCard(usize),
    CardUp(usize),
    Speed(u32),
    Skip,
    ToggleTrace,
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

/// Background colours for hover feedback.
#[derive(Component, Debug, Clone, Copy)]
pub struct HoverColors {
    pub normal: Color,
    pub hover: Color,
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
            .add_systems(Update, (fit_ui_to_window, hover_colors));
    }
}

fn on_activate(ev: On<Activate>, clickables: Query<&Clickable>, mut queue: ResMut<ActionQueue>) {
    if let Ok(c) = clickables.get(ev.entity) {
        queue.0.push(c.0);
    }
}

fn on_drag_start(
    ev: On<bevy::picking::events::PointerDragStart>,
    mut q: Query<&mut GlobalZIndex, With<DragSource>>,
) {
    if let Ok(mut z) = q.get_mut(ev.entity) {
        z.0 = 100;
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
    mut q: Query<(&mut UiTransform, &mut GlobalZIndex), With<DragSource>>,
) {
    if let Ok((mut t, mut z)) = q.get_mut(ev.entity) {
        t.translation = Val2::ZERO;
        z.0 = 0;
    }
}

fn on_drop(
    ev: On<bevy::picking::events::PointerDragDrop>,
    targets: Query<&DropTarget>,
    sources: Query<&DragSource>,
    mut queue: ResMut<ActionQueue>,
) {
    // The event bubbles up through ancestors; act only on the drop target itself.
    if let (Ok(target), Ok(source)) = (targets.get(ev.entity), sources.get(ev.dropped)) {
        queue.0.push(UiAction::Drop(*source, *target));
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

/// Text that never intercepts the pointer (so clicks reach the button behind it).
pub fn text(value: impl Into<String>, size: f32, color: Color) -> impl Bundle {
    (
        Text::new(value),
        TextFont { font_size: FontSize::Px(size), ..default() },
        TextColor(color),
        Pickable::IGNORE,
    )
}

/// Visual style for [`button`].
#[derive(Clone, Copy)]
pub struct ButtonStyle {
    pub fill: Color,
    pub hover: Color,
    pub border: Color,
}

pub const PRIMARY: ButtonStyle =
    ButtonStyle { fill: color::GOLD, hover: color::GOLD_LIGHT, border: color::GOLD };
pub const SECONDARY: ButtonStyle =
    ButtonStyle { fill: color::PANEL, hover: color::PANEL_2, border: color::LINE_STRONG };
pub const TOGGLED: ButtonStyle =
    ButtonStyle { fill: color::GOLD_DARK, hover: color::GOLD_DARK, border: color::GOLD };

/// A clickable box. Add content as children.
pub fn button(action: UiAction, style: ButtonStyle, node: Node) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Clickable(action),
        Node {
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(8)),
            ..node
        },
        BackgroundColor(style.fill),
        BorderColor::all(style.border),
        HoverColors { normal: style.fill, hover: style.hover },
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
    let text_color = if style.fill == color::GOLD { color::ON_GOLD } else { color::TEXT };
    parent
        .spawn(button(
            action,
            style,
            Node { width, height, padding: UiRect::horizontal(px(14)), ..default() },
        ))
        .with_child(text(label, 16.0, text_color));
}

/// A simple filled panel node.
pub fn panel(node: Node) -> impl Bundle {
    (
        Node { border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(12)), ..node },
        BackgroundColor(color::PANEL),
        BorderColor::all(color::LINE),
    )
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

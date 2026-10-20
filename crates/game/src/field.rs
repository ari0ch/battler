//! The isometric battlefield: where things are drawn, and which cell the
//! pointer is over.
//!
//! The sim's grid (depth `x` 0..8, lane `y` 0..7) is drawn as a diamond of
//! 96 x 48 tiles: depth runs up and to the right, lanes down and to the right,
//! so your half is bottom-left and the enemy's top-right. Three spaces meet here:
//!
//! - **cells** (`Pos`), the sim's;
//! - **ground**, a flat plane where one cell is [`GROUND`] pixels square, used
//!   by the free movement in `motion.rs` (x along lanes, y along depth);
//! - **screen**, pixels inside the field node, where the diamond is drawn.
//!
//! Projection is a linear map, so going back from screen to cell is exact: a
//! diamond on screen is a square in (depth, lane) space, and rounding each
//! coordinate finds the cell. That is how drops and clicks land on diamonds
//! even though UI nodes are rectangles (see [`FieldPicker`]).

use battler_sim::Pos;
use battler_sim::run::economy::{BOARD_DEPTH, BOARD_LANES, SIDE_DEPTH};
use bevy::picking::events::{PointerClick, PointerDragDrop, PointerMove, PointerOut};
use bevy::prelude::*;

use crate::kit;
use crate::ui::{ActionQueue, DragSource, DropTarget, UiAction};

/// Half a tile: the diamond is twice as wide as it is tall.
const HALF_W: f32 = 48.0;
const HALF_H: f32 = 24.0;
/// Room above the board's top corner for the back row's sprites and bars.
pub const HEADROOM: f32 = 96.0;
/// The board image (diamond plus its slab of earth) at 2x.
const BOARD_IMG_H: f32 = 384.0;
pub const FIELD_W: f32 = HALF_W * (BOARD_DEPTH + BOARD_LANES) as f32;
pub const FIELD_H: f32 = HEADROOM + BOARD_IMG_H;
/// One cell on the ground plane, in ground pixels.
pub const GROUND: f32 = 66.0;

/// Screen position of a point given in (fractional) depth and lane, where
/// whole numbers are cell centres.
pub fn to_screen(depth: f32, lane: f32) -> Vec2 {
    Vec2::new(
        HALF_W * (depth + lane + 1.0),
        HEADROOM + HALF_H * (lane - depth + BOARD_DEPTH as f32),
    )
}

/// The centre of a cell's diamond on screen.
pub fn cell_centre(p: Pos) -> Vec2 {
    to_screen(p.x as f32, p.y as f32)
}

/// The centre of a cell on the ground plane.
pub fn ground_centre(p: Pos) -> Vec2 {
    Vec2::new((p.y as f32 + 0.5) * GROUND, (p.x as f32 + 0.5) * GROUND)
}

/// The ground plane's size: lanes across, depth along.
pub fn ground_size() -> Vec2 {
    Vec2::new(BOARD_LANES as f32 * GROUND, BOARD_DEPTH as f32 * GROUND)
}

/// Where a point on the ground is drawn.
pub fn ground_to_screen(g: Vec2) -> Vec2 {
    to_screen(g.y / GROUND - 0.5, g.x / GROUND - 0.5)
}

/// The cell under a screen position, if it is on the board.
pub fn cell_at(s: Vec2) -> Option<Pos> {
    let sum = s.x / HALF_W - 1.0; // depth + lane
    let diff = (s.y - HEADROOM) / HALF_H - BOARD_DEPTH as f32; // lane - depth
    let depth = ((sum - diff) / 2.0).round() as i32;
    let lane = ((sum + diff) / 2.0).round() as i32;
    ((0..BOARD_DEPTH).contains(&depth) && (0..BOARD_LANES).contains(&lane))
        .then(|| Pos::new(depth, lane))
}

/// Stacking order for something standing at screen height `y`: nearer the
/// bottom of the screen is nearer the viewer, so it draws on top.
pub fn depth_z(y: f32) -> i32 {
    10 + (y / 4.0) as i32
}

/// One of the four diagonal directions a unit can face on the diamond.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Facing {
    NorthEast,
    NorthWest,
    SouthEast,
    SouthWest,
}

impl Facing {
    /// Facing up the screen (we see the unit's back).
    pub fn is_north(self) -> bool {
        matches!(self, Facing::NorthEast | Facing::NorthWest)
    }

    pub fn is_east(self) -> bool {
        matches!(self, Facing::NorthEast | Facing::SouthEast)
    }

    /// The facing nearest a direction on the ground plane (x along lanes, y
    /// along depth). On screen, more depth is up and right and more lane is
    /// down and right, so "east" is their sum and "north" is depth beating lane.
    pub fn from_ground(dir: Vec2) -> Facing {
        match (dir.y > dir.x, dir.x + dir.y >= 0.0) {
            (true, true) => Facing::NorthEast,
            (true, false) => Facing::NorthWest,
            (false, true) => Facing::SouthEast,
            (false, false) => Facing::SouthWest,
        }
    }
}

/// A node spanning the whole field that turns pointer positions into cells:
/// hovering, clicking and dropping on your half. Units stand on top of it but
/// let the pointer through.
#[derive(Component)]
pub struct FieldPicker;

/// The cell of your half under the pointer, while planning.
#[derive(Resource, Default)]
pub struct FieldHover(pub Option<Pos>);

pub struct FieldPlugin;

impl Plugin for FieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FieldHover>()
            .add_observer(on_move)
            .add_observer(on_out)
            .add_observer(on_click)
            .add_observer(on_drop);
    }
}

/// The cell of your half at a picking hit. UI hits are reported relative to
/// the node: (-0.5, -0.5) is its top-left corner and (0.5, 0.5) bottom-right.
fn own_cell(hit: Option<Vec3>) -> Option<Pos> {
    let n = hit?.truncate();
    let local = (n + Vec2::splat(0.5)) * Vec2::new(FIELD_W, FIELD_H);
    cell_at(local).filter(|p| p.x < SIDE_DEPTH)
}

fn on_move(
    ev: On<PointerMove>,
    pickers: Query<(), With<FieldPicker>>,
    mut hover: ResMut<FieldHover>,
) {
    if pickers.contains(ev.entity) {
        let cell = own_cell(ev.hit.position);
        if hover.0 != cell {
            hover.0 = cell;
        }
    }
}

fn on_out(
    ev: On<PointerOut>,
    pickers: Query<(), With<FieldPicker>>,
    mut hover: ResMut<FieldHover>,
) {
    if pickers.contains(ev.entity) {
        hover.0 = None;
    }
}

fn on_click(
    ev: On<PointerClick>,
    pickers: Query<(), With<FieldPicker>>,
    mut queue: ResMut<ActionQueue>,
) {
    if pickers.contains(ev.entity)
        && let Some(p) = own_cell(ev.hit.position)
    {
        queue.0.push(UiAction::ClickCell(p));
    }
}

fn on_drop(
    ev: On<PointerDragDrop>,
    pickers: Query<(), With<FieldPicker>>,
    sources: Query<&DragSource>,
    mut queue: ResMut<ActionQueue>,
) {
    if pickers.contains(ev.entity)
        && let (Ok(source), Some(p)) = (sources.get(ev.dropped), own_cell(ev.hit.position))
    {
        let action = UiAction::Drop(*source, DropTarget::Field(p));
        if !queue.0.contains(&action) {
            queue.0.push(action);
        }
    }
}

/// The board image (planning shows grid lines and dims the enemy half).
pub fn spawn_board(field: &mut ChildSpawnerCommands, planning: bool) {
    let image = if planning { kit::img::BOARD_PLANNING } else { kit::img::BOARD_BATTLE };
    field.spawn((
        ImageNode::new(image),
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(HEADROOM),
            width: px(FIELD_W),
            height: px(BOARD_IMG_H),
            ..default()
        },
        Pickable::IGNORE,
    ));
}

/// The picker over the whole field; spawn it before the units so they sit above.
pub fn spawn_picker(field: &mut ChildSpawnerCommands) {
    field.spawn((
        FieldPicker,
        Node {
            position_type: PositionType::Absolute,
            left: px(0),
            top: px(0),
            width: px(FIELD_W),
            height: px(FIELD_H),
            ..default()
        },
    ));
}

/// An absolutely placed node `w` x `h` whose bottom centre is at `feet`.
pub fn standing(feet: Vec2, w: f32, h: f32) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: px(feet.x - w / 2.0),
        top: px(feet.y - h),
        width: px(w),
        height: px(h),
        ..default()
    }
}

/// A one-tile overlay node centred on a cell.
pub fn tile_node(p: Pos) -> Node {
    let c = cell_centre(p);
    Node {
        position_type: PositionType::Absolute,
        left: px(c.x - HALF_W),
        top: px(c.y - HALF_H),
        width: px(2.0 * HALF_W),
        height: px(2.0 * HALF_H),
        ..default()
    }
}

/// An arrow just off the board's top-right edge pointing at the enemy
/// (north-east on screen, along the depth axis).
pub fn arrow_node() -> (Node, UiTransform) {
    let edge = (to_screen(BOARD_DEPTH as f32 - 0.5, -0.5)
        + to_screen(BOARD_DEPTH as f32 - 0.5, BOARD_LANES as f32 - 0.5))
        / 2.0;
    let at = edge + Vec2::new(22.0, -40.0);
    let angle = (HALF_H / HALF_W).atan().to_degrees(); // the depth axis, above horizontal
    (
        Node {
            position_type: PositionType::Absolute,
            left: px(at.x - 12.0),
            top: px(at.y - 12.0),
            width: px(24),
            height: px(24),
            ..default()
        },
        UiTransform { rotation: Rot2::degrees(90.0 - angle), ..default() },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_board_fills_the_field_exactly() {
        // Left, top, right and bottom corners of the diamond.
        let left = to_screen(-0.5, -0.5);
        let top = to_screen(BOARD_DEPTH as f32 - 0.5, -0.5);
        let right = to_screen(BOARD_DEPTH as f32 - 0.5, BOARD_LANES as f32 - 0.5);
        let bottom = to_screen(-0.5, BOARD_LANES as f32 - 0.5);
        assert_eq!(left.x, 0.0);
        assert_eq!(top.y, HEADROOM);
        assert_eq!(right.x, FIELD_W);
        assert_eq!(bottom.y, HEADROOM + 2.0 * HALF_H * (BOARD_DEPTH + BOARD_LANES) as f32 / 2.0);
        assert!(bottom.y < FIELD_H);
    }

    #[test]
    fn every_cell_centre_picks_its_own_cell() {
        for x in 0..BOARD_DEPTH {
            for y in 0..BOARD_LANES {
                let p = Pos::new(x, y);
                assert_eq!(cell_at(cell_centre(p)), Some(p));
                // Points well inside the diamond pick it too.
                let c = cell_centre(p);
                for d in [
                    Vec2::new(30.0, 0.0),
                    Vec2::new(-30.0, 0.0),
                    Vec2::new(0.0, 15.0),
                    Vec2::new(0.0, -15.0),
                ] {
                    assert_eq!(cell_at(c + d), Some(p), "{p:?} + {d:?}");
                }
            }
        }
    }

    #[test]
    fn off_the_board_picks_nothing() {
        assert_eq!(cell_at(Vec2::new(5.0, HEADROOM + 5.0)), None);
        assert_eq!(cell_at(Vec2::new(FIELD_W - 5.0, FIELD_H - 5.0)), None);
    }

    #[test]
    fn your_half_is_bottom_left_and_theirs_top_right() {
        let mine = cell_centre(Pos::new(0, 3));
        let theirs = cell_centre(Pos::new(7, 3));
        assert!(theirs.x > mine.x && theirs.y < mine.y);
    }

    #[test]
    fn ground_and_cells_agree() {
        for x in 0..BOARD_DEPTH {
            for y in 0..BOARD_LANES {
                let p = Pos::new(x, y);
                let s = ground_to_screen(ground_centre(p));
                assert!(s.distance(cell_centre(p)) < 0.01);
            }
        }
    }

    #[test]
    fn facings_follow_the_axes() {
        assert_eq!(Facing::from_ground(Vec2::new(0.0, 1.0)), Facing::NorthEast); // towards the enemy
        assert_eq!(Facing::from_ground(Vec2::new(0.0, -1.0)), Facing::SouthWest);
        assert_eq!(Facing::from_ground(Vec2::new(1.0, 0.0)), Facing::SouthEast);
        assert_eq!(Facing::from_ground(Vec2::new(-1.0, 0.0)), Facing::NorthWest);
    }
}

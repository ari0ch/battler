//! The fantasy UI kit: small pixel-art images (9-slice frames, buttons, card
//! tags, bars, icons, ground) and the Pixelify Sans font, all embedded in the
//! binary.
//!
//! Each asset has a **constant handle** made with `uuid_handle!`, and the
//! plugin inserts the decoded asset under that id at startup. Because the
//! handles are `const`, any builder function can name them directly (for
//! example `kit::img::PANEL_WOOD`) without needing a resource.
//!
//! The art is drawn at 1x. 9-slice corners are drawn at [`SCALE`]x (Bevy's
//! `max_corner_scale`), and tiled textures repeat every [`SCALE`]x their size,
//! so the pixels stay crisp and still follow `UiScale`.
//!
//! Regenerate the art with `tools/art/kit.py`.

use bevy::asset::{RenderAssetUsages, uuid_handle};
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;

/// Pixel scale of the kit on screen.
pub const SCALE: f32 = 2.0;

/// Pixelify Sans (SIL Open Font License, `assets/fonts/OFL.txt`).
pub const FONT: Handle<Font> = uuid_handle!("8a0d50f0-0efc-529f-9b2e-51f08da24f4f");

/// One constant handle per kit image, named after its file in `assets/ui`.
pub mod img {
    use bevy::asset::uuid_handle;
    use bevy::prelude::*;

    pub const BANNER: Handle<Image> = uuid_handle!("20d9a4d3-35aa-5883-a7b0-e75f779afd70");
    pub const BAR_FRAME: Handle<Image> = uuid_handle!("4bed89a1-a5dc-5ab7-bd04-dce63d10bc5f");
    pub const BOARD_BATTLE: Handle<Image> = uuid_handle!("57e83636-2469-4637-b683-3ef909746eac");
    pub const BOARD_PLANNING: Handle<Image> = uuid_handle!("d7abc897-a61f-42bb-aa95-f824dd914635");
    pub const TILE_HOVER: Handle<Image> = uuid_handle!("a31b6b1a-249f-4b26-8546-c06be90e365a");
    pub const TILE_REACH: Handle<Image> = uuid_handle!("ce679cc3-af24-4107-be90-b317b149c031");
    pub const BUTTON: Handle<Image> = uuid_handle!("3853b134-1b49-5e04-b31b-d6a8b5404081");
    pub const BUTTON_GOLD: Handle<Image> = uuid_handle!("eb435f3b-bceb-59a7-8686-82d272da59e2");
    pub const BUTTON_HOVER: Handle<Image> = uuid_handle!("e67d4ab9-9487-5d5a-8659-6222e5ec46df");
    pub const BUTTON_PRESSED: Handle<Image> = uuid_handle!("8b70ea42-60ec-522d-877b-22dcae146e8d");
    pub const CARD_ROW: Handle<Image> = uuid_handle!("e0085a32-0c03-5160-86b4-755f36f286a8");
    pub const FIELD_GRASS: Handle<Image> = uuid_handle!("8e9d1a2d-83b4-57ae-8f24-8f6e2e5992f1");
    pub const FILL_ENEMY: Handle<Image> = uuid_handle!("9c07337f-b54f-56f7-b8cf-2883da53c2e4");
    pub const FILL_HP: Handle<Image> = uuid_handle!("43ee81da-93e0-5368-9555-a50f0f1e29f4");
    pub const FILL_HP_LOW: Handle<Image> = uuid_handle!("db17031e-1371-54d8-b897-7f0fc7e74131");
    pub const FILL_TIME: Handle<Image> = uuid_handle!("9d332903-e851-5276-8855-5a6c9d7521bf");
    pub const FILL_XP: Handle<Image> = uuid_handle!("4e2af90c-4abe-5a51-8c4d-621cb263e441");
    pub const FRAME_COMMON: Handle<Image> = uuid_handle!("1203bf70-6f8d-53f0-b5b8-066533edbb82");
    pub const FRAME_ENEMY: Handle<Image> = uuid_handle!("161dcd84-4f8b-52a6-a050-437e99f5e7f1");
    pub const FRAME_EPIC: Handle<Image> = uuid_handle!("1410c6fa-bc61-58e8-a792-294142567038");
    pub const FRAME_RARE: Handle<Image> = uuid_handle!("5ea74b74-8623-583b-b34e-8b7ba71a61fb");
    pub const HEADER_WOOD: Handle<Image> = uuid_handle!("d539aa7f-ae6a-5d73-920c-c3a6a4eacff6");
    pub const ICON_ARROW_UP: Handle<Image> = uuid_handle!("18cfe5a8-cb6d-50de-91df-eb16bd520e21");
    pub const ICON_AXE: Handle<Image> = uuid_handle!("dd0e10e4-3ea3-57b5-9b35-3a7f8855cab5");
    pub const ICON_BOW: Handle<Image> = uuid_handle!("ae0e2b9f-2787-5470-984d-f8f9c3ec9669");
    pub const ICON_COIN: Handle<Image> = uuid_handle!("8a6f66bb-7a83-5f43-98a1-02edfa29dcd2");
    pub const ICON_CROWN: Handle<Image> = uuid_handle!("f04244a2-bbb5-522c-a0ed-c42f2f00ab01");
    pub const ICON_DAGGER: Handle<Image> = uuid_handle!("41ed51a4-a071-5ccf-bb6f-4906db5f0689");
    pub const ICON_HEART: Handle<Image> = uuid_handle!("1772b89f-dc38-524c-a3ad-ab18623dd1e9");
    pub const ICON_HELM: Handle<Image> = uuid_handle!("93b578fb-1696-5fc8-bfc5-8126324ae364");
    pub const ICON_HOLY: Handle<Image> = uuid_handle!("d356a645-1adb-5baf-8b42-082fdab81a0b");
    pub const ICON_HOURGLASS: Handle<Image> = uuid_handle!("e76c3e35-c5cf-5f33-98f1-1c0cdf14eb0b");
    pub const ICON_SKULL: Handle<Image> = uuid_handle!("09201c26-8039-5a82-b0c0-8476be91815d");
    pub const ICON_STAFF: Handle<Image> = uuid_handle!("2610334e-48cd-53c5-aeba-c6b174ab1343");
    pub const ICON_STAR: Handle<Image> = uuid_handle!("c53718bb-820b-544e-9b56-9f74b416b462");
    pub const ICON_SWORD: Handle<Image> = uuid_handle!("ff542504-dd23-539c-bee2-508fbf7440d4");
    pub const INSET: Handle<Image> = uuid_handle!("6b4565d3-7135-5024-9f22-3708702540bd");
    pub const PANEL_PARCHMENT: Handle<Image> = uuid_handle!("e2d2a1cb-f780-5154-8a0b-0fe0db319d6a");
    pub const PANEL_WOOD: Handle<Image> = uuid_handle!("1c3899f7-dc34-55f0-b831-14ca27dc27ae");
    pub const TAG_ACTION: Handle<Image> = uuid_handle!("a58399ab-6b81-5c32-a5a1-a52cd466bb99");
    pub const TAG_CONDITION: Handle<Image> = uuid_handle!("334b89cc-1b8f-5b65-b2bb-2340294c399a");
    pub const TAG_EMPTY: Handle<Image> = uuid_handle!("3912d0f6-ccb8-58e6-b10f-78b3ea7c164a");
    pub const TAG_SIGNATURE: Handle<Image> = uuid_handle!("29e7f999-9996-57f8-9123-7b83d68f6dd2");
    pub const WAX_SEAL: Handle<Image> = uuid_handle!("da0012aa-1c7a-56dd-9fd5-5d109965b928");
}

const IMAGES: &[(Handle<Image>, &[u8])] = &[
    (img::BANNER, include_bytes!("../assets/ui/banner.png")),
    (img::BAR_FRAME, include_bytes!("../assets/ui/bar_frame.png")),
    (img::BOARD_BATTLE, include_bytes!("../assets/ui/board_battle.png")),
    (img::BOARD_PLANNING, include_bytes!("../assets/ui/board_planning.png")),
    (img::TILE_HOVER, include_bytes!("../assets/ui/tile_hover.png")),
    (img::TILE_REACH, include_bytes!("../assets/ui/tile_reach.png")),
    (img::BUTTON, include_bytes!("../assets/ui/button.png")),
    (img::BUTTON_GOLD, include_bytes!("../assets/ui/button_gold.png")),
    (img::BUTTON_HOVER, include_bytes!("../assets/ui/button_hover.png")),
    (img::BUTTON_PRESSED, include_bytes!("../assets/ui/button_pressed.png")),
    (img::CARD_ROW, include_bytes!("../assets/ui/card_row.png")),
    (img::FIELD_GRASS, include_bytes!("../assets/ui/field_grass.png")),
    (img::FILL_ENEMY, include_bytes!("../assets/ui/fill_enemy.png")),
    (img::FILL_HP, include_bytes!("../assets/ui/fill_hp.png")),
    (img::FILL_HP_LOW, include_bytes!("../assets/ui/fill_hp_low.png")),
    (img::FILL_TIME, include_bytes!("../assets/ui/fill_time.png")),
    (img::FILL_XP, include_bytes!("../assets/ui/fill_xp.png")),
    (img::FRAME_COMMON, include_bytes!("../assets/ui/frame_common.png")),
    (img::FRAME_ENEMY, include_bytes!("../assets/ui/frame_enemy.png")),
    (img::FRAME_EPIC, include_bytes!("../assets/ui/frame_epic.png")),
    (img::FRAME_RARE, include_bytes!("../assets/ui/frame_rare.png")),
    (img::HEADER_WOOD, include_bytes!("../assets/ui/header_wood.png")),
    (img::ICON_ARROW_UP, include_bytes!("../assets/ui/icon_arrow_up.png")),
    (img::ICON_AXE, include_bytes!("../assets/ui/icon_axe.png")),
    (img::ICON_BOW, include_bytes!("../assets/ui/icon_bow.png")),
    (img::ICON_COIN, include_bytes!("../assets/ui/icon_coin.png")),
    (img::ICON_CROWN, include_bytes!("../assets/ui/icon_crown.png")),
    (img::ICON_DAGGER, include_bytes!("../assets/ui/icon_dagger.png")),
    (img::ICON_HEART, include_bytes!("../assets/ui/icon_heart.png")),
    (img::ICON_HELM, include_bytes!("../assets/ui/icon_helm.png")),
    (img::ICON_HOLY, include_bytes!("../assets/ui/icon_holy.png")),
    (img::ICON_HOURGLASS, include_bytes!("../assets/ui/icon_hourglass.png")),
    (img::ICON_SKULL, include_bytes!("../assets/ui/icon_skull.png")),
    (img::ICON_STAFF, include_bytes!("../assets/ui/icon_staff.png")),
    (img::ICON_STAR, include_bytes!("../assets/ui/icon_star.png")),
    (img::ICON_SWORD, include_bytes!("../assets/ui/icon_sword.png")),
    (img::INSET, include_bytes!("../assets/ui/inset.png")),
    (img::PANEL_PARCHMENT, include_bytes!("../assets/ui/panel_parchment.png")),
    (img::PANEL_WOOD, include_bytes!("../assets/ui/panel_wood.png")),
    (img::TAG_ACTION, include_bytes!("../assets/ui/tag_action.png")),
    (img::TAG_CONDITION, include_bytes!("../assets/ui/tag_condition.png")),
    (img::TAG_EMPTY, include_bytes!("../assets/ui/tag_empty.png")),
    (img::TAG_SIGNATURE, include_bytes!("../assets/ui/tag_signature.png")),
    (img::WAX_SEAL, include_bytes!("../assets/ui/wax_seal.png")),
];

pub struct KitPlugin;

impl Plugin for KitPlugin {
    /// Runs while the app is being built, after `DefaultPlugins` registered
    /// `Assets<Image>` and `Assets<Font>`, so the assets exist before any UI.
    fn build(&self, app: &mut App) {
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        for (handle, bytes) in IMAGES {
            let image = Image::from_buffer(
                bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::nearest(),
                RenderAssetUsages::RENDER_WORLD,
            )
            .expect("kit images are valid PNGs");
            images.insert(handle, image).expect("constant handles are always valid");
        }
        let font = Font::from_bytes(include_bytes!("../assets/fonts/PixelifySans.ttf").to_vec());
        let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
        fonts.insert(&FONT, font).expect("constant handles are always valid");
    }
}

/// A 9-slice image: corners of `border` source pixels stay sharp at
/// [`SCALE`]x, edges and centre stretch. The node's own size sets the size.
///
/// It fills the node's whole border box. Bevy 0.20 draws an `ImageNode` in the
/// *content* box by default (inside the padding), which would shrink a frame
/// onto the very text its padding is meant to keep clear of it.
pub fn sliced(image: Handle<Image>, border: f32) -> ImageNode {
    ImageNode::new(image)
        .with_mode(NodeImageMode::Sliced(TextureSlicer {
            border: BorderRect::all(border),
            center_scale_mode: SliceScaleMode::Stretch,
            sides_scale_mode: SliceScaleMode::Stretch,
            max_corner_scale: SCALE,
        }))
        .with_visual_box(VisualBox::BorderBox)
}

/// A texture repeated across the node's whole border box at [`SCALE`]x.
pub fn tiled(image: Handle<Image>, tile_x: bool, tile_y: bool) -> ImageNode {
    ImageNode::new(image)
        .with_mode(NodeImageMode::Tiled { tile_x, tile_y, stretch_value: SCALE })
        .with_visual_box(VisualBox::BorderBox)
}

/// A square icon of `size` UI pixels that ignores the pointer.
pub fn icon(image: Handle<Image>, size: f32) -> impl Bundle {
    (
        ImageNode::new(image),
        Node { width: px(size), height: px(size), flex_shrink: 0.0, ..default() },
        Pickable::IGNORE,
    )
}

/// A dark wooden panel with gold corners, for light text.
pub fn wood_panel(node: Node) -> impl Bundle {
    (Node { padding: UiRect::all(px(16)), ..node }, sliced(img::PANEL_WOOD, 10.0))
}

/// A parchment panel in a wooden frame, for dark ink text.
pub fn parchment_panel(node: Node) -> impl Bundle {
    (Node { padding: UiRect::all(px(16)), ..node }, sliced(img::PANEL_PARCHMENT, 10.0))
}

/// A carved slot (bench slots, empty sockets).
pub fn inset() -> ImageNode {
    sliced(img::INSET, 4.0)
}

/// The icon for a class.
pub fn class_icon(class: battler_sim::Class) -> Handle<Image> {
    use battler_sim::Class;
    match class {
        Class::Knight => img::ICON_HELM,
        Class::Ranger => img::ICON_BOW,
        Class::Mage => img::ICON_STAFF,
        Class::Cleric => img::ICON_HOLY,
        Class::Rogue => img::ICON_DAGGER,
        Class::Berserker => img::ICON_AXE,
    }
}

/// A health/XP/timer bar: a dark sliced frame holding a tiled fill, `filled`
/// percent wide. Add `marker` to the fill to update it later.
pub fn spawn_bar(
    parent: &mut ChildSpawnerCommands,
    fill: Handle<Image>,
    filled: f32,
    width: Val,
    height: f32,
    marker: impl Bundle,
) {
    parent
        .spawn((
            Node {
                width,
                height: px(height),
                flex_shrink: 0.0,
                padding: UiRect::all(px(3)),
                ..default()
            },
            sliced(img::BAR_FRAME, 3.0),
        ))
        .with_child((
            Node { width: percent(filled.clamp(0.0, 100.0)), height: percent(100), ..default() },
            tiled(fill, true, false),
            marker,
        ));
}

/// The unit frame for a rarity.
pub fn rarity_frame(rarity: battler_sim::Rarity) -> Handle<Image> {
    use battler_sim::Rarity;
    match rarity {
        Rarity::Common => img::FRAME_COMMON,
        Rarity::Rare => img::FRAME_RARE,
        Rarity::Epic => img::FRAME_EPIC,
    }
}

//! Character sprites: 32x32 pixel art facing right, drawn at whole-number
//! scales with nearest-neighbour sampling so the pixels stay crisp.
//!
//! The PNGs are embedded in the binary with `include_bytes!` rather than loaded
//! through the `AssetServer`. They are tiny (about 4 KB each), and embedding
//! means the web build needs no asset folder or URL handling under the GitHub
//! Pages `/battler/` prefix.

use battler_sim::{Character, UnitKind};
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;

/// The sprite's size in source pixels.
pub const SPRITE_PX: f32 = 32.0;

/// One image handle per character, indexed by `Character::index()`.
#[derive(Resource)]
pub struct Sprites {
    characters: Vec<Handle<Image>>,
}

impl Sprites {
    pub fn character(&self, c: Character) -> Handle<Image> {
        self.characters[c.index()].clone()
    }

    /// The sprite for a battle unit; monsters have none yet.
    pub fn unit(&self, kind: UnitKind) -> Option<Handle<Image>> {
        kind.character().map(|c| self.character(c))
    }
}

/// `FromWorld` lets `init_resource` build this from other resources: here it
/// decodes every PNG and adds it to `Assets<Image>`, which `DefaultPlugins`
/// has already registered.
impl FromWorld for Sprites {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        let characters = Character::ALL
            .into_iter()
            .map(|c| {
                let image = Image::from_buffer(
                    png_bytes(c),
                    ImageType::Extension("png"),
                    CompressedImageFormats::NONE,
                    true,
                    ImageSampler::nearest(),
                    RenderAssetUsages::RENDER_WORLD,
                )
                .unwrap_or_else(|e| panic!("sprite for {c:?} is not a valid PNG: {e}"));
                images.add(image)
            })
            .collect();
        Self { characters }
    }
}

/// The embedded PNG for each character. The match is exhaustive, so adding a
/// character without art is a compile error rather than a missing sprite.
fn png_bytes(c: Character) -> &'static [u8] {
    use Character::*;
    match c {
        BramGatewarden => include_bytes!("../assets/sprites/bram_gatewarden.png"),
        SirAldric => include_bytes!("../assets/sprites/sir_aldric.png"),
        Ysolde => include_bytes!("../assets/sprites/ysolde_the_unbroken.png"),
        PipQuickfeather => include_bytes!("../assets/sprites/pip_quickfeather.png"),
        Kestrel => include_bytes!("../assets/sprites/kestrel.png"),
        Elowen => include_bytes!("../assets/sprites/elowen_starshot.png"),
        Tobble => include_bytes!("../assets/sprites/tobble_the_apprentice.png"),
        Mirelle => include_bytes!("../assets/sprites/mirelle_frostweaver.png"),
        Vorn => include_bytes!("../assets/sprites/archmage_vorn.png"),
        SisterAgnes => include_bytes!("../assets/sprites/sister_agnes.png"),
        BrotherCalloway => include_bytes!("../assets/sprites/brother_calloway.png"),
        Seraphine => include_bytes!("../assets/sprites/seraphine_dawnbringer.png"),
        Nettle => include_bytes!("../assets/sprites/nettle.png"),
        Vex => include_bytes!("../assets/sprites/vex.png"),
        GreyWidow => include_bytes!("../assets/sprites/the_grey_widow.png"),
        Grub => include_bytes!("../assets/sprites/grub_ironjaw.png"),
        Hilda => include_bytes!("../assets/sprites/hilda_bloodaxe.png"),
        Korgath => include_bytes!("../assets/sprites/korgath_the_undying.png"),
    }
}

/// A sprite node `scale` times its source size, ignoring the pointer so the
/// tile underneath still gets clicks and drags.
pub fn sprite_node(image: Handle<Image>, scale: f32, flip: bool) -> impl Bundle {
    let size = SPRITE_PX * scale;
    (
        ImageNode { image, flip_x: flip, ..default() },
        Node { width: px(size), height: px(size), flex_shrink: 0.0, ..default() },
        Pickable::IGNORE,
    )
}

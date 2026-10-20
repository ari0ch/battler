//! Character and monster sprites: 32x32 pixel art, drawn at whole-number scales
//! with nearest-neighbour sampling so the pixels stay crisp.
//!
//! The roster art faces right in a front three-quarter view, which on the
//! isometric field is south-east (towards the camera); mirrored, it is
//! south-west. Characters also have back art (`assets/sprites/back`) for
//! north-east, mirrored for north-west. Monsters (always the enemy, so mostly
//! facing you) show their front from every side for now.
//!
//! The PNGs are embedded in the binary with `include_bytes!` rather than loaded
//! through the `AssetServer`. They are tiny (about 4 KB each), and embedding
//! means the web build needs no asset folder or URL handling under the GitHub
//! Pages `/battler/` prefix.

use battler_sim::{Character, Monster, UnitKind};
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;

use crate::field::Facing;

/// The sprite's size in source pixels.
pub const SPRITE_PX: f32 = 32.0;

/// One image handle per character (indexed by `Character::index()`) and per
/// monster (in `Monster::ALL` order).
#[derive(Resource)]
pub struct Sprites {
    characters: Vec<Handle<Image>>,
    monsters: Vec<Handle<Image>>,
    /// Back views (facing north-east); same indexing as `characters`.
    character_backs: Vec<Handle<Image>>,
}

impl Sprites {
    pub fn character(&self, c: Character) -> Handle<Image> {
        self.characters[c.index()].clone()
    }

    pub fn monster(&self, m: Monster) -> Handle<Image> {
        let i = Monster::ALL.iter().position(|&x| x == m).expect("every monster is in ALL");
        self.monsters[i].clone()
    }

    /// The sprite for any battle unit.
    pub fn unit(&self, kind: UnitKind) -> Handle<Image> {
        match kind {
            UnitKind::Character(c) => self.character(c),
            UnitKind::Monster(m) => self.monster(m),
        }
    }

    /// The image and mirroring that show `kind` facing `facing`: the back art
    /// for the two northern facings when there is some, else the front.
    pub fn facing(&self, kind: UnitKind, facing: Facing) -> (Handle<Image>, bool) {
        let back = match kind {
            UnitKind::Character(c) if facing.is_north() => {
                Some(self.character_backs[c.index()].clone())
            }
            _ => None,
        };
        // Both views are drawn facing east; west is the mirror image.
        (back.unwrap_or_else(|| self.unit(kind)), !facing.is_east())
    }
}

/// How many screen pixels per sprite pixel a unit is drawn at in battle:
/// bosses loom at 3x, everyone else 2x.
pub fn battle_scale(kind: UnitKind) -> f32 {
    match kind {
        UnitKind::Monster(Monster::OgreChieftain | Monster::FrostWyrm | Monster::Lich) => 3.0,
        _ => 2.0,
    }
}

/// `FromWorld` lets `init_resource` build this from other resources: here it
/// decodes every PNG and adds it to `Assets<Image>`, which `DefaultPlugins`
/// has already registered.
impl FromWorld for Sprites {
    fn from_world(world: &mut World) -> Self {
        let mut images = world.resource_mut::<Assets<Image>>();
        // A closure that decodes one embedded PNG and stores it, returning its handle.
        let mut load = |name: &str, bytes: &[u8]| {
            let image = Image::from_buffer(
                bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::nearest(),
                RenderAssetUsages::RENDER_WORLD,
            )
            .unwrap_or_else(|e| panic!("sprite for {name} is not a valid PNG: {e}"));
            images.add(image)
        };
        let characters =
            Character::ALL.into_iter().map(|c| load(c.name(), character_png(c))).collect();
        let monsters = Monster::ALL.into_iter().map(|m| load(m.name(), monster_png(m))).collect();
        let character_backs =
            Character::ALL.into_iter().map(|c| load(c.name(), character_back_png(c))).collect();
        Self { characters, monsters, character_backs }
    }
}

/// The embedded PNG for each character. The match is exhaustive, so adding a
/// character without art is a compile error rather than a missing sprite.
fn character_png(c: Character) -> &'static [u8] {
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

/// Each character seen from behind (facing north-east), for walking away
/// from the camera. Exhaustive like the fronts.
fn character_back_png(c: Character) -> &'static [u8] {
    use Character::*;
    match c {
        BramGatewarden => include_bytes!("../assets/sprites/back/bram_gatewarden.png"),
        SirAldric => include_bytes!("../assets/sprites/back/sir_aldric.png"),
        Ysolde => include_bytes!("../assets/sprites/back/ysolde_the_unbroken.png"),
        PipQuickfeather => include_bytes!("../assets/sprites/back/pip_quickfeather.png"),
        Kestrel => include_bytes!("../assets/sprites/back/kestrel.png"),
        Elowen => include_bytes!("../assets/sprites/back/elowen_starshot.png"),
        Tobble => include_bytes!("../assets/sprites/back/tobble_the_apprentice.png"),
        Mirelle => include_bytes!("../assets/sprites/back/mirelle_frostweaver.png"),
        Vorn => include_bytes!("../assets/sprites/back/archmage_vorn.png"),
        SisterAgnes => include_bytes!("../assets/sprites/back/sister_agnes.png"),
        BrotherCalloway => include_bytes!("../assets/sprites/back/brother_calloway.png"),
        Seraphine => include_bytes!("../assets/sprites/back/seraphine_dawnbringer.png"),
        Nettle => include_bytes!("../assets/sprites/back/nettle.png"),
        Vex => include_bytes!("../assets/sprites/back/vex.png"),
        GreyWidow => include_bytes!("../assets/sprites/back/the_grey_widow.png"),
        Grub => include_bytes!("../assets/sprites/back/grub_ironjaw.png"),
        Hilda => include_bytes!("../assets/sprites/back/hilda_bloodaxe.png"),
        Korgath => include_bytes!("../assets/sprites/back/korgath_the_undying.png"),
    }
}

fn monster_png(m: Monster) -> &'static [u8] {
    use Monster::*;
    match m {
        Slime => include_bytes!("../assets/sprites/monsters/slime.png"),
        Wolf => include_bytes!("../assets/sprites/monsters/wolf.png"),
        GoblinBrute => include_bytes!("../assets/sprites/monsters/goblin_brute.png"),
        GoblinArcher => include_bytes!("../assets/sprites/monsters/goblin_archer.png"),
        Skeleton => include_bytes!("../assets/sprites/monsters/skeleton.png"),
        OgreChieftain => include_bytes!("../assets/sprites/monsters/ogre_chieftain.png"),
        FrostWyrm => include_bytes!("../assets/sprites/monsters/frost_wyrm.png"),
        Lich => include_bytes!("../assets/sprites/monsters/lich.png"),
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

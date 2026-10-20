//! Items: equipment that monsters and bosses drop, two per unit.
//!
//! Items are plain bonuses: stats, or a simple combat effect. They are applied
//! when a unit spawns into a battle (stats) or at fixed points in combat
//! (`exec.rs`), always with integer percentages like the rest of the sim.
//!
//! | Item            | Rarity | Effect                                                   |
//! |-----------------|--------|----------------------------------------------------------|
//! | Iron Sword      | Common | +15% attack                                              |
//! | Leather Jerkin  | Common | +20% max HP                                              |
//! | Lucky Coin      | Common | +10% crit chance                                         |
//! | Healing Draught | Common | first time below 40% HP, heals 30% of max HP             |
//! | Whetstone       | Common | first hit each battle deals double damage                |
//! | Scout's Map     | Common | starts one row further forward (if free, own half)       |
//! | Spyglass        | Rare   | +1 range                                                 |
//! | Vampire Fang    | Rare   | heals 20% of the damage it deals                         |
//! | Spiked Buckler  | Rare   | reflects 20% of melee damage taken                       |
//! | Hourglass       | Rare   | signature ability cooldowns 1 tick shorter (min 1)       |
//! | Phoenix Feather | Epic   | revives once at 40% HP                                   |
//! | Banner          | Epic   | bears a class; the unit also counts as it for synergies  |
//! | Twin Blades     | Epic   | basic attacks hit twice at 60% each                      |
//! | Aegis           | Epic   | can't be harmed for the first 2 ticks; -15% damage after |

use crate::{Class, Rarity, Rng};

/// Item slots on every unit.
pub const ITEM_SLOTS: usize = 2;

/// The items a unit carries: up to [`ITEM_SLOTS`].
pub type ItemSlots = [Option<Item>; ITEM_SLOTS];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    IronSword,
    LeatherJerkin,
    LuckyCoin,
    HealingDraught,
    Whetstone,
    ScoutsMap,
    Spyglass,
    VampireFang,
    SpikedBuckler,
    Hourglass,
    PhoenixFeather,
    /// The unit also counts as this class for synergies.
    Banner(Class),
    TwinBlades,
    Aegis,
}

impl Item {
    pub const COMMON: [Item; 6] = [
        Item::IronSword,
        Item::LeatherJerkin,
        Item::LuckyCoin,
        Item::HealingDraught,
        Item::Whetstone,
        Item::ScoutsMap,
    ];
    pub const RARE: [Item; 4] =
        [Item::Spyglass, Item::VampireFang, Item::SpikedBuckler, Item::Hourglass];
    /// Banners are drawn with a random class (see [`Item::roll`]).
    pub const EPIC: [Item; 4] =
        [Item::PhoenixFeather, Item::Banner(Class::Knight), Item::TwinBlades, Item::Aegis];

    pub fn rarity(self) -> Rarity {
        use Item::*;
        match self {
            IronSword | LeatherJerkin | LuckyCoin | HealingDraught | Whetstone | ScoutsMap => {
                Rarity::Common
            }
            Spyglass | VampireFang | SpikedBuckler | Hourglass => Rarity::Rare,
            PhoenixFeather | Banner(_) | TwinBlades | Aegis => Rarity::Epic,
        }
    }

    pub fn name(self) -> String {
        use Item::*;
        match self {
            IronSword => "Iron Sword".into(),
            LeatherJerkin => "Leather Jerkin".into(),
            LuckyCoin => "Lucky Coin".into(),
            HealingDraught => "Healing Draught".into(),
            Whetstone => "Whetstone".into(),
            ScoutsMap => "Scout's Map".into(),
            Spyglass => "Spyglass".into(),
            VampireFang => "Vampire Fang".into(),
            SpikedBuckler => "Spiked Buckler".into(),
            Hourglass => "Hourglass".into(),
            PhoenixFeather => "Phoenix Feather".into(),
            Banner(class) => format!("{} Banner", class.name()),
            TwinBlades => "Twin Blades".into(),
            Aegis => "Aegis".into(),
        }
    }

    /// What the item does, in one line.
    pub fn description(self) -> String {
        use Item::*;
        match self {
            IronSword => "+15% attack".into(),
            LeatherJerkin => "+20% max HP".into(),
            LuckyCoin => "+10% crit chance".into(),
            HealingDraught => "The first time below 40% HP, heals 30% of max HP".into(),
            Whetstone => "The first hit each battle deals double damage".into(),
            ScoutsMap => "Starts the battle one row further forward, if free".into(),
            Spyglass => "+1 range".into(),
            VampireFang => "Heals 20% of the damage it deals".into(),
            SpikedBuckler => "Reflects 20% of melee damage taken".into(),
            Hourglass => "Signature ability cooldowns 1 tick shorter".into(),
            PhoenixFeather => "Revives once at 40% HP".into(),
            Banner(class) => {
                format!("Also counts as a {} for synergies", class.name())
            }
            TwinBlades => "Basic attacks hit twice, at 60% each".into(),
            Aegis => "Can't be harmed for the first 2 ticks, then takes 15% less damage".into(),
        }
    }

    /// A random item of a rarity. Banners get a random class.
    pub fn roll(rarity: Rarity, rng: &mut Rng) -> Item {
        let pool: &[Item] = match rarity {
            Rarity::Common => &Item::COMMON,
            Rarity::Rare => &Item::RARE,
            Rarity::Epic => &Item::EPIC,
        };
        match pool[rng.below(pool.len() as u32) as usize] {
            Item::Banner(_) => {
                Item::Banner(Class::ALL[rng.below(Class::ALL.len() as u32) as usize])
            }
            item => item,
        }
    }
}

/// True if any of `slots` holds `item`.
pub fn has(slots: &ItemSlots, item: Item) -> bool {
    slots.contains(&Some(item))
}

/// The class a Banner in `slots` adds, if any.
pub fn banner_class(slots: &ItemSlots) -> Option<Class> {
    slots.iter().flatten().find_map(|i| match i {
        Item::Banner(c) => Some(*c),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_pool_item_has_its_rarity() {
        for (pool, rarity) in [
            (&Item::COMMON[..], Rarity::Common),
            (&Item::RARE, Rarity::Rare),
            (&Item::EPIC, Rarity::Epic),
        ] {
            for item in pool {
                assert_eq!(item.rarity(), rarity, "{item:?}");
            }
        }
    }

    #[test]
    fn rolls_are_deterministic_and_of_the_asked_rarity() {
        let (mut a, mut b) = (Rng::new(9), Rng::new(9));
        for _ in 0..50 {
            for r in Rarity::ALL {
                let x = Item::roll(r, &mut a);
                assert_eq!(x, Item::roll(r, &mut b));
                assert_eq!(x.rarity(), r);
            }
        }
    }

    #[test]
    fn banners_come_in_every_class() {
        let mut rng = Rng::new(3);
        let mut seen = Vec::new();
        for _ in 0..500 {
            if let Item::Banner(c) = Item::roll(Rarity::Epic, &mut rng)
                && !seen.contains(&c)
            {
                seen.push(c);
            }
        }
        assert_eq!(seen.len(), Class::ALL.len());
    }
}

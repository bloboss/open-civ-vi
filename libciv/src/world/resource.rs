use crate::YieldBundle;
pub use crate::ResourceCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinResource {
    // Bonus
    Wheat,
    Rice,
    Cattle,
    Sheep,
    Fish,
    Stone,
    Copper,
    Deer,
    Bananas,
    Crabs,
    // Luxury
    Wine,
    Silk,
    Spices,
    Incense,
    Cotton,
    Ivory,
    Sugar,
    Salt,
    Citrus,
    Cocoa,
    Coffee,
    Diamonds,
    Dyes,
    Furs,
    Gypsum,
    Jade,
    Marble,
    Mercury,
    Pearls,
    Silver,
    Tea,
    Tobacco,
    Truffles,
    Whales,
    // Strategic
    Horses,
    Iron,
    Coal,
    Oil,
    Aluminum,
    Niter,
    Uranium,
}

impl BuiltinResource {
    pub fn name(self) -> &'static str {
        match self {
            BuiltinResource::Wheat    => "Wheat",
            BuiltinResource::Rice     => "Rice",
            BuiltinResource::Cattle   => "Cattle",
            BuiltinResource::Sheep    => "Sheep",
            BuiltinResource::Fish     => "Fish",
            BuiltinResource::Stone    => "Stone",
            BuiltinResource::Copper   => "Copper",
            BuiltinResource::Deer     => "Deer",
            BuiltinResource::Bananas  => "Bananas",
            BuiltinResource::Crabs    => "Crabs",
            BuiltinResource::Wine     => "Wine",
            BuiltinResource::Silk     => "Silk",
            BuiltinResource::Spices   => "Spices",
            BuiltinResource::Incense  => "Incense",
            BuiltinResource::Cotton   => "Cotton",
            BuiltinResource::Ivory    => "Ivory",
            BuiltinResource::Sugar    => "Sugar",
            BuiltinResource::Salt     => "Salt",
            BuiltinResource::Citrus   => "Citrus",
            BuiltinResource::Cocoa    => "Cocoa",
            BuiltinResource::Coffee   => "Coffee",
            BuiltinResource::Diamonds => "Diamonds",
            BuiltinResource::Dyes     => "Dyes",
            BuiltinResource::Furs     => "Furs",
            BuiltinResource::Gypsum   => "Gypsum",
            BuiltinResource::Jade     => "Jade",
            BuiltinResource::Marble   => "Marble",
            BuiltinResource::Mercury  => "Mercury",
            BuiltinResource::Pearls   => "Pearls",
            BuiltinResource::Silver   => "Silver",
            BuiltinResource::Tea      => "Tea",
            BuiltinResource::Tobacco  => "Tobacco",
            BuiltinResource::Truffles => "Truffles",
            BuiltinResource::Whales   => "Whales",
            BuiltinResource::Horses   => "Horses",
            BuiltinResource::Iron     => "Iron",
            BuiltinResource::Coal     => "Coal",
            BuiltinResource::Oil      => "Oil",
            BuiltinResource::Aluminum => "Aluminum",
            BuiltinResource::Niter    => "Niter",
            BuiltinResource::Uranium  => "Uranium",
        }
    }

    pub fn category(self) -> ResourceCategory {
        match self {
            BuiltinResource::Wheat
            | BuiltinResource::Rice
            | BuiltinResource::Cattle
            | BuiltinResource::Sheep
            | BuiltinResource::Fish
            | BuiltinResource::Stone
            | BuiltinResource::Copper
            | BuiltinResource::Deer
            | BuiltinResource::Bananas
            | BuiltinResource::Crabs => ResourceCategory::Bonus,

            BuiltinResource::Wine
            | BuiltinResource::Silk
            | BuiltinResource::Spices
            | BuiltinResource::Incense
            | BuiltinResource::Cotton
            | BuiltinResource::Ivory
            | BuiltinResource::Sugar
            | BuiltinResource::Salt
            | BuiltinResource::Citrus
            | BuiltinResource::Cocoa
            | BuiltinResource::Coffee
            | BuiltinResource::Diamonds
            | BuiltinResource::Dyes
            | BuiltinResource::Furs
            | BuiltinResource::Gypsum
            | BuiltinResource::Jade
            | BuiltinResource::Marble
            | BuiltinResource::Mercury
            | BuiltinResource::Pearls
            | BuiltinResource::Silver
            | BuiltinResource::Tea
            | BuiltinResource::Tobacco
            | BuiltinResource::Truffles
            | BuiltinResource::Whales => ResourceCategory::Luxury,

            BuiltinResource::Horses
            | BuiltinResource::Iron
            | BuiltinResource::Coal
            | BuiltinResource::Oil
            | BuiltinResource::Aluminum
            | BuiltinResource::Niter
            | BuiltinResource::Uranium => ResourceCategory::Strategic,
        }
    }

    pub fn base_yields(self) -> YieldBundle {
        match self {
            BuiltinResource::Wheat    => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Rice     => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Cattle   => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Sheep    => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Fish     => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Stone    => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinResource::Copper   => YieldBundle::new().with(crate::YieldType::Gold, 2),
            BuiltinResource::Deer     => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinResource::Bananas  => YieldBundle::new().with(crate::YieldType::Food, 1),
            BuiltinResource::Crabs    => YieldBundle::new().with(crate::YieldType::Gold, 2),
            BuiltinResource::Wine     => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Silk     => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Spices   => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Incense  => YieldBundle::new()
                .with(crate::YieldType::Faith, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Cotton   => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Ivory    => YieldBundle::new()
                .with(crate::YieldType::Production, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Sugar    => YieldBundle::new()
                .with(crate::YieldType::Food, 2)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Salt     => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Citrus   => YieldBundle::new().with(crate::YieldType::Food, 2),
            BuiltinResource::Cocoa    => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Coffee   => YieldBundle::new().with(crate::YieldType::Culture, 1),
            BuiltinResource::Diamonds => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Dyes     => YieldBundle::new().with(crate::YieldType::Faith, 1),
            BuiltinResource::Furs     => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Gold, 1),
            BuiltinResource::Gypsum   => YieldBundle::new()
                .with(crate::YieldType::Gold, 1)
                .with(crate::YieldType::Production, 1),
            BuiltinResource::Jade     => YieldBundle::new().with(crate::YieldType::Culture, 1),
            BuiltinResource::Marble   => YieldBundle::new().with(crate::YieldType::Culture, 1),
            BuiltinResource::Mercury  => YieldBundle::new().with(crate::YieldType::Science, 1),
            BuiltinResource::Pearls   => YieldBundle::new().with(crate::YieldType::Faith, 1),
            BuiltinResource::Silver   => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Tea      => YieldBundle::new().with(crate::YieldType::Science, 1),
            BuiltinResource::Tobacco  => YieldBundle::new().with(crate::YieldType::Faith, 1),
            BuiltinResource::Truffles => YieldBundle::new().with(crate::YieldType::Gold, 3),
            BuiltinResource::Whales   => YieldBundle::new()
                .with(crate::YieldType::Gold, 1)
                .with(crate::YieldType::Production, 1),
            BuiltinResource::Horses   => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinResource::Iron     => YieldBundle::new().with(crate::YieldType::Production, 1),
            BuiltinResource::Coal     => YieldBundle::new().with(crate::YieldType::Production, 2),
            BuiltinResource::Oil      => YieldBundle::new().with(crate::YieldType::Production, 3),
            BuiltinResource::Aluminum => YieldBundle::new().with(crate::YieldType::Science, 1),
            BuiltinResource::Niter    => YieldBundle::new()
                .with(crate::YieldType::Food, 1)
                .with(crate::YieldType::Production, 1),
            BuiltinResource::Uranium  => YieldBundle::new().with(crate::YieldType::Production, 2),
        }
    }

    /// Tech required to see/use this resource (None = always visible).
    pub fn reveal_tech(self) -> Option<&'static str> {
        match self {
            BuiltinResource::Horses   => Some("Animal Husbandry"),
            BuiltinResource::Iron     => Some("Bronze Working"),
            BuiltinResource::Coal     => Some("Industrialization"),
            BuiltinResource::Oil      => Some("Refining"),
            BuiltinResource::Aluminum => Some("Refining"),
            BuiltinResource::Niter    => Some("Military Engineering"),
            BuiltinResource::Uranium  => Some("Nuclear Fission"),
            _ => None,
        }
    }
}

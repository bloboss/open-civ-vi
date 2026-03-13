use crate::YieldBundle;
pub use crate::ResourceCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    // Luxury
    Wine,
    Silk,
    Spices,
    Incense,
    Cotton,
    Ivory,
    Sugar,
    Salt,
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
            BuiltinResource::Wine     => "Wine",
            BuiltinResource::Silk     => "Silk",
            BuiltinResource::Spices   => "Spices",
            BuiltinResource::Incense  => "Incense",
            BuiltinResource::Cotton   => "Cotton",
            BuiltinResource::Ivory    => "Ivory",
            BuiltinResource::Sugar    => "Sugar",
            BuiltinResource::Salt     => "Salt",
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
            | BuiltinResource::Deer => ResourceCategory::Bonus,

            BuiltinResource::Wine
            | BuiltinResource::Silk
            | BuiltinResource::Spices
            | BuiltinResource::Incense
            | BuiltinResource::Cotton
            | BuiltinResource::Ivory
            | BuiltinResource::Sugar
            | BuiltinResource::Salt => ResourceCategory::Luxury,

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

use crate::{CityId, CivId, GovernorId, PromotionId};

pub trait GovernorDef: std::fmt::Debug {
    fn id(&self) -> GovernorId;
    fn name(&self) -> &'static str;
    fn title(&self) -> &'static str;
    fn base_ability_description(&self) -> &'static str;
}

pub trait GovernorPromotion: std::fmt::Debug {
    fn id(&self) -> PromotionId;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn requires(&self) -> Vec<PromotionId>;
}

#[derive(Debug, Clone)]
pub struct Governor {
    pub id: GovernorId,
    pub def_name: &'static str,
    pub owner: CivId,
    pub assigned_city: Option<CityId>,
    pub promotions: Vec<PromotionId>,
    pub turns_to_establish: u32,
}

impl Governor {
    pub fn new(id: GovernorId, def_name: &'static str, owner: CivId) -> Self {
        Self {
            id,
            def_name,
            owner,
            assigned_city: None,
            promotions: Vec::new(),
            turns_to_establish: 3,
        }
    }

    pub fn is_established(&self) -> bool {
        self.turns_to_establish == 0
    }
}

// ---- Seven built-in governor definitions ----

// TODO: Move this to a new .rs file and load with macros
macro_rules! define_governor {
    ($name:ident, $title:literal, $ability:literal) => {
        #[derive(Debug, Clone, Copy, Default)]
        pub struct $name;

        impl GovernorDef for $name {
            fn id(&self) -> GovernorId {
                // Placeholder: in Phase 2, load from a registry
                GovernorId::from_ulid(::ulid::Ulid::nil())
            }
            fn name(&self) -> &'static str { stringify!($name) }
            fn title(&self) -> &'static str { $title }
            fn base_ability_description(&self) -> &'static str { $ability }
        }
    };
}

define_governor!(
    Liang,
    "The Surveyor",
    "Establishes City Districts 25% faster."
);
define_governor!(
    Magnus,
    "The Steward",
    "Citizens settling new cities don't reduce population."
);
define_governor!(
    Amani,
    "The Diplomat",
    "Can be sent to City-States to gain influence."
);
define_governor!(
    Victor,
    "The Castellan",
    "City Walls are built twice as fast."
);
define_governor!(
    Pingala,
    "The Educator",
    "+20% Science and Culture in the assigned city."
);
define_governor!(
    Reyna,
    "The Financier",
    "Collect taxes on all trade routes passing through the city."
);
define_governor!(
    Ibrahim,
    "The Grand Vizier",
    "Provides unique bonuses when assigned to a city-state."
);

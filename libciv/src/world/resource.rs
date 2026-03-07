use crate::YieldBundle;
pub use crate::ResourceCategory;

pub trait Resource: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn category(&self) -> ResourceCategory;
    fn base_yields(&self) -> YieldBundle;
    /// Tech required to see/use this resource (None = always visible).
    fn reveal_tech(&self) -> Option<&'static str> { None }
}

// ── Bonus resources ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Wheat;
impl Resource for Wheat {
    fn name(&self) -> &'static str { "Wheat" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Food, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Rice;
impl Resource for Rice {
    fn name(&self) -> &'static str { "Rice" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Food, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Cattle;
impl Resource for Cattle {
    fn name(&self) -> &'static str { "Cattle" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Food, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Sheep;
impl Resource for Sheep {
    fn name(&self) -> &'static str { "Sheep" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Food, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Fish;
impl Resource for Fish {
    fn name(&self) -> &'static str { "Fish" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Food, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Stone;
impl Resource for Stone {
    fn name(&self) -> &'static str { "Stone" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 1) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Copper;
impl Resource for Copper {
    fn name(&self) -> &'static str { "Copper" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Gold, 2) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Deer;
impl Resource for Deer {
    fn name(&self) -> &'static str { "Deer" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Bonus }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 1) }
}

// ── Luxury resources ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Wine;
impl Resource for Wine {
    fn name(&self) -> &'static str { "Wine" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Gold, 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Silk;
impl Resource for Silk {
    fn name(&self) -> &'static str { "Silk" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Gold, 3) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Spices;
impl Resource for Spices {
    fn name(&self) -> &'static str { "Spices" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Gold, 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Incense;
impl Resource for Incense {
    fn name(&self) -> &'static str { "Incense" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Faith, 1)
            .with(crate::YieldType::Gold, 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Cotton;
impl Resource for Cotton {
    fn name(&self) -> &'static str { "Cotton" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Gold, 3) }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Ivory;
impl Resource for Ivory {
    fn name(&self) -> &'static str { "Ivory" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Production, 1)
            .with(crate::YieldType::Gold, 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Sugar;
impl Resource for Sugar {
    fn name(&self) -> &'static str { "Sugar" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 2)
            .with(crate::YieldType::Gold, 1)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Salt;
impl Resource for Salt {
    fn name(&self) -> &'static str { "Salt" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Luxury }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Gold, 1)
    }
}

// ── Strategic resources ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Horses;
impl Resource for Horses {
    fn name(&self) -> &'static str { "Horses" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 1) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Animal Husbandry") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Iron;
impl Resource for Iron {
    fn name(&self) -> &'static str { "Iron" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 1) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Bronze Working") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Coal;
impl Resource for Coal {
    fn name(&self) -> &'static str { "Coal" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 2) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Industrialization") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Oil;
impl Resource for Oil {
    fn name(&self) -> &'static str { "Oil" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 3) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Refining") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Aluminum;
impl Resource for Aluminum {
    fn name(&self) -> &'static str { "Aluminum" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Science, 1) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Refining") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Niter;
impl Resource for Niter {
    fn name(&self) -> &'static str { "Niter" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle {
        YieldBundle::new()
            .with(crate::YieldType::Food, 1)
            .with(crate::YieldType::Production, 1)
    }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Military Engineering") }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)] pub struct Uranium;
impl Resource for Uranium {
    fn name(&self) -> &'static str { "Uranium" }
    fn category(&self) -> ResourceCategory { ResourceCategory::Strategic }
    fn base_yields(&self) -> YieldBundle { YieldBundle::new().with(crate::YieldType::Production, 2) }
    fn reveal_tech(&self) -> Option<&'static str> { Some("Nuclear Fission") }
}

// ── Enum wrapping all built-in resources for inline storage ───────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinResource {
    // Bonus
    Wheat(Wheat),
    Rice(Rice),
    Cattle(Cattle),
    Sheep(Sheep),
    Fish(Fish),
    Stone(Stone),
    Copper(Copper),
    Deer(Deer),
    // Luxury
    Wine(Wine),
    Silk(Silk),
    Spices(Spices),
    Incense(Incense),
    Cotton(Cotton),
    Ivory(Ivory),
    Sugar(Sugar),
    Salt(Salt),
    // Strategic
    Horses(Horses),
    Iron(Iron),
    Coal(Coal),
    Oil(Oil),
    Aluminum(Aluminum),
    Niter(Niter),
    Uranium(Uranium),
}

impl BuiltinResource {
    pub fn as_def(&self) -> &dyn Resource {
        match self {
            BuiltinResource::Wheat(r)     => r,
            BuiltinResource::Rice(r)      => r,
            BuiltinResource::Cattle(r)    => r,
            BuiltinResource::Sheep(r)     => r,
            BuiltinResource::Fish(r)      => r,
            BuiltinResource::Stone(r)     => r,
            BuiltinResource::Copper(r)    => r,
            BuiltinResource::Deer(r)      => r,
            BuiltinResource::Wine(r)      => r,
            BuiltinResource::Silk(r)      => r,
            BuiltinResource::Spices(r)    => r,
            BuiltinResource::Incense(r)   => r,
            BuiltinResource::Cotton(r)    => r,
            BuiltinResource::Ivory(r)     => r,
            BuiltinResource::Sugar(r)     => r,
            BuiltinResource::Salt(r)      => r,
            BuiltinResource::Horses(r)    => r,
            BuiltinResource::Iron(r)      => r,
            BuiltinResource::Coal(r)      => r,
            BuiltinResource::Oil(r)       => r,
            BuiltinResource::Aluminum(r)  => r,
            BuiltinResource::Niter(r)     => r,
            BuiltinResource::Uranium(r)   => r,
        }
    }
}

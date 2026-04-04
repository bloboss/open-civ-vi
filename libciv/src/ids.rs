use ulid::Ulid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct $name(pub(crate) Ulid);

        impl $name {
            pub fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
            }

            /// A sentinel value (all-zero ULID). Used when an ID slot must be
            /// filled but no real entity exists (e.g. city bombardment attacker).
            pub fn nil() -> Self {
                Self(Ulid::nil())
            }

            pub fn as_ulid(&self) -> Ulid {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

define_id!(CityId);
define_id!(UnitId);
define_id!(CivId);
define_id!(TechId);
define_id!(CivicId);
define_id!(GovernmentId);
define_id!(PolicyId);
define_id!(ReligionId);
define_id!(WonderId);
define_id!(GreatPersonId);
define_id!(PromotionId);
define_id!(ImprovementId);
define_id!(ResourceId);
define_id!(RoadId);
define_id!(AgreementId);
define_id!(GrievanceId);
define_id!(GovernorId);
define_id!(BeliefId);
define_id!(VictoryId);
define_id!(UnitTypeId);
define_id!(DistrictTypeId);
define_id!(BuildingId);
define_id!(TradeRouteId);
define_id!(EraId);
define_id!(TerrainId);
define_id!(FeatureId);
define_id!(EdgeFeatureId);
define_id!(NaturalWonderId);
define_id!(GreatWorkId);
define_id!(BarbarianCampId);

// BeliefRefs is defined in civ::religion (alongside the belief registry builder).

/// Named handles to every built-in tech ID, produced alongside the TechTree.
#[derive(Debug, Clone, Copy)]
pub struct TechRefs {
    // ── Ancient Era ─────────────────────────────────────────────────────────
    pub pottery:              TechId,
    pub animal_husbandry:     TechId,
    pub mining:               TechId,
    pub sailing:              TechId,
    pub archery:              TechId,
    pub astrology:            TechId,
    pub writing:              TechId,
    pub irrigation:           TechId,
    pub bronze_working:       TechId,
    pub the_wheel:            TechId,
    pub masonry:              TechId,
    // ── Classical Era ───────────────────────────────────────────────────────
    pub celestial_navigation: TechId,
    pub currency:             TechId,
    pub horseback_riding:     TechId,
    pub iron_working:         TechId,
    pub shipbuilding:         TechId,
    pub mathematics:          TechId,
    pub construction:         TechId,
    pub engineering:          TechId,
    // ── Medieval Era ────────────────────────────────────────────────────────
    pub military_tactics:     TechId,
    pub apprenticeship:       TechId,
    pub machinery:            TechId,
    pub education:            TechId,
    pub stirrups:             TechId,
    pub military_engineering: TechId,
    pub castles:              TechId,
    // ── Renaissance Era ─────────────────────────────────────────────────────
    pub cartography:          TechId,
    pub mass_production:      TechId,
    pub banking:              TechId,
    pub gunpowder:            TechId,
    pub printing:             TechId,
    pub square_rigging:       TechId,
    pub astronomy:            TechId,
    pub metal_casting:        TechId,
    pub siege_tactics:        TechId,
    // ── Industrial Era ──────────────────────────────────────────────────────
    pub industrialization:    TechId,
    pub scientific_theory:    TechId,
    pub ballistics:           TechId,
    pub military_science:     TechId,
    pub steam_power:          TechId,
    pub sanitation:           TechId,
    pub economics:            TechId,
    pub rifling:              TechId,
    // ── Modern Era ──────────────────────────────────────────────────────────
    pub flight:               TechId,
    pub replaceable_parts:    TechId,
    pub steel:                TechId,
    pub electricity:          TechId,
    pub radio:                TechId,
    pub chemistry:            TechId,
    pub combustion:           TechId,
    // ── Atomic Era ──────────────────────────────────────────────────────────
    pub advanced_flight:      TechId,
    pub rocketry:             TechId,
    pub advanced_ballistics:  TechId,
    pub combined_arms:        TechId,
    pub plastics:             TechId,
    pub computers:            TechId,
    pub nuclear_fission:      TechId,
    pub synthetic_materials:  TechId,
    // ── Information Era ─────────────────────────────────────────────────────
    pub telecommunications:   TechId,
    pub satellites:           TechId,
    pub guidance_systems:     TechId,
    pub lasers:               TechId,
    pub composites:           TechId,
    pub stealth_technology:   TechId,
    pub robotics:             TechId,
    pub nanotechnology:       TechId,
    pub nuclear_fusion:       TechId,
    pub future_tech:          TechId,
    /// Sentinel: self-referential prereq → `prerequisites_met()` always returns false.
    pub unreachable:          TechId,
}

/// Named handles to every built-in civic ID, produced alongside the CivicTree.
#[derive(Debug, Clone, Copy)]
pub struct CivicRefs {
    // ── Ancient Era ─────────────────────────────────────────────────────────
    pub code_of_laws:        CivicId,
    pub craftsmanship:       CivicId,
    pub foreign_trade:       CivicId,
    pub early_empire:        CivicId,
    pub mysticism:           CivicId,
    pub military_tradition:  CivicId,
    pub state_workforce:     CivicId,
    // ── Classical Era ───────────────────────────────────────────────────────
    pub games_and_recreation: CivicId,
    pub political_philosophy: CivicId,
    pub drama_and_poetry:    CivicId,
    pub theology:            CivicId,
    pub military_training:   CivicId,
    pub defensive_tactics:   CivicId,
    pub recorded_history:    CivicId,
    // ── Medieval Era ────────────────────────────────────────────────────────
    pub naval_tradition:     CivicId,
    pub feudalism:           CivicId,
    pub civil_service:       CivicId,
    pub divine_right:        CivicId,
    pub mercenaries:         CivicId,
    pub medieval_faires:     CivicId,
    pub guilds:              CivicId,
    // ── Renaissance Era ─────────────────────────────────────────────────────
    pub exploration:         CivicId,
    pub reformed_church:     CivicId,
    pub humanism:            CivicId,
    pub diplomatic_service:  CivicId,
    pub mercantilism:        CivicId,
    pub the_enlightenment:   CivicId,
    // ── Industrial Era ──────────────────────────────────────────────────────
    pub colonialism:         CivicId,
    pub opera_and_ballet:    CivicId,
    pub natural_history:     CivicId,
    pub civil_engineering:   CivicId,
    pub nationalism:         CivicId,
    pub scorched_earth:      CivicId,
    pub urbanization:        CivicId,
    // ── Modern Era ──────────────────────────────────────────────────────────
    pub conservation:        CivicId,
    pub mass_media:          CivicId,
    pub mobilization:        CivicId,
    pub ideology:            CivicId,
    pub capitalism:          CivicId,
    pub nuclear_program:     CivicId,
    pub suffrage:            CivicId,
    pub totalitarianism:     CivicId,
    pub class_struggle:      CivicId,
    // ── Atomic Era ──────────────────────────────────────────────────────────
    pub cultural_heritage:   CivicId,
    pub cold_war:            CivicId,
    pub professional_sports: CivicId,
    pub rapid_deployment:    CivicId,
    pub space_race:          CivicId,
    // ── Information Era ─────────────────────────────────────────────────────
    pub globalization:       CivicId,
    pub social_media:        CivicId,
    pub future_civic:        CivicId,
    /// Sentinel: self-referential prereq → `prerequisites_met()` always returns false.
    pub unreachable:         CivicId,
}

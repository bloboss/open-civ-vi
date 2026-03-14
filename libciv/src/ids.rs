use ulid::Ulid;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub(crate) Ulid);

        impl $name {
            pub fn from_ulid(ulid: Ulid) -> Self {
                Self(ulid)
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

/// Named handles to every built-in tech ID, produced alongside the TechTree.
#[derive(Debug, Clone, Copy)]
pub struct TechRefs {
    pub pottery:          TechId,
    pub animal_husbandry: TechId,
    pub mining:           TechId,
    pub sailing:          TechId,
    pub archery:          TechId,
    pub astrology:        TechId,
    pub writing:          TechId,
    pub irrigation:       TechId,
    pub bronze_working:   TechId,
    pub the_wheel:        TechId,
    pub masonry:          TechId,
    /// Sentinel: self-referential prereq → `prerequisites_met()` always returns false.
    pub unreachable:      TechId,
}

/// Named handles to every built-in civic ID, produced alongside the CivicTree.
#[derive(Debug, Clone, Copy)]
pub struct CivicRefs {
    pub code_of_laws:  CivicId,
    pub craftsmanship: CivicId,
    pub foreign_trade: CivicId,
    pub early_empire:  CivicId,
    pub mysticism:     CivicId,
    /// Sentinel: self-referential prereq → `prerequisites_met()` always returns false.
    pub unreachable:   CivicId,
}

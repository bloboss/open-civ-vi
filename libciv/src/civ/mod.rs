//! Civilization-layer types: civilizations, cities, units, diplomacy, religion,
//! great people, governors, era system, loyalty, trade routes, tourism, and
//! barbarian clans.

pub mod barbarian;
pub mod city;
pub mod city_state;
pub mod city_state_defs;
pub mod civ_ability;
pub mod civ_identity;
pub mod civilization;
pub mod congress;
pub mod diplomacy;
pub mod district;
pub mod era;
pub mod governor;
pub mod great_people;
pub mod great_works;
pub mod grievance;
pub mod religion;
pub mod tourism;
pub mod trade;
pub mod unit;

pub use city::{City, CityFocus, CityKind, CityOwnership, ProductionItem, WallLevel};
pub use city_state::{CityStateBonus, CityStateData, CityStateType};
pub use city_state_defs::{CityStateDef, builtin_city_state_defs};
pub use civilization::{
    Agenda, BuiltinAgenda, CivicProgress, Civilization, Leader, LeaderAbility, StartBias,
    TechProgress,
};
pub use diplomacy::{
    Agreement, AllianceType, DiplomaticRelation, DiplomaticStatus, GrievanceRecord,
    GrievanceTrigger, GrievanceVisibility,
};
pub use district::{
    AdjacencyContext, BuildingDef, BuiltinDistrict, DistrictDef, DistrictRequirements,
    PlacedDistrict,
};
pub use era::{
    Era, EraAge, EraDedication, EraTrigger, HistoricMoment, HistoricMomentDef, HistoricMomentKind,
};
pub use grievance::{CapturedCityGrievance, DeclaredWarGrievance, PillageGrievance};
pub mod historic_moments;
pub use barbarian::{BarbarianCamp, BarbarianConfig, ClanInteraction, ClanType, ScoutState};
pub use civ_ability::{CityFoundedHook, CivAbilityBundle, RuleOverride};
pub use civ_identity::{BuiltinCiv, BuiltinLeader};
pub use congress::{ActiveResolution, ResolutionKind, WorldCongress};
pub use governor::{
    GOVERNOR_NAMES, Governor, GovernorDef, GovernorPromotion, GovernorPromotionDef,
    all_promotion_defs, get_governor_modifiers, promotion_def, promotions_for,
};
pub use great_people::{
    GP_BASE_POINTS_PER_BUILDING, GP_BASE_POINTS_PER_DISTRICT, GP_BASE_THRESHOLD,
    GP_PATRONAGE_FAITH_PER_POINT, GP_PATRONAGE_GOLD_PER_POINT, GP_THRESHOLD_INCREMENT, GreatPerson,
    GreatPersonAbility, GreatPersonDef, RetireEffect, building_great_person_points,
    builtin_great_person_defs, current_era_name, district_great_person_types,
    era_is_current_or_earlier, next_candidate_name, recruitment_threshold, spawn_great_person,
};
pub use great_works::{GreatWork, GreatWorkSlot, GreatWorkSlotType, GreatWorkType};
pub use religion::{BeliefCategory, BeliefContext, BeliefRefs, BuiltinBelief, Religion};
pub use tourism::{WonderTourism, compute_tourism, domestic_tourists, has_cultural_dominance};
pub use trade::TradeRoute;
pub use unit::{BasicUnit, Unit};

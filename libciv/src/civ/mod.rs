//! Civilization-layer types: civilizations, cities, units, diplomacy, religion,
//! great people, governors, era system, loyalty, trade routes, tourism, and
//! barbarian clans.

pub mod barbarian;
pub mod civ_ability;
pub mod civ_identity;
pub mod city;
pub mod city_state;
pub mod congress;
pub mod city_state_defs;
pub mod civilization;
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

pub use city::{City, CityKind, CityOwnership, ProductionItem, WallLevel};
pub use city_state::{CityStateBonus, CityStateData, CityStateType};
pub use city_state_defs::{CityStateDef, builtin_city_state_defs};
pub use civilization::{Agenda, BuiltinAgenda, Civilization, CivicProgress, Leader, LeaderAbility, StartBias, TechProgress};
pub use diplomacy::{
    Agreement, AllianceType, DiplomaticRelation, DiplomaticStatus, GrievanceTrigger,
    GrievanceRecord, GrievanceVisibility,
};
pub use grievance::{CapturedCityGrievance, DeclaredWarGrievance, PillageGrievance};
pub use district::{AdjacencyContext, BuildingDef, BuiltinDistrict, DistrictDef, DistrictRequirements, PlacedDistrict};
pub use era::{Era, EraAge, EraDedication, EraTrigger, HistoricMoment, HistoricMomentDef, HistoricMomentKind};
pub mod historic_moments;
pub use great_people::{
    GreatPerson, GreatPersonAbility, GreatPersonDef, RetireEffect,
    builtin_great_person_defs, spawn_great_person,
    district_great_person_types, next_candidate_name, recruitment_threshold,
    current_era_name, era_is_current_or_earlier,
    GP_BASE_POINTS_PER_DISTRICT, GP_BASE_THRESHOLD, GP_THRESHOLD_INCREMENT,
    GP_PATRONAGE_GOLD_PER_POINT, GP_PATRONAGE_FAITH_PER_POINT,
};
pub use governor::{
    Governor, GovernorDef, GovernorPromotion, GovernorPromotionDef,
    all_promotion_defs, promotions_for, promotion_def, get_governor_modifiers,
    GOVERNOR_NAMES,
};
pub use great_works::{GreatWork, GreatWorkSlot, GreatWorkSlotType, GreatWorkType};
pub use religion::{BeliefCategory, BeliefContext, BeliefRefs, BuiltinBelief, Religion};
pub use tourism::{WonderTourism, compute_tourism, domestic_tourists, has_cultural_dominance};
pub use trade::TradeRoute;
pub use unit::{BasicUnit, Unit};
pub use civ_identity::{BuiltinCiv, BuiltinLeader};
pub use barbarian::{BarbarianCamp, BarbarianConfig, ClanType, ScoutState, ClanInteraction};
pub use civ_ability::{CivAbilityBundle, CityFoundedHook, RuleOverride};
pub use congress::{WorldCongress, ActiveResolution, ResolutionKind};

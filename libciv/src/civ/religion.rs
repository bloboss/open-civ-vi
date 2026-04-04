use crate::{BeliefId, CivId, ReligionId};
use crate::rules::modifier::Modifier;
use super::city::City;

// ── Belief categories ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BeliefCategory {
    /// Terrain/gameplay bonus chosen before founding a religion.
    Pantheon,
    /// Bonus to the founding civilization only.
    Founder,
    /// Bonus to all cities where this religion is the majority.
    Follower,
    /// Unlocks a worship building purchasable with faith.
    Worship,
    /// Bonus to religious spread and defense.
    Enhancer,
}

// ── Built-in belief definition ───────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct BuiltinBelief {
    pub id: BeliefId,
    pub name: &'static str,
    pub description: &'static str,
    pub category: BeliefCategory,
    pub modifiers: Vec<Modifier>,
}

/// Context for evaluating belief effects.
#[derive(Debug, Clone, Default)]
pub struct BeliefContext {
    pub followers: u32,
    /// Number of cities where this religion is the majority.
    pub majority_religion_city_count: u32,
}

// ── Religion ─────────────────────────────────────────────────────────────────

/// A founded religion. Follower counts live on `City.religious_followers`;
/// `Religion` provides query methods that take `&[City]` to derive totals.
#[derive(Debug, Clone)]
pub struct Religion {
    pub id: ReligionId,
    pub name: String,
    pub founded_by: CivId,
    pub holy_city: crate::CityId,
    pub beliefs: Vec<BeliefId>,
}

impl Religion {
    pub fn new(id: ReligionId, name: String, founder: CivId, holy_city: crate::CityId) -> Self {
        Self {
            id,
            name,
            founded_by: founder,
            holy_city,
            beliefs: Vec::new(),
        }
    }

    /// Compute total followers by querying all cities (source of truth is City).
    pub fn total_followers(&self, cities: &[City]) -> u32 {
        cities.iter()
            .filter_map(|c| c.religious_followers.get(&self.id))
            .sum()
    }

    /// Check enhancement status by querying belief definitions.
    pub fn is_enhanced(&self, belief_defs: &[BuiltinBelief]) -> bool {
        self.beliefs.iter().any(|bid| {
            belief_defs.iter().any(|def| def.id == *bid && def.category == BeliefCategory::Enhancer)
        })
    }

    /// Collect all cities where this religion is the majority.
    pub fn majority_cities(&self, cities: &[City]) -> Vec<crate::CityId> {
        cities.iter()
            .filter(|c| c.majority_religion() == Some(self.id))
            .map(|c| c.id)
            .collect()
    }

    /// Get followers in a specific city.
    pub fn followers_in_city(&self, city: &City) -> u32 {
        city.religious_followers.get(&self.id).copied().unwrap_or(0)
    }
}

// ── Belief registry builder ──────────────────────────────────────────────────

/// Build the built-in belief definitions with deterministic IDs.
/// Returns (beliefs_vec, named_refs) — same pattern as `build_tech_tree`.
pub fn build_beliefs(ids: &mut crate::game::state::IdGenerator) -> (Vec<BuiltinBelief>, BeliefRefs) {
    use crate::rules::modifier::*;
    use crate::civ::district::BuiltinDistrict;
    use crate::world::feature::BuiltinFeature;
    use crate::world::improvement::BuiltinImprovement;
    use crate::world::terrain::BuiltinTerrain;
    use crate::ResourceCategory;
    use crate::YieldType;

    let mut beliefs: Vec<BuiltinBelief> = Vec::new();

    let refs: BeliefRefs = include!("belief_defs.rs");

    (beliefs, refs)
}

/// Named handles to every built-in belief ID.
#[derive(Debug, Clone, Copy)]
pub struct BeliefRefs {
    // ── Pantheon beliefs ──
    pub dance_of_the_aurora: BeliefId,
    pub desert_folklore: BeliefId,
    pub sacred_path: BeliefId,
    pub stone_circles: BeliefId,
    pub religious_idols: BeliefId,
    pub earth_goddess: BeliefId,
    pub god_of_the_sea: BeliefId,
    pub god_of_the_forge: BeliefId,
    pub divine_spark: BeliefId,
    pub lady_of_the_reeds_and_marshes: BeliefId,
    pub oral_tradition: BeliefId,
    pub monument_to_the_gods: BeliefId,
    pub river_goddess: BeliefId,
    pub city_patron_goddess: BeliefId,
    pub fertility_rites: BeliefId,
    pub god_of_war: BeliefId,
    pub god_of_craftsmen: BeliefId,
    pub initiation_rites: BeliefId,
    pub religious_settlements: BeliefId,
    pub goddess_of_the_hunt: BeliefId,
    // ── Founder beliefs ──
    pub church_property: BeliefId,
    pub tithe: BeliefId,
    pub papal_primacy: BeliefId,
    pub religious_unity: BeliefId,
    // ── Follower beliefs ──
    pub divine_inspiration: BeliefId,
    pub choral_music: BeliefId,
    pub religious_community: BeliefId,
    pub feed_the_world: BeliefId,
    // ── Worship beliefs ──
    pub cathedral: BeliefId,
    pub gurdwara: BeliefId,
    pub mosque: BeliefId,
    pub pagoda: BeliefId,
    pub synagogue: BeliefId,
    pub wat: BeliefId,
    pub meeting_house: BeliefId,
    pub stupa: BeliefId,
    pub dar_e_mehr: BeliefId,
    // ── Enhancer beliefs ──
    pub missionary_zeal: BeliefId,
    pub holy_order: BeliefId,
    pub itinerant_preachers: BeliefId,
    pub scripture: BeliefId,
}

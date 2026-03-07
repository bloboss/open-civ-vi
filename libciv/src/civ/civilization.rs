use std::collections::{HashMap, HashSet};
use crate::{
    AgeType, CivId, CivicId, GovernmentId, PolicyId, ResourceId, TechId, YieldBundle,
};
use crate::rules::modifier::Modifier;
use crate::rules::policy::{Government, Policy};
use crate::rules::tech::{TechTree, CivicTree};
use crate::world::resource::BuiltinResource;

pub trait StartBias: std::fmt::Debug {
    fn terrain_preference(&self) -> Option<crate::TerrainId>;
    fn feature_preference(&self) -> Option<crate::FeatureId>;
    fn resource_preference(&self) -> Option<crate::ResourceCategory>;
}

pub trait LeaderAbility: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}

pub trait Agenda: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Returns a grievance score (-ve = bad, +ve = good) toward the given civ.
    fn attitude(&self, toward: CivId) -> i32;
}

#[derive(Debug, Clone)]
pub struct TechProgress {
    pub tech_id: TechId,
    pub progress: u32,
    pub boosted: bool,
}

#[derive(Debug, Clone)]
pub struct CivicProgress {
    pub civic_id: CivicId,
    pub progress: u32,
    pub inspired: bool,
}

#[derive(Debug)]
pub struct Leader {
    pub name: &'static str,
    pub civ_id: CivId,
    pub abilities: Vec<Box<dyn LeaderAbility>>,
    pub agenda: Box<dyn Agenda>,
}

#[derive(Debug)]
pub struct Civilization {
    pub id: CivId,
    pub name: &'static str,
    // FIXME: What does this do??
    pub adjective: &'static str,
    pub leader: Leader,
    pub cities: Vec<crate::CityId>,
    // TODO: Remove this. Make a method get_capital, then we can query ownership
    pub capital: Option<crate::CityId>,
    pub current_era: AgeType,
    pub researched_techs: Vec<TechId>,
    // TODO: Should this be a queue? Especially since the user can queue up
    //  techs and civics to research in succession.
    pub tech_in_progress: Option<TechProgress>,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgress>,
    pub current_government: Option<GovernmentId>,
    pub active_policies: Vec<PolicyId>,
    pub gold: i32,
    // TODO: Should this be recomputed as a query? Since it is given by effects
    pub treasury_per_turn: i32,
    pub yields: YieldBundle,
    /// Stockpile of consumable strategic resources (e.g. Iron, Horses).
    pub strategic_resources: HashMap<ResourceId, u32>,

    // ── OneShotEffect tracking fields ─────────────────────────────────────────
    /// Resources made visible by tech or effect. Used as idempotency guard for
    /// `OneShotEffect::RevealResource`.
    pub revealed_resources: HashSet<BuiltinResource>,
    /// Techs for which the Eureka boost has been earned. Guards re-triggering.
    pub eureka_triggered: HashSet<&'static str>,
    /// Civics for which the Inspiration boost has been earned. Guards re-triggering.
    pub inspiration_triggered: HashSet<&'static str>,
    /// Government types unlocked for adoption.
    pub unlocked_governments: Vec<&'static str>,
    /// Name of the currently adopted government, if any.
    pub current_government_name: Option<&'static str>,
    /// Unit types unlocked for production by this civ.
    pub unlocked_units: Vec<&'static str>,
    /// Building types unlocked for production by this civ.
    pub unlocked_buildings: Vec<&'static str>,
    /// Improvement types unlocked for builders of this civ.
    pub unlocked_improvements: Vec<&'static str>,
}

impl Civilization {
    pub fn new(id: CivId, name: &'static str, adjective: &'static str, leader: Leader) -> Self {
        Self {
            id,
            name,
            adjective,
            leader,
            cities: Vec::new(),
            capital: None,
            current_era: AgeType::Ancient,
            researched_techs: Vec::new(),
            tech_in_progress: None,
            completed_civics: Vec::new(),
            civic_in_progress: None,
            current_government: None,
            active_policies: Vec::new(),
            gold: 0,
            treasury_per_turn: 0,
            yields: YieldBundle::default(),
            strategic_resources: HashMap::new(),
            revealed_resources: HashSet::new(),
            eureka_triggered: HashSet::new(),
            inspiration_triggered: HashSet::new(),
            unlocked_governments: Vec::new(),
            current_government_name: None,
            unlocked_units: Vec::new(),
            unlocked_buildings: Vec::new(),
            unlocked_improvements: Vec::new(),
        }
    }

    pub fn has_tech(&self, tech_id: TechId) -> bool {
        self.researched_techs.contains(&tech_id)
    }

    pub fn has_civic(&self, civic_id: CivicId) -> bool {
        self.completed_civics.contains(&civic_id)
    }

    /// Collect all active modifiers for this civilization from every source:
    /// leader abilities, active policies, and current government. This is a
    /// computed view — no separate modifier cache is maintained.
    ///
    /// Tech/civic completion modifiers are handled by `OneShotEffect` and will
    /// be integrated here when a wonder/tech-modifier registry is added.
    pub fn get_modifiers(
        &self,
        policies: &[Policy],
        governments: &[Government],
    ) -> Vec<Modifier> {
        let mut modifiers = Vec::new();

        for ability in &self.leader.abilities {
            modifiers.extend(ability.modifiers());
        }

        for pid in &self.active_policies {
            if let Some(policy) = policies.iter().find(|p| p.id == *pid) {
                modifiers.extend(policy.modifiers.iter().cloned());
            }
        }

        if let Some(gov_id) = self.current_government {
            if let Some(gov) = governments.iter().find(|g| g.id == gov_id) {
                modifiers.extend(gov.inherent_modifiers.iter().cloned());
            }
        }

        // TODO: Get the modifiers from:
        // - completed techs
        // - completed civics
        // - cities

        modifiers
    }

    /// Collect modifiers sourced from completed techs and civics via the tech
    /// and civic trees. Called alongside `get_modifiers` when trees are available.
    pub fn get_tree_modifiers(
        &self,
        _tech_tree: &TechTree,
        _civic_tree: &CivicTree,
    ) -> Vec<Modifier> {
        // Placeholder: when GrantModifier is added to OneShotEffect (Phase 4,
        // with wonder/ability registry), this will iterate researched_techs and
        // completed_civics to collect GrantModifier effects from the tree nodes.
        Vec::new()
    }
}

use std::collections::{HashMap, HashSet, VecDeque};
use crate::{
    AgeType, BeliefId, CivId, CivicId, GovernmentId, GreatPersonType, PolicyId, ReligionId, TechId, YieldType,
};
use super::era::{EraAge, HistoricMoment};
use crate::rules::effect::OneShotEffect;
use crate::rules::modifier::{EffectType, Modifier, ModifierSource, StackingRule, TargetSelector};
use crate::rules::policy::{Government, Policy};
use crate::rules::tech::{TechTree, CivicTree};
use crate::world::resource::BuiltinResource;
use super::city::City;
use super::diplomacy::DiplomaticRelation;

pub trait StartBias: std::fmt::Debug {
    fn terrain_preference(&self) -> Option<crate::TerrainId>;
    fn feature_preference(&self) -> Option<crate::FeatureId>;
    fn resource_preference(&self) -> Option<crate::ResourceCategory>;
}

pub trait LeaderAbility: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn modifiers(&self) -> Vec<Modifier>;
}

pub trait Agenda: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Returns a grievance score (-ve = bad, +ve = good) toward the given civ.
    fn attitude(&self, toward: CivId) -> i32;
}

/// Concrete agenda enum that replaces `Box<dyn Agenda>` for serialization.
/// Currently only a single variant exists; extend as specific leader agendas are added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BuiltinAgenda {
    /// No-op / neutral agenda — returns 0 attitude toward everyone.
    Default,
}

impl BuiltinAgenda {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Default => "Neutral",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Default => "No preferences.",
        }
    }

    pub fn attitude(&self, _toward: CivId) -> i32 {
        match self {
            Self::Default => 0,
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TechProgress {
    pub tech_id: TechId,
    pub progress: u32,
    pub boosted: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CivicProgress {
    pub civic_id: CivicId,
    pub progress: u32,
    pub inspired: bool,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
pub struct Leader {
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
    pub name: &'static str,
    pub civ_id: CivId,
    pub agenda: BuiltinAgenda,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
pub struct Civilization {
    pub id: CivId,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
    pub name: &'static str,
    /// Which built-in civilization this is, for gating unique components.
    pub civ_identity: Option<super::civ_identity::BuiltinCiv>,
    /// Which built-in leader this is.
    pub leader_identity: Option<super::civ_identity::BuiltinLeader>,
    /// Used for flavor text in unique unit unlock descriptions
    /// (e.g., "Roman unit" -> "Legionary"). Not yet queried by the rules engine.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
    pub adjective: &'static str,
    pub leader: Leader,
    pub cities: Vec<crate::CityId>,
    pub current_era: AgeType,
    pub researched_techs: Vec<TechId>,
    /// Ordered research queue. `advance_turn` applies science to `front_mut()`;
    /// when a tech completes `pop_front()` is called so the next queued tech
    /// begins automatically on the following turn.
    pub research_queue: VecDeque<TechProgress>,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgress>,
    pub current_government: Option<GovernmentId>,
    pub active_policies: Vec<PolicyId>,
    pub gold: i32,
    /// Accumulated faith currency, used to purchase religious units and buildings.
    pub faith: u32,
    /// Pantheon belief selected before full religion founding.
    pub pantheon_belief: Option<BeliefId>,
    /// The religion this civilization has founded, if any.
    pub founded_religion: Option<ReligionId>,
    /// Whether this civ has had an apostle launch an inquisition (unlocks Inquisitor purchase).
    pub inquisition_launched: bool,
    /// Number of each religious unit type purchased (for cost scaling).
    pub faith_purchase_counts: HashMap<String, u32>,
    // treasury_per_turn removed (PHASE3-3.4): gold income is computed per turn
    // via compute_yields().gold and goes stale when policies or government change.
    /// Stockpile of consumable strategic resources (e.g. Iron, Horses).
    pub strategic_resources: HashMap<BuiltinResource, u32>,

    // ── OneShotEffect tracking fields ─────────────────────────────────────────
    /// Resources made visible by tech or effect. Used as idempotency guard for
    /// `OneShotEffect::RevealResource`.
    pub revealed_resources: HashSet<BuiltinResource>,
    /// Techs for which the Eureka boost has been earned. Guards re-triggering.
    pub eureka_triggered: HashSet<TechId>,
    /// Civics for which the Inspiration boost has been earned. Guards re-triggering.
    pub inspiration_triggered: HashSet<CivicId>,
    /// Government types unlocked for adoption.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_vec"))]
    pub unlocked_governments: Vec<&'static str>,
    /// Name of the currently adopted government, if any.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_opt"))]
    pub current_government_name: Option<&'static str>,
    /// Policy cards unlocked for equipping in government slots.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_vec"))]
    pub unlocked_policies: Vec<&'static str>,
    /// Unit types unlocked for production by this civ.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_vec"))]
    pub unlocked_units: Vec<&'static str>,
    /// Building types unlocked for production by this civ.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_vec"))]
    pub unlocked_buildings: Vec<&'static str>,
    /// Improvement types unlocked for builders of this civ.
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_vec"))]
    pub unlocked_improvements: Vec<&'static str>,

    // ── Great person tracking ───────────────────────────────────────────────
    /// Permanent modifiers granted by retired great persons.
    /// Skipped during serialization; rebuilt from great person data after load.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub great_person_modifiers: Vec<Modifier>,
    /// Accumulated great person points per type. Districts generate points
    /// each turn; when points reach the recruitment threshold the next
    /// available candidate of that type is automatically recruited.
    pub great_person_points: HashMap<GreatPersonType, u32>,

    // ── Tourism & culture tracking ──────────────────────────────────────────
    /// Total culture accumulated over the lifetime of this civilization.
    /// Used to compute domestic tourists (defense against culture victory).
    pub lifetime_culture: u32,
    /// Per-target-civ accumulated tourism. Key = target civ, value = total
    /// tourism pressure applied so far. Culture victory is achieved when
    /// `tourism_accumulated[B] >= B.lifetime_culture` for every other civ B.
    pub tourism_accumulated: HashMap<CivId, u32>,

    // ── Era score tracking ──────────────────────────────────────────────────
    /// Accumulated era score for the current era; resets when the global era advances.
    pub era_score: u32,
    /// The civ's current era age (Dark/Normal/Golden/Heroic).
    pub era_age: EraAge,
    /// All historic moments earned by this civ across all eras.
    #[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
    pub historic_moments: Vec<HistoricMoment>,
    /// Names of unique historic moments already earned this era (uniqueness guard).
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str_set"))]
    pub earned_moments: HashSet<&'static str>,

    // ── Tourism / cultural victory ──────────────────────────────────────────
    /// Total tourism generated per turn (recomputed in advance_turn Phase 3c).
    pub tourism_output: u32,
    /// Accumulated lifetime culture. Serves as the "domestic culture" defense
    /// against other civs' tourism for the cultural victory condition.
    pub domestic_culture: u32,
    // ── Governor titles ─────────────────────────────────────────────────────────
    /// Number of unspent governor titles. Titles are earned from civics and spent
    /// to appoint new governors or promote existing ones.
    pub governor_titles: u32,

    // ── Science victory tracking ─────────────────────────────────────────────
    /// Number of science milestones completed (max 3 for Science Victory).
    pub science_milestones_completed: u32,

    // ── Diplomatic victory tracking ──────────────────────────────────────────
    /// Accumulated diplomatic favor. At 100 the civ wins a Diplomatic Victory.
    pub diplomatic_favor: u32,

    // ── Fog of war ────────────────────────────────────────────────────────────
    /// Tiles currently within this civ's vision this turn.
    /// Cleared and rebuilt by `recalculate_visibility` after every unit move
    /// and at the start of each new game session.
    pub visible_tiles: HashSet<libhexgrid::coord::HexCoord>,
    /// Tiles that have ever been seen; never removed. Used to render
    /// fog-of-war memory (explored but currently out of vision).
    pub explored_tiles: HashSet<libhexgrid::coord::HexCoord>,
}

impl Civilization {
    pub fn new(id: CivId, name: &'static str, adjective: &'static str, leader: Leader) -> Self {
        Self {
            id,
            name,
            civ_identity: None,
            leader_identity: None,
            adjective,
            leader,
            cities: Vec::new(),
            current_era: AgeType::Ancient,
            researched_techs: Vec::new(),
            research_queue: VecDeque::new(),
            completed_civics: Vec::new(),
            civic_in_progress: None,
            current_government: None,
            active_policies: Vec::new(),
            gold: 0,
            faith: 0,
            pantheon_belief: None,
            founded_religion: None,
            inquisition_launched: false,
            faith_purchase_counts: HashMap::new(),
            strategic_resources: HashMap::new(),
            revealed_resources: HashSet::new(),
            eureka_triggered: HashSet::new(),
            inspiration_triggered: HashSet::new(),
            unlocked_governments: Vec::new(),
            current_government_name: None,
            unlocked_policies: Vec::new(),
            unlocked_units: Vec::new(),
            unlocked_buildings: Vec::new(),
            unlocked_improvements: Vec::new(),
            great_person_modifiers: Vec::new(),
            great_person_points: HashMap::new(),
            lifetime_culture: 0,
            tourism_accumulated: HashMap::new(),
            era_score: 0,
            era_age: EraAge::Normal,
            historic_moments: Vec::new(),
            earned_moments: HashSet::new(),
            tourism_output: 0,
            domestic_culture: 0,
            governor_titles: 0,
            science_milestones_completed: 0,
            diplomatic_favor: 0,
            visible_tiles:  HashSet::new(),
            explored_tiles: HashSet::new(),
        }
    }

    pub fn has_tech(&self, tech_id: TechId) -> bool {
        self.researched_techs.contains(&tech_id)
    }

    pub fn has_civic(&self, civic_id: CivicId) -> bool {
        self.completed_civics.contains(&civic_id)
    }

    /// Return the capital city of this civilization from the provided cities slice.
    /// Searches for a city owned by this civ with `is_capital == true`.
    /// This is a linear scan; cache later if profiling warrants it.
    pub fn capital<'a>(&self, cities: &'a [City]) -> Option<&'a City> {
        cities.iter().find(|c| c.owner == self.id && c.is_capital)
    }

    /// Collect all active modifiers for this civilization from every source:
    /// leader abilities, active policies, current government, and war weariness.
    /// This is a computed view — no separate modifier cache is maintained.
    ///
    /// Tech/civic tree modifiers (GrantModifier effects) are collected separately
    /// via `get_tree_modifiers` and merged at the `compute_yields` call site.
    pub fn get_modifiers(
        &self,
        policies: &[Policy],
        governments: &[Government],
        diplomatic_relations: &[DiplomaticRelation],
    ) -> Vec<Modifier> {
        let mut modifiers = Vec::new();

        // Leader abilities: currently no concrete implementations exist.
        // When BuiltinLeaderAbility is added, iterate here.

        for pid in &self.active_policies {
            if let Some(policy) = policies.iter().find(|p| p.id == *pid) {
                modifiers.extend(policy.modifiers.iter().cloned());
            }
        }

        if let Some(gov_id) = self.current_government
            && let Some(gov) = governments.iter().find(|g| g.id == gov_id)
        {
            modifiers.extend(gov.inherent_modifiers.iter().cloned());
        }

        // War weariness: each active war applies a culture and amenity penalty.
        for rel in diplomatic_relations {
            if (rel.civ_a == self.id || rel.civ_b == self.id) && rel.turns_at_war > 0 {
                modifiers.push(Modifier::new(
                    ModifierSource::Custom("war_weariness"),
                    TargetSelector::Civilization(self.id),
                    EffectType::YieldFlat(YieldType::Culture, -2),
                    StackingRule::Additive,
                ));
                modifiers.push(Modifier::new(
                    ModifierSource::Custom("war_weariness"),
                    TargetSelector::Civilization(self.id),
                    EffectType::YieldFlat(YieldType::Amenities, -1),
                    StackingRule::Additive,
                ));
            }
        }

        modifiers
    }

    /// Collect modifiers granted by completed techs and civics. Each `GrantModifier`
    /// effect stored on a node's `effects` list is returned here; `resolve_modifiers`
    /// will stack them correctly alongside the other modifier sources.
    pub fn get_tree_modifiers(
        &self,
        tech_tree: &TechTree,
        civic_tree: &CivicTree,
    ) -> Vec<Modifier> {
        let mut modifiers = Vec::new();

        for &tech_id in &self.researched_techs {
            if let Some(node) = tech_tree.get(tech_id) {
                for effect in &node.effects {
                    if let OneShotEffect::GrantModifier(m) = effect {
                        modifiers.push(m.clone());
                    }
                }
            }
        }

        for &civic_id in &self.completed_civics {
            if let Some(node) = civic_tree.get(civic_id) {
                for effect in &node.effects {
                    if let OneShotEffect::GrantModifier(m) = effect {
                        modifiers.push(m.clone());
                    }
                }
            }
        }

        modifiers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::tech::{TechNode, TechTree, CivicTree};
    use crate::rules::modifier::{EffectType, ModifierSource, StackingRule, TargetSelector};
    use crate::civ::diplomacy::{DiplomaticRelation, DiplomaticStatus};
    use ulid::Ulid;

    fn empty_civ() -> Civilization {
        let id = CivId::from_ulid(Ulid::nil());
        Civilization::new(id, "Test", "Test",
            Leader { name: "L", civ_id: id, agenda: BuiltinAgenda::Default })
    }

    #[test]
    fn get_tree_modifiers_collects_grant_modifier_from_tech() {
        let mut civ = empty_civ();
        let mut tech_tree = TechTree::new();
        let civic_tree = CivicTree::new();

        let tid = TechId::from_ulid(Ulid::nil());
        let modifier = Modifier::new(
            ModifierSource::Tech("Pottery"),
            TargetSelector::Global,
            EffectType::YieldFlat(YieldType::Culture, 1),
            StackingRule::Additive,
        );
        tech_tree.add_node(TechNode {
            id: tid, name: "Pottery", cost: 50,
            prerequisites: vec![],
            effects: vec![OneShotEffect::GrantModifier(modifier)],
            eureka_description: "",
            eureka_effects: vec![],
        });
        civ.researched_techs.push(tid);

        let mods = civ.get_tree_modifiers(&tech_tree, &civic_tree);
        assert_eq!(mods.len(), 1, "one GrantModifier collected");
        match mods[0].effect {
            EffectType::YieldFlat(YieldType::Culture, 1) => {}
            other => panic!("unexpected effect: {:?}", other),
        }
    }

    #[test]
    fn get_tree_modifiers_ignores_non_grant_effects() {
        let mut civ = empty_civ();
        let mut tech_tree = TechTree::new();
        let civic_tree = CivicTree::new();

        let tid = TechId::from_ulid(Ulid::nil());
        tech_tree.add_node(TechNode {
            id: tid, name: "Mining", cost: 50,
            prerequisites: vec![],
            effects: vec![OneShotEffect::UnlockImprovement("Mine")],
            eureka_description: "",
            eureka_effects: vec![],
        });
        civ.researched_techs.push(tid);

        let mods = civ.get_tree_modifiers(&tech_tree, &civic_tree);
        assert!(mods.is_empty(), "non-GrantModifier effects should not appear");
    }

    #[test]
    fn get_modifiers_war_weariness_applied() {
        let civ = empty_civ();
        let civ_b = CivId::from_ulid(Ulid::from_parts(1, 0));
        let mut rel = DiplomaticRelation::new(civ.id, civ_b);
        rel.status = DiplomaticStatus::War;
        rel.turns_at_war = 5;

        let mods = civ.get_modifiers(&[], &[], &[rel]);
        let culture_penalty: i32 = mods.iter().filter_map(|m| {
            if let EffectType::YieldFlat(YieldType::Culture, v) = m.effect { Some(v) } else { None }
        }).sum();
        assert!(culture_penalty < 0, "culture penalty from war weariness: {}", culture_penalty);
    }

    #[test]
    fn get_modifiers_no_war_no_penalty() {
        let civ = empty_civ();
        // Peaceful relation: turns_at_war == 0.
        let civ_b = CivId::from_ulid(Ulid::from_parts(1, 0));
        let rel = DiplomaticRelation::new(civ.id, civ_b);

        let mods = civ.get_modifiers(&[], &[], &[rel]);
        let culture_delta: i32 = mods.iter().filter_map(|m| {
            if let EffectType::YieldFlat(YieldType::Culture, v) = m.effect { Some(v) } else { None }
        }).sum();
        assert_eq!(culture_delta, 0, "no war weariness in peacetime");
    }
}

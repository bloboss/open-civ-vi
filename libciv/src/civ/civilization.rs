use std::collections::{HashMap, HashSet, VecDeque};
use crate::{
    AgeType, CivId, CivicId, GovernmentId, PolicyId, TechId, YieldType,
};
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
    /// Used for flavor text in unique unit unlock descriptions
    /// (e.g., "Roman unit" -> "Legionary"). Not yet queried by the rules engine.
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
    pub unlocked_governments: Vec<&'static str>,
    /// Name of the currently adopted government, if any.
    pub current_government_name: Option<&'static str>,
    /// Policy cards unlocked for equipping in government slots.
    pub unlocked_policies: Vec<&'static str>,
    /// Unit types unlocked for production by this civ.
    pub unlocked_units: Vec<&'static str>,
    /// Building types unlocked for production by this civ.
    pub unlocked_buildings: Vec<&'static str>,
    /// Improvement types unlocked for builders of this civ.
    pub unlocked_improvements: Vec<&'static str>,

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

        for ability in &self.leader.abilities {
            modifiers.extend(ability.modifiers());
        }

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
        struct NoOp;
        impl std::fmt::Debug for NoOp {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "NoOp") }
        }
        impl Agenda for NoOp {
            fn name(&self) -> &'static str { "noop" }
            fn description(&self) -> &'static str { "" }
            fn attitude(&self, _: CivId) -> i32 { 0 }
        }
        let id = CivId::from_ulid(Ulid::nil());
        Civilization::new(id, "Test", "Test",
            Leader { name: "L", civ_id: id, abilities: vec![], agenda: Box::new(NoOp) })
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

use crate::{CivicId, CivicRefs, TechId, TechRefs};
use std::collections::HashMap;
use super::effect::OneShotEffect;
use crate::game::state::IdGenerator;

#[derive(Debug, Clone)]
pub struct TechNode {
    pub id:                 TechId,
    pub name:               &'static str,
    pub cost:               u32,
    pub prerequisites:      Vec<TechId>,
    /// Effects applied when this tech is completed.
    pub effects:            Vec<OneShotEffect>,
    pub eureka_description: &'static str,
    // TODO(PHASE3-8.4): Add eureka_conditions: Vec<Box<dyn EurekaCondition>>; advance_turn
    //   evaluates is_met(&GameState, CivId) each turn and fires TriggerEureka if true.
    //   Concrete types: KilledUnitsEureka, FoundedCityOnCoast, BuiltImprovementEureka, etc.
    /// Effects applied when the Eureka boost for this tech is triggered.
    pub eureka_effects:     Vec<OneShotEffect>,
}

#[derive(Debug, Clone)]
pub struct CivicNode {
    pub id:                        CivicId,
    pub name:                      &'static str,
    pub cost:                      u32,
    pub prerequisites:             Vec<CivicId>,
    /// Effects applied when this civic is completed.
    pub effects:                   Vec<OneShotEffect>,
    pub inspiration_description:   &'static str,
    /// Effects applied when the Inspiration boost for this civic is triggered.
    pub inspiration_effects:       Vec<OneShotEffect>,
}

#[derive(Debug, Default)]
pub struct TechTree {
    pub nodes: HashMap<TechId, TechNode>,
}

impl TechTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: TechNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn get(&self, id: TechId) -> Option<&TechNode> {
        self.nodes.get(&id)
    }

    pub fn prerequisites_met(&self, id: TechId, researched: &[TechId]) -> bool {
        let Some(node) = self.nodes.get(&id) else { return false };
        node.prerequisites.iter().all(|p| researched.contains(p))
    }
}

#[derive(Debug, Default)]
pub struct CivicTree {
    pub nodes: HashMap<CivicId, CivicNode>,
}

impl CivicTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: CivicNode) {
        self.nodes.insert(node.id, node);
    }

    pub fn get(&self, id: CivicId) -> Option<&CivicNode> {
        self.nodes.get(&id)
    }
}

/// Build the game's full tech tree, returning the tree and named ID handles.
///
/// IDs are generated from `ids` in a fixed order so the same game seed always
/// produces the same tree. The definition is included verbatim from
/// `tech_tree_def.rs` so the node list can be edited without touching this file.
pub fn build_tech_tree(ids: &mut IdGenerator) -> (TechTree, TechRefs) {
    #[allow(unused_imports)]
    use OneShotEffect::*;
    let mut tree = TechTree::new();
    let refs = include!("tech_tree_def.rs");
    (tree, refs)
}

/// Build the game's full civic tree, returning the tree and named ID handles.
/// See `build_tech_tree` for the design notes.
pub fn build_civic_tree(ids: &mut IdGenerator) -> (CivicTree, CivicRefs) {
    #[allow(unused_imports)]
    use OneShotEffect::*;
    let mut tree = CivicTree::new();
    let refs = include!("civic_tree_def.rs");
    (tree, refs)
}

use crate::{CivicId, TechId};
use std::collections::HashMap;
use super::effect::OneShotEffect;

#[derive(Debug, Clone)]
pub struct TechNode {
    pub id:                 TechId,
    pub name:               &'static str,
    pub cost:               u32,
    pub prerequisites:      Vec<TechId>,
    /// Effects applied when this tech is completed.
    pub effects:            Vec<OneShotEffect>,
    pub eureka_description: &'static str,
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

use libcommon::{BuildingId, CivicId, TechId};
use std::collections::HashMap;

/// Something unlocked by completing a tech or civic.
#[derive(Debug, Clone)]
pub enum Unlock {
    Unit(&'static str),
    Building(&'static str),
    Improvement(&'static str),
    District(&'static str),
    Policy(&'static str),
    Government(&'static str),
    Resource(&'static str),
    Ability(&'static str),
}

/// Condition that grants Eureka/Inspiration boost.
pub trait EurekaCondition: std::fmt::Debug {
    fn description(&self) -> &'static str;
    fn is_met(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct TechNode {
    pub id: TechId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<TechId>,
    pub unlocks: Vec<Unlock>,
    pub eureka_description: &'static str,
}

#[derive(Debug, Clone)]
pub struct CivicNode {
    pub id: CivicId,
    pub name: &'static str,
    pub cost: u32,
    pub prerequisites: Vec<CivicId>,
    pub unlocks: Vec<Unlock>,
    pub inspiration_description: &'static str,
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

// Suppress unused import
const _: Option<BuildingId> = None;

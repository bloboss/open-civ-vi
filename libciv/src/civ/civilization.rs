use libcommon::{
    AgeType, CivId, CivicId, GovernmentId, PolicyId, TechId, YieldBundle,
};
use crate::rules::modifier::Modifier;

pub trait StartBias: std::fmt::Debug {
    fn terrain_preference(&self) -> Option<&'static str>;
    fn feature_preference(&self) -> Option<&'static str>;
    fn resource_preference(&self) -> Option<&'static str>;
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

#[derive(Debug, Clone)]
pub struct Leader {
    pub name: &'static str,
    pub civ_id: CivId,
    /// Names of leader ability implementations (resolved at runtime).
    pub ability_names: Vec<&'static str>,
    pub agenda_name: &'static str,
}

#[derive(Debug, Clone)]
pub struct Civilization {
    pub id: CivId,
    pub name: &'static str,
    pub adjective: &'static str,
    pub leader: Leader,
    pub cities: Vec<libcommon::CityId>,
    pub capital: Option<libcommon::CityId>,
    pub current_era: AgeType,
    pub researched_techs: Vec<TechId>,
    pub tech_in_progress: Option<TechProgress>,
    pub completed_civics: Vec<CivicId>,
    pub civic_in_progress: Option<CivicProgress>,
    pub current_government: Option<GovernmentId>,
    pub active_policies: Vec<PolicyId>,
    pub gold: i32,
    pub treasury_per_turn: i32,
    pub yields: YieldBundle,
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
        }
    }

    pub fn has_tech(&self, tech_id: TechId) -> bool {
        self.researched_techs.contains(&tech_id)
    }

    pub fn has_civic(&self, civic_id: CivicId) -> bool {
        self.completed_civics.contains(&civic_id)
    }
}

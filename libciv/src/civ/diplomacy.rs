use libcommon::{AgreementId, CivId, GrievanceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiplomaticStatus {
    War,
    ColdWar,
    Neutral,
    OpenBorders,
    Friendship,
    Alliance,
}

pub trait Agreement: std::fmt::Debug {
    fn id(&self) -> AgreementId;
    fn name(&self) -> &'static str;
    fn duration_turns(&self) -> Option<u32>;
    fn is_expired(&self, current_turn: u32, signed_turn: u32) -> bool {
        match self.duration_turns() {
            None => false,
            Some(dur) => current_turn >= signed_turn + dur,
        }
    }
}

pub trait GrievanceTrigger: std::fmt::Debug {
    fn description(&self) -> &'static str;
    fn grievance_amount(&self) -> i32;
}

#[derive(Debug, Clone)]
pub struct DiplomaticRelation {
    pub civ_a: CivId,
    pub civ_b: CivId,
    pub status: DiplomaticStatus,
    pub grievances_a_against_b: i32,
    pub grievances_b_against_a: i32,
    pub active_agreements: Vec<AgreementId>,
    pub turns_at_war: u32,
}

impl DiplomaticRelation {
    pub fn new(civ_a: CivId, civ_b: CivId) -> Self {
        Self {
            civ_a,
            civ_b,
            status: DiplomaticStatus::Neutral,
            grievances_a_against_b: 0,
            grievances_b_against_a: 0,
            active_agreements: Vec::new(),
            turns_at_war: 0,
        }
    }

    pub fn is_at_war(&self) -> bool {
        self.status == DiplomaticStatus::War
    }

    pub fn add_grievance(&mut self, by: CivId, amount: i32) {
        if by == self.civ_a {
            self.grievances_a_against_b += amount;
        } else if by == self.civ_b {
            self.grievances_b_against_a += amount;
        }
    }
}

// Built-in grievance triggers
#[derive(Debug, Clone, Copy, Default)]
pub struct DeclaredWarGrievance;
impl GrievanceTrigger for DeclaredWarGrievance {
    fn description(&self) -> &'static str { "Declared war" }
    fn grievance_amount(&self) -> i32 { 30 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PillageGrievance;
impl GrievanceTrigger for PillageGrievance {
    fn description(&self) -> &'static str { "Pillaged improvement" }
    fn grievance_amount(&self) -> i32 { 5 }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CapturedCityGrievance;
impl GrievanceTrigger for CapturedCityGrievance {
    fn description(&self) -> &'static str { "Captured city" }
    fn grievance_amount(&self) -> i32 { 20 }
}

// Suppress unused
const _: Option<GrievanceId> = None;

use crate::{AgreementId, CivId, GrievanceId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum DiplomaticStatus {
    War,
    Denounced,
    Neutral,
    Friendly,
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
    fn visibility(&self) -> GrievanceVisibility { GrievanceVisibility::Public }
}

/// Who can see a grievance in the diplomacy screen.
// TODO: This should be arranged in levels:
// - Public
// - Hidden(u8) // we can have various levels of spy
// - Secret
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GrievanceVisibility {
    /// All civilizations can see this grievance.
    Public,
    /// Only visible if the observing civ has a spy in the relevant city.
    RequiresSpy,
    /// Only visible to alliance members.
    RequiresAlliance,
}

/// A single recorded grievance event between two civilizations.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "")))]
pub struct GrievanceRecord {
    pub grievance_id: GrievanceId,
    #[cfg_attr(feature = "serde", serde(with = "crate::serde_static_str"))]
    pub description: &'static str,
    pub amount: i32,
    pub visibility: GrievanceVisibility,
    pub recorded_turn: u32,
}

// TODO: Any way we can just track this as a bi-directional graph? Is there
// a nicer way to do that?
#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DiplomaticRelation {
    pub civ_a: CivId,
    pub civ_b: CivId,
    pub status: DiplomaticStatus,
    /// Skipped during serialization — grievance records contain `&'static str`
    /// descriptions that require special handling. Grievances are transient
    /// per-era data; on load they start empty.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub grievances_a_against_b: Vec<GrievanceRecord>,
    #[cfg_attr(feature = "serde", serde(skip))]
    pub grievances_b_against_a: Vec<GrievanceRecord>,
    pub active_agreements: Vec<AgreementId>,
    pub turns_at_war: u32,
}

impl DiplomaticRelation {
    pub fn new(civ_a: CivId, civ_b: CivId) -> Self {
        Self {
            civ_a,
            civ_b,
            status: DiplomaticStatus::Neutral,
            grievances_a_against_b: Vec::new(),
            grievances_b_against_a: Vec::new(),
            active_agreements: Vec::new(),
            turns_at_war: 0,
        }
    }

    pub fn is_at_war(&self) -> bool {
        self.status == DiplomaticStatus::War
    }

    pub fn add_grievance(&mut self, by: CivId, record: GrievanceRecord) {
        if by == self.civ_a {
            self.grievances_a_against_b.push(record);
        } else if by == self.civ_b {
            self.grievances_b_against_a.push(record);
        }
    }

    /// Net opinion score of civ_a toward civ_b (negative = hostile).
    pub fn opinion_score_a_toward_b(&self) -> i32 {
        self.grievances_a_against_b.iter().map(|g| -g.amount).sum()
    }

    /// Net opinion score of civ_b toward civ_a (negative = hostile).
    pub fn opinion_score_b_toward_a(&self) -> i32 {
        self.grievances_b_against_a.iter().map(|g| -g.amount).sum()
    }
}


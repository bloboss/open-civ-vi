use libcommon::{GovernmentId, PolicyId, PolicyType};

use super::modifier::Modifier;

#[derive(Debug, Clone)]
pub struct PolicySlots {
    pub military: u8,
    pub economic: u8,
    pub diplomatic: u8,
    pub wildcard: u8,
}

impl PolicySlots {
    pub fn total(&self) -> u8 {
        self.military + self.economic + self.diplomatic + self.wildcard
    }
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub id: PolicyId,
    pub name: &'static str,
    pub policy_type: PolicyType,
    pub modifiers: Vec<Modifier>,
    pub maintenance: u32,
}

#[derive(Debug, Clone)]
pub struct Government {
    pub id: GovernmentId,
    pub name: &'static str,
    pub slots: PolicySlots,
    pub inherent_modifiers: Vec<Modifier>,
    pub legacy_bonus: Option<&'static str>,
}

impl Government {
    pub fn can_slot_policy(&self, policy: &Policy) -> bool {
        match policy.policy_type {
            PolicyType::Military => self.slots.military > 0,
            PolicyType::Economic => self.slots.economic > 0,
            PolicyType::Diplomatic => self.slots.diplomatic > 0,
            PolicyType::Wildcard => self.slots.wildcard > 0,
        }
    }
}

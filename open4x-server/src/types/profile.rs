use serde::{Deserialize, Serialize};

use super::ids::CivTemplateId;

/// A player profile: identity + civ selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileView {
    /// Ed25519 public key bytes (32 bytes).
    pub pubkey: Vec<u8>,
    /// User-chosen display name.
    pub display_name: String,
    /// The civ template this player has selected.
    pub selected_template: CivTemplateId,
}

/// A predefined civilization template providing bonuses and flavour.
///
/// Replaces the old Leader/Agenda system. Users select a template and provide
/// their own display name via their profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CivTemplate {
    pub id: CivTemplateId,
    pub civ_name: String,
    pub adjective: String,
    /// The default leader name for this civilization.
    pub leader_name: String,
    /// Human-readable description of the civ's unique ability.
    pub ability_description: String,
    /// Name of the unique unit (if any).
    pub unique_unit: Option<String>,
    /// Name of the unique district/building (if any).
    pub unique_infrastructure: Option<String>,
}

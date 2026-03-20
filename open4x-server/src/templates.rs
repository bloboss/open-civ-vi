//! Predefined civilization templates.

use open4x_api::ids::CivTemplateId;
use open4x_api::profile::CivTemplate;
use ulid::Ulid;

/// Return the list of built-in civ templates.
pub fn builtin_templates() -> Vec<CivTemplate> {
    vec![
        CivTemplate {
            id: CivTemplateId::from_ulid(Ulid::from_parts(0, 1)),
            civ_name: "Rome".into(),
            adjective: "Roman".into(),
            leader_name: "Caesar".into(),
            ability_description: "All roads lead to Rome: Trade routes generate +1 gold.".into(),
            unique_unit: Some("Legion".into()),
            unique_infrastructure: Some("Bath".into()),
        },
        CivTemplate {
            id: CivTemplateId::from_ulid(Ulid::from_parts(0, 2)),
            civ_name: "Babylon".into(),
            adjective: "Babylonian".into(),
            leader_name: "Hammurabi".into(),
            ability_description: "Enuma Anu Enlil: Eurekas grant full tech instead of boost.".into(),
            unique_unit: Some("Sabum Kibittum".into()),
            unique_infrastructure: Some("Palgum".into()),
        },
        CivTemplate {
            id: CivTemplateId::from_ulid(Ulid::from_parts(0, 3)),
            civ_name: "Greece".into(),
            adjective: "Greek".into(),
            leader_name: "Pericles".into(),
            ability_description: "Plato's Republic: +1 wildcard policy slot.".into(),
            unique_unit: Some("Hoplite".into()),
            unique_infrastructure: Some("Acropolis".into()),
        },
        CivTemplate {
            id: CivTemplateId::from_ulid(Ulid::from_parts(0, 4)),
            civ_name: "Egypt".into(),
            adjective: "Egyptian".into(),
            leader_name: "Cleopatra".into(),
            ability_description: "Iteru: +15% production for wonders on rivers.".into(),
            unique_unit: Some("Maryannu Chariot Archer".into()),
            unique_infrastructure: Some("Sphinx".into()),
        },
    ]
}

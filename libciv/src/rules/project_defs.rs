//! Built-in city project definitions.

use crate::game::state::{IdGenerator, ProjectDef};

/// Register all built-in projects. Returns a Vec of `ProjectDef`.
pub fn builtin_project_defs(id_gen: &mut IdGenerator) -> Vec<ProjectDef> {
    vec![
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Launch Earth Satellite",
            production_cost: 1500,
            requires_district: Some("Spaceport"),
            repeatable: false,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Launch Moon Landing",
            production_cost: 1800,
            requires_district: Some("Spaceport"),
            repeatable: false,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Launch Mars Colony",
            production_cost: 2000,
            requires_district: Some("Spaceport"),
            repeatable: false,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Exoplanet Expedition",
            production_cost: 2400,
            requires_district: Some("Spaceport"),
            repeatable: false,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Carbon Recapture",
            production_cost: 400,
            requires_district: Some("Industrial Zone"),
            repeatable: true,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Bread and Circuses",
            production_cost: 100,
            requires_district: Some("Entertainment Complex"),
            repeatable: true,
        },
        ProjectDef {
            id: id_gen.next_project_id(),
            name: "Campus Research Grants",
            production_cost: 200,
            requires_district: Some("Campus"),
            repeatable: true,
        },
    ]
}

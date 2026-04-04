use crate::game::state::WonderDef;
use crate::game::IdGenerator;
use crate::{AgeType, WonderId};

/// Returns all 29 base-game world wonder definitions.
pub fn builtin_wonder_defs(id_gen: &mut IdGenerator) -> Vec<WonderDef> {
    vec![
        // ── Ancient Era (3) ───────────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Stonehenge",
            production_cost: 180,
            era: Some(AgeType::Ancient),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Hanging Gardens",
            production_cost: 180,
            era: Some(AgeType::Ancient),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Pyramids",
            production_cost: 220,
            era: Some(AgeType::Ancient),
        },
        // ── Classical Era (8) ─────────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Oracle",
            production_cost: 290,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Great Lighthouse",
            production_cost: 290,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Colossus",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Petra",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Colosseum",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Great Library",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Mahabodhi Temple",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Terracotta Army",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        // ── Medieval Era (4) ──────────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Hagia Sophia",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Alhambra",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Chichen Itza",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Mont St. Michel",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        // ── Renaissance Era (4) ───────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Venetian Arsenal",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Great Zimbabwe",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Forbidden City",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Potala Palace",
            production_cost: 1060,
            era: Some(AgeType::Renaissance),
        },
        // ── Industrial Era (3) ────────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Ruhr Valley",
            production_cost: 1240,
            era: Some(AgeType::Industrial),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Bolshoi Theatre",
            production_cost: 1240,
            era: Some(AgeType::Industrial),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Oxford University",
            production_cost: 1240,
            era: Some(AgeType::Industrial),
        },
        // ── Modern Era (5) ────────────────────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Big Ben",
            production_cost: 1450,
            era: Some(AgeType::Modern),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Hermitage",
            production_cost: 1450,
            era: Some(AgeType::Modern),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Eiffel Tower",
            production_cost: 1620,
            era: Some(AgeType::Modern),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Broadway",
            production_cost: 1620,
            era: Some(AgeType::Modern),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Cristo Redentor",
            production_cost: 1620,
            era: Some(AgeType::Modern),
        },
        // ── Atomic / Information Era (2) ──────────────────────────────────
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Estadio do Maracana",
            production_cost: 1740,
            era: Some(AgeType::Atomic),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Sydney Opera House",
            production_cost: 1850,
            era: Some(AgeType::Information),
        },

        // ── Rise & Fall wonders ─────────────────────────────────────────────

        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Temple of Artemis",
            production_cost: 180,
            era: Some(AgeType::Ancient),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Kilwa Kisiwani",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Kotoku-in",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Casa de Contratacion",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "St. Basil's Cathedral",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Taj Mahal",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Statue of Liberty",
            production_cost: 1240,
            era: Some(AgeType::Industrial),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Amundsen-Scott Research Station",
            production_cost: 1620,
            era: Some(AgeType::Atomic),
        },

        // ── Gathering Storm wonders ─────────────────────────────────────────

        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Great Bath",
            production_cost: 180,
            era: Some(AgeType::Ancient),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Machu Picchu",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Meenakshi Temple",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "University of Sankore",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Orszaghaz",
            production_cost: 920,
            era: Some(AgeType::Industrial),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Panama Canal",
            production_cost: 920,
            era: Some(AgeType::Industrial),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Golden Gate Bridge",
            production_cost: 1620,
            era: Some(AgeType::Modern),
        },

        // ── DLC wonders ─────────────────────────────────────────────────────

        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Etemenanki",
            production_cost: 220,
            era: Some(AgeType::Ancient),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Statue of Zeus",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Apadana",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Mausoleum at Halicarnassus",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Jebel Barkal",
            production_cost: 400,
            era: Some(AgeType::Classical),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Huey Teocalli",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Angkor Wat",
            production_cost: 710,
            era: Some(AgeType::Medieval),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Torre de Belem",
            production_cost: 920,
            era: Some(AgeType::Renaissance),
        },
        WonderDef {
            id: WonderId::from_ulid(id_gen.next_ulid()),
            name: "Biosphere",
            production_cost: 1740,
            era: Some(AgeType::Atomic),
        },
    ]
}

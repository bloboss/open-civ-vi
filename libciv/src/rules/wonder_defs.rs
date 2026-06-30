use crate::game::state::WonderDef;
use crate::game::IdGenerator;
use crate::rules::modifier::{
    EffectType, Modifier, ModifierSource, StackingRule, TargetSelector,
};
use crate::{AgeType, WonderId, YieldType};

/// Convenience constructor for an unconditional, civ-wide flat yield modifier
/// granted by a completed wonder.
fn wonder_yield(name: &'static str, yt: YieldType, amount: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Wonder(name),
        TargetSelector::Global,
        EffectType::YieldFlat(yt, amount),
        StackingRule::Additive,
    )
}

/// Convenience constructor for an unconditional, civ-wide percentage yield
/// modifier granted by a completed wonder.
fn wonder_yield_pct(name: &'static str, yt: YieldType, pct: i32) -> Modifier {
    Modifier::new(
        ModifierSource::Wonder(name),
        TargetSelector::Global,
        EffectType::YieldPercent(yt, pct),
        StackingRule::Additive,
    )
}

/// Persistent yield/modifier effects for a handful of iconic wonders. Wonders
/// not listed here grant no economic effect yet (their `effects` stay empty).
fn builtin_wonder_effects(name: &'static str) -> Vec<Modifier> {
    match name {
        // +2 Faith — the classic ancient faith wonder.
        "Stonehenge" => vec![wonder_yield("Stonehenge", YieldType::Faith, 2)],
        // +2 Culture (free Builder is not modelled here).
        "Pyramids" => vec![wonder_yield("Pyramids", YieldType::Culture, 2)],
        // +15% Food growth across the empire.
        "Hanging Gardens" => vec![wonder_yield_pct("Hanging Gardens", YieldType::Food, 15)],
        // +1 Faith and +1 Culture.
        "Oracle" => vec![
            wonder_yield("Oracle", YieldType::Faith, 1),
            wonder_yield("Oracle", YieldType::Culture, 1),
        ],
        // +3 Gold from increased trade capacity.
        "Colossus" => vec![wonder_yield("Colossus", YieldType::Gold, 3)],
        // Petra's oasis: +2 Food, +2 Gold.
        "Petra" => vec![
            wonder_yield("Petra", YieldType::Food, 2),
            wonder_yield("Petra", YieldType::Gold, 2),
        ],
        // +4 Science from the great repository.
        "Great Library" => vec![wonder_yield("Great Library", YieldType::Science, 4)],
        // +6 Gold from the seat of commerce.
        "Big Ben" => vec![wonder_yield("Big Ben", YieldType::Gold, 6)],
        _ => Vec::new(),
    }
}

/// Returns all 29 base-game world wonder definitions.
pub fn builtin_wonder_defs(id_gen: &mut IdGenerator) -> Vec<WonderDef> {
    // Closure builds a wonder with an empty effects vec; effects are attached
    // in a second pass below so the iconic-wonder data stays in one place.
    let mut w = |name: &'static str, production_cost: u32, era: AgeType| WonderDef {
        id: WonderId::from_ulid(id_gen.next_ulid()),
        name,
        production_cost,
        era: Some(era),
        effects: Vec::new(),
    };

    let mut defs = vec![
        // ── Ancient Era (3) ───────────────────────────────────────────────
        w("Stonehenge", 180, AgeType::Ancient),
        w("Hanging Gardens", 180, AgeType::Ancient),
        w("Pyramids", 220, AgeType::Ancient),
        // ── Classical Era (8) ─────────────────────────────────────────────
        w("Oracle", 290, AgeType::Classical),
        w("Great Lighthouse", 290, AgeType::Classical),
        w("Colossus", 400, AgeType::Classical),
        w("Petra", 400, AgeType::Classical),
        w("Colosseum", 400, AgeType::Classical),
        w("Great Library", 400, AgeType::Classical),
        w("Mahabodhi Temple", 400, AgeType::Classical),
        w("Terracotta Army", 400, AgeType::Classical),
        // ── Medieval Era (4) ──────────────────────────────────────────────
        w("Hagia Sophia", 710, AgeType::Medieval),
        w("Alhambra", 710, AgeType::Medieval),
        w("Chichen Itza", 710, AgeType::Medieval),
        w("Mont St. Michel", 710, AgeType::Medieval),
        // ── Renaissance Era (4) ───────────────────────────────────────────
        w("Venetian Arsenal", 920, AgeType::Renaissance),
        w("Great Zimbabwe", 920, AgeType::Renaissance),
        w("Forbidden City", 920, AgeType::Renaissance),
        w("Potala Palace", 1060, AgeType::Renaissance),
        // ── Industrial Era (3) ────────────────────────────────────────────
        w("Ruhr Valley", 1240, AgeType::Industrial),
        w("Bolshoi Theatre", 1240, AgeType::Industrial),
        w("Oxford University", 1240, AgeType::Industrial),
        // ── Modern Era (5) ────────────────────────────────────────────────
        w("Big Ben", 1450, AgeType::Modern),
        w("Hermitage", 1450, AgeType::Modern),
        w("Eiffel Tower", 1620, AgeType::Modern),
        w("Broadway", 1620, AgeType::Modern),
        w("Cristo Redentor", 1620, AgeType::Modern),
        // ── Atomic / Information Era (2) ──────────────────────────────────
        w("Estadio do Maracana", 1740, AgeType::Atomic),
        w("Sydney Opera House", 1850, AgeType::Information),

        // ── Rise & Fall wonders ─────────────────────────────────────────────
        w("Temple of Artemis", 180, AgeType::Ancient),
        w("Kilwa Kisiwani", 710, AgeType::Medieval),
        w("Kotoku-in", 710, AgeType::Medieval),
        w("Casa de Contratacion", 920, AgeType::Renaissance),
        w("St. Basil's Cathedral", 920, AgeType::Renaissance),
        w("Taj Mahal", 920, AgeType::Renaissance),
        w("Statue of Liberty", 1240, AgeType::Industrial),
        w("Amundsen-Scott Research Station", 1620, AgeType::Atomic),

        // ── Gathering Storm wonders ─────────────────────────────────────────
        w("Great Bath", 180, AgeType::Ancient),
        w("Machu Picchu", 400, AgeType::Classical),
        w("Meenakshi Temple", 710, AgeType::Medieval),
        w("University of Sankore", 710, AgeType::Medieval),
        w("Orszaghaz", 920, AgeType::Industrial),
        w("Panama Canal", 920, AgeType::Industrial),
        w("Golden Gate Bridge", 1620, AgeType::Modern),

        // ── DLC wonders ─────────────────────────────────────────────────────
        w("Etemenanki", 220, AgeType::Ancient),
        w("Statue of Zeus", 400, AgeType::Classical),
        w("Apadana", 400, AgeType::Classical),
        w("Mausoleum at Halicarnassus", 400, AgeType::Classical),
        w("Jebel Barkal", 400, AgeType::Classical),
        w("Huey Teocalli", 710, AgeType::Medieval),
        w("Angkor Wat", 710, AgeType::Medieval),
        w("Torre de Belem", 920, AgeType::Renaissance),
        w("Biosphere", 1740, AgeType::Atomic),
    ];

    // Attach iconic-wonder effects in a single pass.
    for def in defs.iter_mut() {
        def.effects = builtin_wonder_effects(def.name);
    }

    defs
}

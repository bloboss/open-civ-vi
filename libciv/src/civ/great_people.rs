use crate::{CivId, GreatPersonId, GreatPersonType, UnitDomain};
use crate::civ::district::BuiltinDistrict;
use libhexgrid::coord::HexCoord;

// ── Great person point constants ─────────────────────────────────────────────

/// Points generated per district per turn.
pub const GP_BASE_POINTS_PER_DISTRICT: u32 = 1;

/// Base recruitment threshold for the first great person of each type.
pub const GP_BASE_THRESHOLD: u32 = 60;

/// Additional threshold per previously recruited great person of the same type.
pub const GP_THRESHOLD_INCREMENT: u32 = 60;

/// Gold cost per missing point when patronizing a great person.
pub const GP_PATRONAGE_GOLD_PER_POINT: u32 = 3;

/// Faith cost per missing point when patronizing a Great Prophet.
pub const GP_PATRONAGE_FAITH_PER_POINT: u32 = 2;

/// Maps a district to the great person type(s) it generates points for.
///
/// TheaterSquare generates points for Writer, Artist, and Musician (the caller
/// picks one via round-robin). Most other districts map to a single type.
/// Returns an empty slice for districts that don't generate great person points.
pub fn district_great_person_types(district: BuiltinDistrict) -> &'static [GreatPersonType] {
    match district {
        BuiltinDistrict::Campus          => &[GreatPersonType::Scientist],
        BuiltinDistrict::TheaterSquare   => &[GreatPersonType::Writer, GreatPersonType::Artist, GreatPersonType::Musician],
        BuiltinDistrict::CommercialHub   => &[GreatPersonType::Merchant],
        BuiltinDistrict::Harbor          => &[GreatPersonType::Admiral],
        BuiltinDistrict::HolySite        => &[GreatPersonType::Prophet],
        BuiltinDistrict::Encampment      => &[GreatPersonType::General],
        BuiltinDistrict::IndustrialZone  => &[GreatPersonType::Engineer],
        _ => &[],
    }
}

/// Era ordering for gating great person candidates.
fn era_order(era: &str) -> u32 {
    match era {
        "Ancient"      => 0,
        "Classical"    => 1,
        "Medieval"     => 2,
        "Renaissance"  => 3,
        "Industrial"   => 4,
        "Modern"       => 5,
        "Atomic"       => 6,
        "Information"  => 7,
        "Future"       => 8,
        _              => u32::MAX,
    }
}

/// Returns true if `candidate_era` is the same as or earlier than `current_era`.
pub fn era_is_current_or_earlier(candidate_era: &str, current_era: &str) -> bool {
    era_order(candidate_era) <= era_order(current_era)
}

/// Returns the current global era name from the game state's era list.
pub fn current_era_name(state: &crate::game::state::GameState) -> &'static str {
    state.eras.get(state.current_era_index)
        .map(|e| e.name)
        .unwrap_or("Ancient")
}

/// Compute the recruitment threshold for a given GP type based on how many
/// of that type have already been recruited globally.
pub fn recruitment_threshold(person_type: GreatPersonType, state: &crate::game::state::GameState) -> u32 {
    let recruited_count = state.great_people.iter()
        .filter(|gp| gp.person_type == person_type && gp.owner.is_some())
        .count() as u32;
    GP_BASE_THRESHOLD + recruited_count * GP_THRESHOLD_INCREMENT
}

/// Find the name of the next available (unrecruited, era-gated) candidate
/// of the given type, or `None` if the pool is exhausted.
pub fn next_candidate_name(
    person_type: GreatPersonType,
    state: &crate::game::state::GameState,
) -> Option<&'static str> {
    let era_name = current_era_name(state);
    let recruited_names: std::collections::HashSet<&str> = state.great_people.iter()
        .filter(|gp| gp.owner.is_some())
        .map(|gp| gp.name)
        .collect();

    state.great_person_defs.iter()
        .filter(|d| d.person_type == person_type)
        .filter(|d| !recruited_names.contains(d.name))
        .filter(|d| era_is_current_or_earlier(d.era, era_name))
        .map(|d| d.name)
        .next()
}

pub trait GreatPersonAbility: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Number of uses (None = unlimited / passive).
    fn uses(&self) -> Option<u32>;
}

/// The effect applied when a great person is retired (consumed).
#[derive(Debug, Clone)]
pub enum RetireEffect {
    /// Grant a permanent combat strength bonus to all units of the given domain.
    CombatStrengthBonus { domain: UnitDomain, bonus: i32 },
    /// Instantly add production to the nearest owned city's current queue item.
    ProductionBurst { amount: u32 },
    /// Instantly grant gold to the owning civilization.
    GoldGrant { amount: u32 },
}

/// Static definition of a great person type, analogous to `UnitTypeDef`.
#[derive(Debug, Clone)]
pub struct GreatPersonDef {
    pub name: &'static str,
    pub person_type: GreatPersonType,
    pub era: &'static str,
    pub retire_effect: RetireEffect,
}

/// Returns the built-in great person definitions across multiple types and eras.
pub fn builtin_great_person_defs() -> Vec<GreatPersonDef> {
    vec![
        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Sun Tzu",
            person_type: GreatPersonType::General,
            era: "Ancient",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 5,
            },
        },
        GreatPersonDef {
            name: "Boudica",
            person_type: GreatPersonType::General,
            era: "Classical",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 5,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Themistocles",
            person_type: GreatPersonType::Admiral,
            era: "Ancient",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 5,
            },
        },
        GreatPersonDef {
            name: "Artemisia",
            person_type: GreatPersonType::Admiral,
            era: "Classical",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 5,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Imhotep",
            person_type: GreatPersonType::Engineer,
            era: "Ancient",
            retire_effect: RetireEffect::ProductionBurst { amount: 200 },
        },
        GreatPersonDef {
            name: "Bi Sheng",
            person_type: GreatPersonType::Engineer,
            era: "Classical",
            retire_effect: RetireEffect::ProductionBurst { amount: 250 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Marco Polo",
            person_type: GreatPersonType::Merchant,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
        },
        GreatPersonDef {
            name: "Colaeus",
            person_type: GreatPersonType::Merchant,
            era: "Classical",
            retire_effect: RetireEffect::GoldGrant { amount: 250 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Euclid",
            person_type: GreatPersonType::Scientist,
            era: "Ancient",
            retire_effect: RetireEffect::ProductionBurst { amount: 150 },
        },
        GreatPersonDef {
            name: "Hypatia",
            person_type: GreatPersonType::Scientist,
            era: "Classical",
            retire_effect: RetireEffect::ProductionBurst { amount: 200 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Homer",
            person_type: GreatPersonType::Writer,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 150 },
        },
        GreatPersonDef {
            name: "Ovid",
            person_type: GreatPersonType::Writer,
            era: "Classical",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Phidias",
            person_type: GreatPersonType::Artist,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 150 },
        },
        GreatPersonDef {
            name: "Andrei Rublev",
            person_type: GreatPersonType::Artist,
            era: "Classical",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Orpheus",
            person_type: GreatPersonType::Musician,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 150 },
        },
        GreatPersonDef {
            name: "Pindar",
            person_type: GreatPersonType::Musician,
            era: "Classical",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
        },
        // ── Great Prophets ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Confucius",
            person_type: GreatPersonType::Prophet,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
        },
        GreatPersonDef {
            name: "Adi Shankara",
            person_type: GreatPersonType::Prophet,
            era: "Classical",
            retire_effect: RetireEffect::GoldGrant { amount: 250 },
        },
        // ══════════════════════════════════════════════════════════════════════
        // Medieval era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "El Cid",
            person_type: GreatPersonType::General,
            era: "Medieval",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 7,
            },
        },
        GreatPersonDef {
            name: "Yi Sun-sin",
            person_type: GreatPersonType::General,
            era: "Medieval",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 7,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Zheng He",
            person_type: GreatPersonType::Admiral,
            era: "Medieval",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 7,
            },
        },
        GreatPersonDef {
            name: "Leif Erikson",
            person_type: GreatPersonType::Admiral,
            era: "Medieval",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 7,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Isidore of Miletus",
            person_type: GreatPersonType::Engineer,
            era: "Medieval",
            retire_effect: RetireEffect::ProductionBurst { amount: 300 },
        },
        GreatPersonDef {
            name: "Filippo Brunelleschi",
            person_type: GreatPersonType::Engineer,
            era: "Medieval",
            retire_effect: RetireEffect::ProductionBurst { amount: 350 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Ibn Fadlan",
            person_type: GreatPersonType::Merchant,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 300 },
        },
        GreatPersonDef {
            name: "Jakob Fugger",
            person_type: GreatPersonType::Merchant,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 350 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Al-Khwarizmi",
            person_type: GreatPersonType::Scientist,
            era: "Medieval",
            retire_effect: RetireEffect::ProductionBurst { amount: 250 },
        },
        GreatPersonDef {
            name: "Omar Khayyam",
            person_type: GreatPersonType::Scientist,
            era: "Medieval",
            retire_effect: RetireEffect::ProductionBurst { amount: 300 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Murasaki Shikibu",
            person_type: GreatPersonType::Writer,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 250 },
        },
        GreatPersonDef {
            name: "Geoffrey Chaucer",
            person_type: GreatPersonType::Writer,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 300 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Giotto",
            person_type: GreatPersonType::Artist,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 250 },
        },
        GreatPersonDef {
            name: "Donatello",
            person_type: GreatPersonType::Artist,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 300 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Hildegard of Bingen",
            person_type: GreatPersonType::Musician,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 250 },
        },
        GreatPersonDef {
            name: "Guillaume de Machaut",
            person_type: GreatPersonType::Musician,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 300 },
        },
        // ── Great Prophets ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Thomas Aquinas",
            person_type: GreatPersonType::Prophet,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 300 },
        },
        GreatPersonDef {
            name: "Francis of Assisi",
            person_type: GreatPersonType::Prophet,
            era: "Medieval",
            retire_effect: RetireEffect::GoldGrant { amount: 350 },
        },

        // ══════════════════════════════════════════════════════════════════════
        // Renaissance era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Gustavus Adolphus",
            person_type: GreatPersonType::General,
            era: "Renaissance",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 8,
            },
        },
        GreatPersonDef {
            name: "Oda Nobunaga",
            person_type: GreatPersonType::General,
            era: "Renaissance",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 8,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Francis Drake",
            person_type: GreatPersonType::Admiral,
            era: "Renaissance",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 8,
            },
        },
        GreatPersonDef {
            name: "Santa Cruz",
            person_type: GreatPersonType::Admiral,
            era: "Renaissance",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 8,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Leonardo da Vinci",
            person_type: GreatPersonType::Engineer,
            era: "Renaissance",
            retire_effect: RetireEffect::ProductionBurst { amount: 400 },
        },
        GreatPersonDef {
            name: "Mimar Sinan",
            person_type: GreatPersonType::Engineer,
            era: "Renaissance",
            retire_effect: RetireEffect::ProductionBurst { amount: 450 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Giovanni de Medici",
            person_type: GreatPersonType::Merchant,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 400 },
        },
        GreatPersonDef {
            name: "Raja Todar Mal",
            person_type: GreatPersonType::Merchant,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 450 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Galileo Galilei",
            person_type: GreatPersonType::Scientist,
            era: "Renaissance",
            retire_effect: RetireEffect::ProductionBurst { amount: 350 },
        },
        GreatPersonDef {
            name: "Isaac Newton",
            person_type: GreatPersonType::Scientist,
            era: "Renaissance",
            retire_effect: RetireEffect::ProductionBurst { amount: 400 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "William Shakespeare",
            person_type: GreatPersonType::Writer,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 350 },
        },
        GreatPersonDef {
            name: "Miguel de Cervantes",
            person_type: GreatPersonType::Writer,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 400 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Michelangelo",
            person_type: GreatPersonType::Artist,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 350 },
        },
        GreatPersonDef {
            name: "Albrecht Durer",
            person_type: GreatPersonType::Artist,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 400 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Giovanni Pierluigi da Palestrina",
            person_type: GreatPersonType::Musician,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 350 },
        },
        GreatPersonDef {
            name: "Antonio Vivaldi",
            person_type: GreatPersonType::Musician,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 400 },
        },
        // ── Great Prophets ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Martin Luther",
            person_type: GreatPersonType::Prophet,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 400 },
        },
        GreatPersonDef {
            name: "Guru Nanak",
            person_type: GreatPersonType::Prophet,
            era: "Renaissance",
            retire_effect: RetireEffect::GoldGrant { amount: 450 },
        },

        // ══════════════════════════════════════════════════════════════════════
        // Industrial era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Napoleon Bonaparte",
            person_type: GreatPersonType::General,
            era: "Industrial",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 10,
            },
        },
        GreatPersonDef {
            name: "Simón Bolívar",
            person_type: GreatPersonType::General,
            era: "Industrial",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 10,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Horatio Nelson",
            person_type: GreatPersonType::Admiral,
            era: "Industrial",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 10,
            },
        },
        GreatPersonDef {
            name: "Laskarina Bouboulina",
            person_type: GreatPersonType::Admiral,
            era: "Industrial",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 10,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "James Watt",
            person_type: GreatPersonType::Engineer,
            era: "Industrial",
            retire_effect: RetireEffect::ProductionBurst { amount: 500 },
        },
        GreatPersonDef {
            name: "Nikola Tesla",
            person_type: GreatPersonType::Engineer,
            era: "Industrial",
            retire_effect: RetireEffect::ProductionBurst { amount: 550 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Adam Smith",
            person_type: GreatPersonType::Merchant,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 500 },
        },
        GreatPersonDef {
            name: "John Jacob Astor",
            person_type: GreatPersonType::Merchant,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 550 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Charles Darwin",
            person_type: GreatPersonType::Scientist,
            era: "Industrial",
            retire_effect: RetireEffect::ProductionBurst { amount: 500 },
        },
        GreatPersonDef {
            name: "Dmitri Mendeleev",
            person_type: GreatPersonType::Scientist,
            era: "Industrial",
            retire_effect: RetireEffect::ProductionBurst { amount: 550 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Jane Austen",
            person_type: GreatPersonType::Writer,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 500 },
        },
        GreatPersonDef {
            name: "Mark Twain",
            person_type: GreatPersonType::Writer,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 550 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Claude Monet",
            person_type: GreatPersonType::Artist,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 500 },
        },
        GreatPersonDef {
            name: "Vincent van Gogh",
            person_type: GreatPersonType::Artist,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 550 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Ludwig van Beethoven",
            person_type: GreatPersonType::Musician,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 500 },
        },
        GreatPersonDef {
            name: "Johann Sebastian Bach",
            person_type: GreatPersonType::Musician,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 550 },
        },
        // ── Great Prophets ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Bahá'u'lláh",
            person_type: GreatPersonType::Prophet,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 500 },
        },
        GreatPersonDef {
            name: "Hong Xiuquan",
            person_type: GreatPersonType::Prophet,
            era: "Industrial",
            retire_effect: RetireEffect::GoldGrant { amount: 550 },
        },

        // ══════════════════════════════════════════════════════════════════════
        // Modern era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Dwight D. Eisenhower",
            person_type: GreatPersonType::General,
            era: "Modern",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 12,
            },
        },
        GreatPersonDef {
            name: "Douglas MacArthur",
            person_type: GreatPersonType::General,
            era: "Modern",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 12,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Chester Nimitz",
            person_type: GreatPersonType::Admiral,
            era: "Modern",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 12,
            },
        },
        GreatPersonDef {
            name: "Sergei Gorshkov",
            person_type: GreatPersonType::Admiral,
            era: "Modern",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 12,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Robert Goddard",
            person_type: GreatPersonType::Engineer,
            era: "Modern",
            retire_effect: RetireEffect::ProductionBurst { amount: 600 },
        },
        GreatPersonDef {
            name: "Wernher von Braun",
            person_type: GreatPersonType::Engineer,
            era: "Modern",
            retire_effect: RetireEffect::ProductionBurst { amount: 650 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Helena Rubinstein",
            person_type: GreatPersonType::Merchant,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 600 },
        },
        GreatPersonDef {
            name: "Melitta Bentz",
            person_type: GreatPersonType::Merchant,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 650 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Albert Einstein",
            person_type: GreatPersonType::Scientist,
            era: "Modern",
            retire_effect: RetireEffect::ProductionBurst { amount: 600 },
        },
        GreatPersonDef {
            name: "Marie Curie",
            person_type: GreatPersonType::Scientist,
            era: "Modern",
            retire_effect: RetireEffect::ProductionBurst { amount: 650 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "F. Scott Fitzgerald",
            person_type: GreatPersonType::Writer,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 600 },
        },
        GreatPersonDef {
            name: "Karel Čapek",
            person_type: GreatPersonType::Writer,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 650 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Pablo Picasso",
            person_type: GreatPersonType::Artist,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 600 },
        },
        GreatPersonDef {
            name: "Frida Kahlo",
            person_type: GreatPersonType::Artist,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 650 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Duke Ellington",
            person_type: GreatPersonType::Musician,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 600 },
        },
        GreatPersonDef {
            name: "Billie Holiday",
            person_type: GreatPersonType::Musician,
            era: "Modern",
            retire_effect: RetireEffect::GoldGrant { amount: 650 },
        },

        // ══════════════════════════════════════════════════════════════════════
        // Atomic era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Ahmad Shah Massoud",
            person_type: GreatPersonType::General,
            era: "Atomic",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 14,
            },
        },
        GreatPersonDef {
            name: "Vo Nguyen Giap",
            person_type: GreatPersonType::General,
            era: "Atomic",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 14,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Clancy Fernando",
            person_type: GreatPersonType::Admiral,
            era: "Atomic",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 14,
            },
        },
        GreatPersonDef {
            name: "Grace Hopper",
            person_type: GreatPersonType::Admiral,
            era: "Atomic",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 14,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "John Roebling",
            person_type: GreatPersonType::Engineer,
            era: "Atomic",
            retire_effect: RetireEffect::ProductionBurst { amount: 700 },
        },
        GreatPersonDef {
            name: "Sergei Korolev",
            person_type: GreatPersonType::Engineer,
            era: "Atomic",
            retire_effect: RetireEffect::ProductionBurst { amount: 750 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Estée Lauder",
            person_type: GreatPersonType::Merchant,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 700 },
        },
        GreatPersonDef {
            name: "Jamsetji Tata",
            person_type: GreatPersonType::Merchant,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 750 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Alan Turing",
            person_type: GreatPersonType::Scientist,
            era: "Atomic",
            retire_effect: RetireEffect::ProductionBurst { amount: 700 },
        },
        GreatPersonDef {
            name: "Lise Meitner",
            person_type: GreatPersonType::Scientist,
            era: "Atomic",
            retire_effect: RetireEffect::ProductionBurst { amount: 750 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Alexander Solzhenitsyn",
            person_type: GreatPersonType::Writer,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 700 },
        },
        GreatPersonDef {
            name: "Rabindranath Tagore",
            person_type: GreatPersonType::Writer,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 750 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Andy Warhol",
            person_type: GreatPersonType::Artist,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 700 },
        },
        GreatPersonDef {
            name: "Yayoi Kusama",
            person_type: GreatPersonType::Artist,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 750 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Ella Fitzgerald",
            person_type: GreatPersonType::Musician,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 700 },
        },
        GreatPersonDef {
            name: "Louis Armstrong",
            person_type: GreatPersonType::Musician,
            era: "Atomic",
            retire_effect: RetireEffect::GoldGrant { amount: 750 },
        },

        // ══════════════════════════════════════════════════════════════════════
        // Information era
        // ══════════════════════════════════════════════════════════════════════

        // ── Great Generals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Vijaya Wimalaratne",
            person_type: GreatPersonType::General,
            era: "Information",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 15,
            },
        },
        GreatPersonDef {
            name: "Norman Schwarzkopf",
            person_type: GreatPersonType::General,
            era: "Information",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Land,
                bonus: 15,
            },
        },
        // ── Great Admirals ──────────────────────────────────────────────────
        GreatPersonDef {
            name: "Tōgō Heihachirō",
            person_type: GreatPersonType::Admiral,
            era: "Information",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 15,
            },
        },
        GreatPersonDef {
            name: "Arleigh Burke",
            person_type: GreatPersonType::Admiral,
            era: "Information",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 15,
            },
        },
        // ── Great Engineers ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Tim Berners-Lee",
            person_type: GreatPersonType::Engineer,
            era: "Information",
            retire_effect: RetireEffect::ProductionBurst { amount: 800 },
        },
        GreatPersonDef {
            name: "Hedy Lamarr",
            person_type: GreatPersonType::Engineer,
            era: "Information",
            retire_effect: RetireEffect::ProductionBurst { amount: 850 },
        },
        // ── Great Merchants ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Bill Gates",
            person_type: GreatPersonType::Merchant,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 800 },
        },
        GreatPersonDef {
            name: "Oprah Winfrey",
            person_type: GreatPersonType::Merchant,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 850 },
        },
        // ── Great Scientists ────────────────────────────────────────────────
        GreatPersonDef {
            name: "Stephen Hawking",
            person_type: GreatPersonType::Scientist,
            era: "Information",
            retire_effect: RetireEffect::ProductionBurst { amount: 800 },
        },
        GreatPersonDef {
            name: "Tu Youyou",
            person_type: GreatPersonType::Scientist,
            era: "Information",
            retire_effect: RetireEffect::ProductionBurst { amount: 850 },
        },
        // ── Great Writers ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Toni Morrison",
            person_type: GreatPersonType::Writer,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 800 },
        },
        GreatPersonDef {
            name: "Gabriel García Márquez",
            person_type: GreatPersonType::Writer,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 850 },
        },
        // ── Great Artists ───────────────────────────────────────────────────
        GreatPersonDef {
            name: "Ai Weiwei",
            person_type: GreatPersonType::Artist,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 800 },
        },
        GreatPersonDef {
            name: "Banksy",
            person_type: GreatPersonType::Artist,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 850 },
        },
        // ── Great Musicians ─────────────────────────────────────────────────
        GreatPersonDef {
            name: "Bob Dylan",
            person_type: GreatPersonType::Musician,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 800 },
        },
        GreatPersonDef {
            name: "Aretha Franklin",
            person_type: GreatPersonType::Musician,
            era: "Information",
            retire_effect: RetireEffect::GoldGrant { amount: 850 },
        },
    ]
}

#[derive(Debug, Clone)]
pub struct GreatPerson {
    pub id: GreatPersonId,
    pub name: &'static str,
    pub person_type: GreatPersonType,
    // TODO(PHASE3-8.6): Change era from &'static str to EraId (typed ID); update
    //   GreatPerson::new() signature. Currently a string risks silent mismatches.
    pub era: &'static str,
    pub owner: Option<CivId>,
    pub coord: Option<HexCoord>,
    pub ability_names: Vec<&'static str>,
    pub is_retired: bool,
}

impl GreatPerson {
    pub fn new(id: GreatPersonId, name: &'static str, person_type: GreatPersonType, era: &'static str) -> Self {
        Self {
            id,
            name,
            person_type,
            era,
            owner: None,
            coord: None,
            ability_names: Vec::new(),
            is_retired: false,
        }
    }

    pub fn is_available(&self) -> bool {
        self.owner.is_none() && !self.is_retired
    }
}

/// Spawn a great person from a definition, creating both the `GreatPerson`
/// entry and a corresponding `BasicUnit` (category = GreatPerson, no combat
/// strength). Returns the `GreatPersonId`.
pub fn spawn_great_person(
    state: &mut crate::game::state::GameState,
    civ_id: CivId,
    def_name: &str,
    coord: HexCoord,
) -> GreatPersonId {
    let def = state.great_person_defs.iter()
        .find(|d| d.name == def_name)
        .expect("great person def not found");

    let person_type = def.person_type;
    let era = def.era;
    let name = def.name;
    let domain = match person_type {
        GreatPersonType::Admiral => UnitDomain::Sea,
        _ => UnitDomain::Land,
    };

    let gp_id = state.id_gen.next_great_person_id();
    let mut gp = GreatPerson::new(gp_id, name, person_type, era);
    gp.owner = Some(civ_id);
    gp.coord = Some(coord);
    state.great_people.push(gp);

    // Create a corresponding unit so the great person appears on the map.
    let unit_id = state.id_gen.next_unit_id();
    state.units.push(crate::civ::BasicUnit {
        id: unit_id,
        unit_type: crate::UnitTypeId::nil(),
        owner: civ_id,
        coord,
        domain,
        category: crate::UnitCategory::GreatPerson,
        movement_left: 200,
        max_movement: 200,
        combat_strength: None,
        promotions: Vec::new(),
        experience: 0,
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None, religion_id: None, spread_charges: None, religious_strength: None,
    });

    gp_id
}

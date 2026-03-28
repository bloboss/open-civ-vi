use crate::{CivId, GreatPersonId, GreatPersonType, UnitDomain};
use libhexgrid::coord::HexCoord;

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

/// Returns the four built-in dummy great person definitions.
pub fn builtin_great_person_defs() -> Vec<GreatPersonDef> {
    vec![
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
            name: "Themistocles",
            person_type: GreatPersonType::Admiral,
            era: "Ancient",
            retire_effect: RetireEffect::CombatStrengthBonus {
                domain: UnitDomain::Sea,
                bonus: 5,
            },
        },
        GreatPersonDef {
            name: "Imhotep",
            person_type: GreatPersonType::Engineer,
            era: "Ancient",
            retire_effect: RetireEffect::ProductionBurst { amount: 200 },
        },
        GreatPersonDef {
            name: "Marco Polo",
            person_type: GreatPersonType::Merchant,
            era: "Ancient",
            retire_effect: RetireEffect::GoldGrant { amount: 200 },
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

// TODO(PHASE3-8.6): Also add great_person_points: HashMap<GreatPersonType, u32> to
//   Civilization; advance_turn accumulates great_person_points yield; threshold
//   unlocks GreatPerson in state.great_people for recruitment via RulesEngine.
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
        health: 100,
        range: 0,
        vision_range: 2,
        charges: None, trade_origin: None, trade_destination: None,
    });

    gp_id
}

use crate::{CivId, GreatPersonId, GreatPersonType};
use libhexgrid::coord::HexCoord;

pub trait GreatPersonAbility: std::fmt::Debug {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    /// Number of uses (None = unlimited / passive).
    fn uses(&self) -> Option<u32>;
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

use super::diplomacy::GrievanceTrigger;

/// Grievance recorded against the civ that declared war on the recording civ.
#[derive(Debug, Clone, Copy, Default)]
pub struct DeclaredWarGrievance;
impl GrievanceTrigger for DeclaredWarGrievance {
    fn description(&self) -> &'static str { "Declared war" }
    fn grievance_amount(&self) -> i32 { 30 }
}

/// Grievance recorded against the civ that pillaged an improvement.
#[derive(Debug, Clone, Copy, Default)]
pub struct PillageGrievance;
impl GrievanceTrigger for PillageGrievance {
    fn description(&self) -> &'static str { "Pillaged improvement" }
    fn grievance_amount(&self) -> i32 { 5 }
}

/// Grievance recorded against the civ that captured a city.
#[derive(Debug, Clone, Copy, Default)]
pub struct CapturedCityGrievance;
impl GrievanceTrigger for CapturedCityGrievance {
    fn description(&self) -> &'static str { "Captured city" }
    fn grievance_amount(&self) -> i32 { 20 }
}

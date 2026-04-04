//! Climate change constants and helpers (GS-2).

/// CO2 thresholds that trigger each sea-level rise stage (levels 1–7).
pub const CLIMATE_THRESHOLDS: [u32; 7] = [200, 400, 600, 800, 1000, 1200, 1500];

/// Given the current cumulative CO2, return the climate level (0–7).
pub fn climate_level_for_co2(co2: u32) -> u8 {
    let mut level: u8 = 0;
    for &threshold in &CLIMATE_THRESHOLDS {
        if co2 >= threshold {
            level += 1;
        } else {
            break;
        }
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_climate_levels() {
        assert_eq!(climate_level_for_co2(0), 0);
        assert_eq!(climate_level_for_co2(199), 0);
        assert_eq!(climate_level_for_co2(200), 1);
        assert_eq!(climate_level_for_co2(400), 2);
        assert_eq!(climate_level_for_co2(1500), 7);
        assert_eq!(climate_level_for_co2(9999), 7);
    }
}

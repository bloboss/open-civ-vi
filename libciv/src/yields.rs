use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum YieldType {
    Food,
    Production,
    Gold,
    Science,
    Culture,
    Faith,
    Housing,
    Amenities,
    Tourism,
    GreatPersonPoints,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct YieldBundle {
    pub food: i32,
    pub production: i32,
    pub gold: i32,
    pub science: i32,
    pub culture: i32,
    pub faith: i32,
    pub housing: i32,
    pub amenities: i32,
    pub tourism: i32,
    pub great_person_points: i32,
}

impl YieldBundle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, yield_type: YieldType, amount: i32) -> Self {
        self.add_yield(yield_type, amount);
        self
    }

    pub fn add_yield(&mut self, yield_type: YieldType, amount: i32) {
        match yield_type {
            YieldType::Food => self.food += amount,
            YieldType::Production => self.production += amount,
            YieldType::Gold => self.gold += amount,
            YieldType::Science => self.science += amount,
            YieldType::Culture => self.culture += amount,
            YieldType::Faith => self.faith += amount,
            YieldType::Housing => self.housing += amount,
            YieldType::Amenities => self.amenities += amount,
            YieldType::Tourism => self.tourism += amount,
            YieldType::GreatPersonPoints => self.great_person_points += amount,
        }
    }

    pub fn get(&self, yield_type: YieldType) -> i32 {
        match yield_type {
            YieldType::Food => self.food,
            YieldType::Production => self.production,
            YieldType::Gold => self.gold,
            YieldType::Science => self.science,
            YieldType::Culture => self.culture,
            YieldType::Faith => self.faith,
            YieldType::Housing => self.housing,
            YieldType::Amenities => self.amenities,
            YieldType::Tourism => self.tourism,
            YieldType::GreatPersonPoints => self.great_person_points,
        }
    }

    pub fn merge(&self, other: &YieldBundle) -> YieldBundle {
        YieldBundle {
            food: self.food + other.food,
            production: self.production + other.production,
            gold: self.gold + other.gold,
            science: self.science + other.science,
            culture: self.culture + other.culture,
            faith: self.faith + other.faith,
            housing: self.housing + other.housing,
            amenities: self.amenities + other.amenities,
            tourism: self.tourism + other.tourism,
            great_person_points: self.great_person_points + other.great_person_points,
        }
    }
}

impl Add for YieldBundle {
    type Output = YieldBundle;

    fn add(self, rhs: YieldBundle) -> YieldBundle {
        self.merge(&rhs)
    }
}

impl AddAssign for YieldBundle {
    fn add_assign(&mut self, rhs: YieldBundle) {
        self.food += rhs.food;
        self.production += rhs.production;
        self.gold += rhs.gold;
        self.science += rhs.science;
        self.culture += rhs.culture;
        self.faith += rhs.faith;
        self.housing += rhs.housing;
        self.amenities += rhs.amenities;
        self.tourism += rhs.tourism;
        self.great_person_points += rhs.great_person_points;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yield_bundle_merge_additive() {
        let a = YieldBundle::new()
            .with(YieldType::Food, 3)
            .with(YieldType::Production, 1);
        let b = YieldBundle::new()
            .with(YieldType::Food, 2)
            .with(YieldType::Gold, 5);
        let merged = a.merge(&b);
        assert_eq!(merged.food, 5);
        assert_eq!(merged.production, 1);
        assert_eq!(merged.gold, 5);
        assert_eq!(merged.science, 0);

        // Test operator form
        let c = YieldBundle::new().with(YieldType::Science, 10);
        let d = YieldBundle::new().with(YieldType::Science, 7);
        let sum = c + d;
        assert_eq!(sum.science, 17);
    }
}

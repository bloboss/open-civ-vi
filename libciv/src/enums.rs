#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ResourceCategory {
    Bonus,
    Luxury,
    Strategic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnitDomain {
    Land,
    Sea,
    Air,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum UnitCategory {
    Civilian,
    Combat,
    Support,
    Religious,
    GreatPerson,
    Trader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GreatPersonType {
    General,
    Admiral,
    Engineer,
    Merchant,
    Musician,
    Artist,
    Writer,
    Prophet,
    Scientist,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AgeType {
    Ancient,
    Classical,
    Medieval,
    Renaissance,
    Industrial,
    Modern,
    Atomic,
    Information,
    Future,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PolicyType {
    Military,
    Economic,
    Diplomatic,
    Wildcard,
}

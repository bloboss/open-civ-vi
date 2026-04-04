// Full civic tree definition — Ancient through Information era.
// Included verbatim inside `build_civic_tree`; `tree`, `ids`, `CivicRefs`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression
// that evaluates to `CivicRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────

// Ancient Era
let code_of_laws_id        = CivicId::from_ulid(ids.next_ulid());
let craftsmanship_id       = CivicId::from_ulid(ids.next_ulid());
let foreign_trade_id       = CivicId::from_ulid(ids.next_ulid());
let early_empire_id        = CivicId::from_ulid(ids.next_ulid());
let mysticism_id           = CivicId::from_ulid(ids.next_ulid());
let military_tradition_id  = CivicId::from_ulid(ids.next_ulid());
let state_workforce_id     = CivicId::from_ulid(ids.next_ulid());

// Classical Era
let games_and_recreation_id = CivicId::from_ulid(ids.next_ulid());
let political_philosophy_id = CivicId::from_ulid(ids.next_ulid());
let drama_and_poetry_id    = CivicId::from_ulid(ids.next_ulid());
let theology_id            = CivicId::from_ulid(ids.next_ulid());
let military_training_id   = CivicId::from_ulid(ids.next_ulid());
let defensive_tactics_id   = CivicId::from_ulid(ids.next_ulid());
let recorded_history_id    = CivicId::from_ulid(ids.next_ulid());

// Medieval Era
let naval_tradition_id     = CivicId::from_ulid(ids.next_ulid());
let feudalism_id           = CivicId::from_ulid(ids.next_ulid());
let civil_service_id       = CivicId::from_ulid(ids.next_ulid());
let divine_right_id        = CivicId::from_ulid(ids.next_ulid());
let mercenaries_id         = CivicId::from_ulid(ids.next_ulid());
let medieval_faires_id     = CivicId::from_ulid(ids.next_ulid());
let guilds_id              = CivicId::from_ulid(ids.next_ulid());

// Renaissance Era
let exploration_id         = CivicId::from_ulid(ids.next_ulid());
let reformed_church_id     = CivicId::from_ulid(ids.next_ulid());
let humanism_id            = CivicId::from_ulid(ids.next_ulid());
let diplomatic_service_id  = CivicId::from_ulid(ids.next_ulid());
let mercantilism_id        = CivicId::from_ulid(ids.next_ulid());
let the_enlightenment_id   = CivicId::from_ulid(ids.next_ulid());

// Industrial Era
let colonialism_id         = CivicId::from_ulid(ids.next_ulid());
let opera_and_ballet_id    = CivicId::from_ulid(ids.next_ulid());
let natural_history_id     = CivicId::from_ulid(ids.next_ulid());
let civil_engineering_id   = CivicId::from_ulid(ids.next_ulid());
let nationalism_id         = CivicId::from_ulid(ids.next_ulid());
let scorched_earth_id      = CivicId::from_ulid(ids.next_ulid());
let urbanization_id        = CivicId::from_ulid(ids.next_ulid());

// Modern Era
let conservation_id        = CivicId::from_ulid(ids.next_ulid());
let mass_media_id          = CivicId::from_ulid(ids.next_ulid());
let mobilization_id        = CivicId::from_ulid(ids.next_ulid());
let ideology_id            = CivicId::from_ulid(ids.next_ulid());
let capitalism_id          = CivicId::from_ulid(ids.next_ulid());
let nuclear_program_id     = CivicId::from_ulid(ids.next_ulid());
let suffrage_id            = CivicId::from_ulid(ids.next_ulid());
let totalitarianism_id     = CivicId::from_ulid(ids.next_ulid());
let class_struggle_id      = CivicId::from_ulid(ids.next_ulid());

// Atomic Era
let cultural_heritage_id   = CivicId::from_ulid(ids.next_ulid());
let cold_war_id            = CivicId::from_ulid(ids.next_ulid());
let professional_sports_id = CivicId::from_ulid(ids.next_ulid());
let rapid_deployment_id    = CivicId::from_ulid(ids.next_ulid());
let space_race_id          = CivicId::from_ulid(ids.next_ulid());

// Information Era
let globalization_id       = CivicId::from_ulid(ids.next_ulid());
let social_media_id        = CivicId::from_ulid(ids.next_ulid());

// Future Era (Gathering Storm)
let environmentalism_id         = CivicId::from_ulid(ids.next_ulid());
let corporate_libertarianism_id = CivicId::from_ulid(ids.next_ulid());
let digital_democracy_id        = CivicId::from_ulid(ids.next_ulid());
let synthetic_technocracy_id    = CivicId::from_ulid(ids.next_ulid());
let information_warfare_id      = CivicId::from_ulid(ids.next_ulid());
let cultural_hegemony_id        = CivicId::from_ulid(ids.next_ulid());
let exodus_imperative_id        = CivicId::from_ulid(ids.next_ulid());
let near_future_governance_id   = CivicId::from_ulid(ids.next_ulid());
let future_civic_id             = CivicId::from_ulid(ids.next_ulid());

// Sentinel
let unreachable_id         = CivicId::from_ulid(ids.next_ulid());

// ══════════════════════════════════════════════════════════════════════════════
// Ancient Era
// ══════════════════════════════════════════════════════════════════════════════

let code_of_laws = CivicNode {
    id:                      code_of_laws_id,
    name:                    "Code of Laws",
    cost:                    20,
    prerequisites:           vec![],
    effects:                 vec![UnlockGovernment("Chiefdom"), UnlockPolicy("Discipline")],
    inspiration_description: "Found your first city.",
    inspiration_effects:     vec![],
};

let craftsmanship = CivicNode {
    id:                      craftsmanship_id,
    name:                    "Craftsmanship",
    cost:                    40,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockPolicy("Ilkum")],
    inspiration_description: "Build 3 tile improvements.",
    inspiration_effects:     vec![],
};

let foreign_trade = CivicNode {
    id:                      foreign_trade_id,
    name:                    "Foreign Trade",
    cost:                    40,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockUnit("Trader")],
    inspiration_description: "Find a second continent.",
    inspiration_effects:     vec![],
};

// P0 fix: Early Empire prereq is Foreign Trade only (removed Craftsmanship).
let early_empire = CivicNode {
    id:                      early_empire_id,
    name:                    "Early Empire",
    cost:                    70,
    prerequisites:           vec![foreign_trade_id],
    effects:                 vec![UnlockGovernment("Autocracy"), UnlockPolicy("Urban Planning")],
    inspiration_description: "Reach a population of 6.",
    inspiration_effects:     vec![],
};

// P0 fix: Mysticism prereq is Foreign Trade (was Code of Laws).
let mysticism = CivicNode {
    id:                      mysticism_id,
    name:                    "Mysticism",
    cost:                    50,
    prerequisites:           vec![foreign_trade_id],
    effects:                 vec![UnlockPolicy("Revelation"), UnlockBuilding("Temple")],
    inspiration_description: "Found a pantheon.",
    inspiration_effects:     vec![],
};

let military_tradition = CivicNode {
    id:                      military_tradition_id,
    name:                    "Military Tradition",
    cost:                    50,
    prerequisites:           vec![craftsmanship_id],
    effects:                 vec![UnlockPolicy("Strategos")],
    inspiration_description: "Clear a Barbarian Camp.",
    inspiration_effects:     vec![],
};

let state_workforce = CivicNode {
    id:                      state_workforce_id,
    name:                    "State Workforce",
    cost:                    70,
    prerequisites:           vec![craftsmanship_id],
    effects:                 vec![UnlockPolicy("Corvee")],
    inspiration_description: "Build a district.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Classical Era
// ══════════════════════════════════════════════════════════════════════════════

let games_and_recreation = CivicNode {
    id:                      games_and_recreation_id,
    name:                    "Games & Recreation",
    cost:                    110,
    prerequisites:           vec![state_workforce_id],
    effects:                 vec![UnlockBuilding("Arena")],
    inspiration_description: "Build an Entertainment Complex.",
    inspiration_effects:     vec![],
};

let political_philosophy = CivicNode {
    id:                      political_philosophy_id,
    name:                    "Political Philosophy",
    cost:                    110,
    prerequisites:           vec![state_workforce_id, early_empire_id],
    effects:                 vec![UnlockGovernment("Autocracy"), UnlockGovernment("Oligarchy"), UnlockGovernment("Classical Republic")],
    inspiration_description: "Meet 3 city-states.",
    inspiration_effects:     vec![],
};

let drama_and_poetry = CivicNode {
    id:                      drama_and_poetry_id,
    name:                    "Drama & Poetry",
    cost:                    110,
    prerequisites:           vec![early_empire_id],
    effects:                 vec![UnlockBuilding("Amphitheater")],
    inspiration_description: "Build a Wonder.",
    inspiration_effects:     vec![],
};

// P0 fix: Theology moved from tech tree to civic tree.
let theology = CivicNode {
    id:                      theology_id,
    name:                    "Theology",
    cost:                    120,
    prerequisites:           vec![drama_and_poetry_id, mysticism_id],
    effects:                 vec![UnlockUnit("Missionary"), UnlockUnit("Apostle")],
    inspiration_description: "Found a religion.",
    inspiration_effects:     vec![],
};

let military_training = CivicNode {
    id:                      military_training_id,
    name:                    "Military Training",
    cost:                    120,
    prerequisites:           vec![military_tradition_id, games_and_recreation_id],
    effects:                 vec![UnlockBuilding("Barracks"), UnlockPolicy("Military Policy")],
    inspiration_description: "Build an Encampment.",
    inspiration_effects:     vec![],
};

let defensive_tactics = CivicNode {
    id:                      defensive_tactics_id,
    name:                    "Defensive Tactics",
    cost:                    175,
    prerequisites:           vec![games_and_recreation_id, political_philosophy_id],
    effects:                 vec![UnlockPolicy("Limes")],
    inspiration_description: "Be the target of a declaration of war.",
    inspiration_effects:     vec![],
};

let recorded_history = CivicNode {
    id:                      recorded_history_id,
    name:                    "Recorded History",
    cost:                    175,
    prerequisites:           vec![political_philosophy_id, drama_and_poetry_id],
    effects:                 vec![],
    inspiration_description: "Build 2 Campus districts.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Medieval Era
// ══════════════════════════════════════════════════════════════════════════════

let naval_tradition = CivicNode {
    id:                      naval_tradition_id,
    name:                    "Naval Tradition",
    cost:                    200,
    prerequisites:           vec![defensive_tactics_id],
    effects:                 vec![UnlockPolicy("Maritime Industries")],
    inspiration_description: "Kill a unit with a Quadrireme.",
    inspiration_effects:     vec![],
};

let feudalism = CivicNode {
    id:                      feudalism_id,
    name:                    "Feudalism",
    cost:                    275,
    prerequisites:           vec![defensive_tactics_id],
    effects:                 vec![],
    inspiration_description: "Build 6 Farms.",
    inspiration_effects:     vec![],
};

let civil_service = CivicNode {
    id:                      civil_service_id,
    name:                    "Civil Service",
    cost:                    275,
    prerequisites:           vec![defensive_tactics_id, recorded_history_id],
    effects:                 vec![],
    inspiration_description: "Grow a city to 10 population.",
    inspiration_effects:     vec![],
};

let divine_right = CivicNode {
    id:                      divine_right_id,
    name:                    "Divine Right",
    cost:                    290,
    prerequisites:           vec![civil_service_id, theology_id],
    effects:                 vec![UnlockGovernment("Monarchy")],
    inspiration_description: "Build 2 Temples.",
    inspiration_effects:     vec![],
};

let mercenaries = CivicNode {
    id:                      mercenaries_id,
    name:                    "Mercenaries",
    cost:                    290,
    prerequisites:           vec![military_training_id, feudalism_id],
    effects:                 vec![UnlockPolicy("Professional Army")],
    inspiration_description: "Have 8 land combat units.",
    inspiration_effects:     vec![],
};

let medieval_faires = CivicNode {
    id:                      medieval_faires_id,
    name:                    "Medieval Faires",
    cost:                    385,
    prerequisites:           vec![feudalism_id],
    effects:                 vec![],
    inspiration_description: "Have 4 trade routes.",
    inspiration_effects:     vec![],
};

let guilds = CivicNode {
    id:                      guilds_id,
    name:                    "Guilds",
    cost:                    385,
    prerequisites:           vec![feudalism_id, civil_service_id],
    effects:                 vec![],
    inspiration_description: "Build 2 Markets.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Renaissance Era
// ══════════════════════════════════════════════════════════════════════════════

let exploration = CivicNode {
    id:                      exploration_id,
    name:                    "Exploration",
    cost:                    400,
    prerequisites:           vec![mercenaries_id, medieval_faires_id],
    effects:                 vec![UnlockGovernment("Merchant Republic")],
    inspiration_description: "Build 2 Caravels.",
    inspiration_effects:     vec![],
};

let reformed_church = CivicNode {
    id:                      reformed_church_id,
    name:                    "Reformed Church",
    cost:                    400,
    prerequisites:           vec![guilds_id, divine_right_id],
    effects:                 vec![],
    inspiration_description: "Have 6 cities following your religion.",
    inspiration_effects:     vec![],
};

let humanism = CivicNode {
    id:                      humanism_id,
    name:                    "Humanism",
    cost:                    540,
    prerequisites:           vec![medieval_faires_id, guilds_id],
    effects:                 vec![],
    inspiration_description: "Earn a Great Artist.",
    inspiration_effects:     vec![],
};

let diplomatic_service = CivicNode {
    id:                      diplomatic_service_id,
    name:                    "Diplomatic Service",
    cost:                    540,
    prerequisites:           vec![guilds_id],
    effects:                 vec![],
    inspiration_description: "Ally with a city-state.",
    inspiration_effects:     vec![],
};

let mercantilism = CivicNode {
    id:                      mercantilism_id,
    name:                    "Mercantilism",
    cost:                    655,
    prerequisites:           vec![humanism_id],
    effects:                 vec![],
    inspiration_description: "Earn a Great Merchant.",
    inspiration_effects:     vec![],
};

let the_enlightenment = CivicNode {
    id:                      the_enlightenment_id,
    name:                    "The Enlightenment",
    cost:                    655,
    prerequisites:           vec![humanism_id, diplomatic_service_id],
    effects:                 vec![],
    inspiration_description: "Earn 3 Great People.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Industrial Era
// ══════════════════════════════════════════════════════════════════════════════

let colonialism = CivicNode {
    id:                      colonialism_id,
    name:                    "Colonialism",
    cost:                    725,
    prerequisites:           vec![mercantilism_id],
    effects:                 vec![],
    inspiration_description: "Research the Astronomy technology.",
    inspiration_effects:     vec![],
};

let opera_and_ballet = CivicNode {
    id:                      opera_and_ballet_id,
    name:                    "Opera & Ballet",
    cost:                    725,
    prerequisites:           vec![the_enlightenment_id],
    effects:                 vec![UnlockBuilding("Art Museum"), UnlockBuilding("Archaeological Museum")],
    inspiration_description: "Build 2 Art Museums.",
    inspiration_effects:     vec![],
};

let natural_history = CivicNode {
    id:                      natural_history_id,
    name:                    "Natural History",
    cost:                    870,
    prerequisites:           vec![colonialism_id],
    effects:                 vec![],
    inspiration_description: "Build 2 Archaeological Museums.",
    inspiration_effects:     vec![],
};

let civil_engineering = CivicNode {
    id:                      civil_engineering_id,
    name:                    "Civil Engineering",
    cost:                    920,
    prerequisites:           vec![mercantilism_id],
    effects:                 vec![],
    inspiration_description: "Build 7 different specialty districts.",
    inspiration_effects:     vec![],
};

let nationalism = CivicNode {
    id:                      nationalism_id,
    name:                    "Nationalism",
    cost:                    920,
    prerequisites:           vec![the_enlightenment_id],
    effects:                 vec![UnlockPolicy("Nationalism Policy")],
    inspiration_description: "Declare war using a Casus Belli.",
    inspiration_effects:     vec![],
};

let scorched_earth = CivicNode {
    id:                      scorched_earth_id,
    name:                    "Scorched Earth",
    cost:                    1060,
    prerequisites:           vec![nationalism_id],
    effects:                 vec![],
    inspiration_description: "Build 2 Field Cannons.",
    inspiration_effects:     vec![],
};

let urbanization = CivicNode {
    id:                      urbanization_id,
    name:                    "Urbanization",
    cost:                    1060,
    prerequisites:           vec![civil_engineering_id, nationalism_id],
    effects:                 vec![],
    inspiration_description: "Grow a city to 15 population.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Modern Era
// ══════════════════════════════════════════════════════════════════════════════

let conservation = CivicNode {
    id:                      conservation_id,
    name:                    "Conservation",
    cost:                    1255,
    prerequisites:           vec![natural_history_id],
    effects:                 vec![],
    inspiration_description: "Have a Neighborhood with Breathtaking Appeal.",
    inspiration_effects:     vec![],
};

let mass_media = CivicNode {
    id:                      mass_media_id,
    name:                    "Mass Media",
    cost:                    1410,
    prerequisites:           vec![natural_history_id, urbanization_id],
    effects:                 vec![UnlockBuilding("Broadcast Center")],
    inspiration_description: "Research Radio.",
    inspiration_effects:     vec![],
};

let mobilization = CivicNode {
    id:                      mobilization_id,
    name:                    "Mobilization",
    cost:                    1410,
    prerequisites:           vec![urbanization_id],
    effects:                 vec![],
    inspiration_description: "Have 3 Corps in your military.",
    inspiration_effects:     vec![],
};

let ideology = CivicNode {
    id:                      ideology_id,
    name:                    "Ideology",
    cost:                    660,
    prerequisites:           vec![mass_media_id, mobilization_id],
    effects:                 vec![],
    inspiration_description: "Have 2 Policies slotted.",
    inspiration_effects:     vec![],
};

let capitalism = CivicNode {
    id:                      capitalism_id,
    name:                    "Capitalism",
    cost:                    1560,
    prerequisites:           vec![mass_media_id],
    effects:                 vec![],
    inspiration_description: "Build 3 Stock Exchanges.",
    inspiration_effects:     vec![],
};

let nuclear_program = CivicNode {
    id:                      nuclear_program_id,
    name:                    "Nuclear Program",
    cost:                    1715,
    prerequisites:           vec![ideology_id],
    effects:                 vec![],
    inspiration_description: "Build a Research Lab.",
    inspiration_effects:     vec![],
};

let suffrage = CivicNode {
    id:                      suffrage_id,
    name:                    "Suffrage",
    cost:                    1715,
    prerequisites:           vec![ideology_id],
    effects:                 vec![UnlockGovernment("Democracy")],
    inspiration_description: "Build 4 Neighborhoods.",
    inspiration_effects:     vec![],
};

let totalitarianism = CivicNode {
    id:                      totalitarianism_id,
    name:                    "Totalitarianism",
    cost:                    1715,
    prerequisites:           vec![ideology_id],
    effects:                 vec![UnlockGovernment("Fascism")],
    inspiration_description: "Build 3 Military Academies.",
    inspiration_effects:     vec![],
};

let class_struggle = CivicNode {
    id:                      class_struggle_id,
    name:                    "Class Struggle",
    cost:                    1715,
    prerequisites:           vec![ideology_id],
    effects:                 vec![UnlockGovernment("Communism")],
    inspiration_description: "Build 3 Factories.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Atomic Era
// ══════════════════════════════════════════════════════════════════════════════

let cultural_heritage = CivicNode {
    id:                      cultural_heritage_id,
    name:                    "Cultural Heritage",
    cost:                    1955,
    prerequisites:           vec![conservation_id],
    effects:                 vec![],
    inspiration_description: "Have a themed building.",
    inspiration_effects:     vec![],
};

let cold_war = CivicNode {
    id:                      cold_war_id,
    name:                    "Cold War",
    cost:                    2185,
    prerequisites:           vec![ideology_id],
    effects:                 vec![],
    inspiration_description: "Research Nuclear Fission.",
    inspiration_effects:     vec![],
};

let professional_sports = CivicNode {
    id:                      professional_sports_id,
    name:                    "Professional Sports",
    cost:                    2185,
    prerequisites:           vec![ideology_id],
    effects:                 vec![],
    inspiration_description: "Build 4 Entertainment Complexes.",
    inspiration_effects:     vec![],
};

let rapid_deployment = CivicNode {
    id:                      rapid_deployment_id,
    name:                    "Rapid Deployment",
    cost:                    2415,
    prerequisites:           vec![cold_war_id],
    effects:                 vec![],
    inspiration_description: "Build an Aerodrome or Airstrip on a foreign continent.",
    inspiration_effects:     vec![],
};

let space_race = CivicNode {
    id:                      space_race_id,
    name:                    "Space Race",
    cost:                    2415,
    prerequisites:           vec![cold_war_id],
    effects:                 vec![],
    inspiration_description: "Build a Spaceport.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Information Era
// ══════════════════════════════════════════════════════════════════════════════

let globalization = CivicNode {
    id:                      globalization_id,
    name:                    "Globalization",
    cost:                    2880,
    prerequisites:           vec![rapid_deployment_id, space_race_id],
    effects:                 vec![],
    inspiration_description: "Build 3 Airports.",
    inspiration_effects:     vec![],
};

let social_media = CivicNode {
    id:                      social_media_id,
    name:                    "Social Media",
    cost:                    2880,
    prerequisites:           vec![space_race_id, professional_sports_id],
    effects:                 vec![],
    inspiration_description: "Have the highest Tourism output.",
    inspiration_effects:     vec![],
};

// ══════════════════════════════════════════════════════════════════════════════
// Future Era (Gathering Storm)
// ══════════════════════════════════════════════════════════════════════════════

let environmentalism = CivicNode {
    id:                      environmentalism_id,
    name:                    "Environmentalism",
    cost:                    2415,
    prerequisites:           vec![cultural_heritage_id],
    effects:                 vec![],
    inspiration_description: "Build a Solar Farm or Wind Farm.",
    inspiration_effects:     vec![],
};

let near_future_governance = CivicNode {
    id:                      near_future_governance_id,
    name:                    "Near Future Governance",
    cost:                    2880,
    prerequisites:           vec![globalization_id],
    effects:                 vec![],
    inspiration_description: "Build 3 Future Era buildings.",
    inspiration_effects:     vec![],
};

let corporate_libertarianism = CivicNode {
    id:                      corporate_libertarianism_id,
    name:                    "Corporate Libertarianism",
    cost:                    2880,
    prerequisites:           vec![globalization_id],
    effects:                 vec![UnlockGovernment("Corporate Libertarianism")],
    inspiration_description: "Have 5 trade routes.",
    inspiration_effects:     vec![],
};

let digital_democracy = CivicNode {
    id:                      digital_democracy_id,
    name:                    "Digital Democracy",
    cost:                    2880,
    prerequisites:           vec![social_media_id],
    effects:                 vec![UnlockGovernment("Digital Democracy")],
    inspiration_description: "Have 2 tier-3 buildings in every district type.",
    inspiration_effects:     vec![],
};

let synthetic_technocracy = CivicNode {
    id:                      synthetic_technocracy_id,
    name:                    "Synthetic Technocracy",
    cost:                    2880,
    prerequisites:           vec![social_media_id],
    effects:                 vec![UnlockGovernment("Synthetic Technocracy")],
    inspiration_description: "Research a Future Era technology.",
    inspiration_effects:     vec![],
};

let information_warfare = CivicNode {
    id:                      information_warfare_id,
    name:                    "Information Warfare",
    cost:                    2880,
    prerequisites:           vec![social_media_id],
    effects:                 vec![],
    inspiration_description: "Complete an espionage operation.",
    inspiration_effects:     vec![],
};

let cultural_hegemony = CivicNode {
    id:                      cultural_hegemony_id,
    name:                    "Cultural Hegemony",
    cost:                    3200,
    prerequisites:           vec![globalization_id, social_media_id],
    effects:                 vec![],
    inspiration_description: "Have cultural dominance over another civ.",
    inspiration_effects:     vec![],
};

let exodus_imperative = CivicNode {
    id:                      exodus_imperative_id,
    name:                    "Exodus Imperative",
    cost:                    3200,
    prerequisites:           vec![globalization_id, social_media_id],
    effects:                 vec![],
    inspiration_description: "Launch an Exoplanet Expedition.",
    inspiration_effects:     vec![],
};

let future_civic = CivicNode {
    id:                      future_civic_id,
    name:                    "Future Civic",
    cost:                    3200,
    prerequisites:           vec![cultural_hegemony_id, exodus_imperative_id],
    effects:                 vec![],
    inspiration_description: "",
    inspiration_effects:     vec![],
};

// Sentinel civic: self-referential prerequisite means prerequisites_met() is always false.
// Used by districts not yet tied to a real civic (required_civic: Some(civic_refs.unreachable)).
let unreachable = CivicNode {
    id:                      unreachable_id,
    name:                    "Unreachable",
    cost:                    u32::MAX,
    prerequisites:           vec![unreachable_id],
    effects:                 vec![],
    inspiration_description: "",
    inspiration_effects:     vec![],
};

// ── Add all nodes to the tree ─────────────────────────────────────────────────

// Ancient
tree.add_node(code_of_laws);
tree.add_node(craftsmanship);
tree.add_node(foreign_trade);
tree.add_node(early_empire);
tree.add_node(mysticism);
tree.add_node(military_tradition);
tree.add_node(state_workforce);

// Classical
tree.add_node(games_and_recreation);
tree.add_node(political_philosophy);
tree.add_node(drama_and_poetry);
tree.add_node(theology);
tree.add_node(military_training);
tree.add_node(defensive_tactics);
tree.add_node(recorded_history);

// Medieval
tree.add_node(naval_tradition);
tree.add_node(feudalism);
tree.add_node(civil_service);
tree.add_node(divine_right);
tree.add_node(mercenaries);
tree.add_node(medieval_faires);
tree.add_node(guilds);

// Renaissance
tree.add_node(exploration);
tree.add_node(reformed_church);
tree.add_node(humanism);
tree.add_node(diplomatic_service);
tree.add_node(mercantilism);
tree.add_node(the_enlightenment);

// Industrial
tree.add_node(colonialism);
tree.add_node(opera_and_ballet);
tree.add_node(natural_history);
tree.add_node(civil_engineering);
tree.add_node(nationalism);
tree.add_node(scorched_earth);
tree.add_node(urbanization);

// Modern
tree.add_node(conservation);
tree.add_node(mass_media);
tree.add_node(mobilization);
tree.add_node(ideology);
tree.add_node(capitalism);
tree.add_node(nuclear_program);
tree.add_node(suffrage);
tree.add_node(totalitarianism);
tree.add_node(class_struggle);

// Atomic
tree.add_node(cultural_heritage);
tree.add_node(cold_war);
tree.add_node(professional_sports);
tree.add_node(rapid_deployment);
tree.add_node(space_race);

// Information
tree.add_node(globalization);
tree.add_node(social_media);

// Future (Gathering Storm)
tree.add_node(environmentalism);
tree.add_node(near_future_governance);
tree.add_node(corporate_libertarianism);
tree.add_node(digital_democracy);
tree.add_node(synthetic_technocracy);
tree.add_node(information_warfare);
tree.add_node(cultural_hegemony);
tree.add_node(exodus_imperative);
tree.add_node(future_civic);

// Sentinel
tree.add_node(unreachable);

// ── Return named ID handles ───────────────────────────────────────────────────

CivicRefs {
    code_of_laws:        code_of_laws_id,
    craftsmanship:       craftsmanship_id,
    foreign_trade:       foreign_trade_id,
    early_empire:        early_empire_id,
    mysticism:           mysticism_id,
    military_tradition:  military_tradition_id,
    state_workforce:     state_workforce_id,
    games_and_recreation: games_and_recreation_id,
    political_philosophy: political_philosophy_id,
    drama_and_poetry:    drama_and_poetry_id,
    theology:            theology_id,
    military_training:   military_training_id,
    defensive_tactics:   defensive_tactics_id,
    recorded_history:    recorded_history_id,
    naval_tradition:     naval_tradition_id,
    feudalism:           feudalism_id,
    civil_service:       civil_service_id,
    divine_right:        divine_right_id,
    mercenaries:         mercenaries_id,
    medieval_faires:     medieval_faires_id,
    guilds:              guilds_id,
    exploration:         exploration_id,
    reformed_church:     reformed_church_id,
    humanism:            humanism_id,
    diplomatic_service:  diplomatic_service_id,
    mercantilism:        mercantilism_id,
    the_enlightenment:   the_enlightenment_id,
    colonialism:         colonialism_id,
    opera_and_ballet:    opera_and_ballet_id,
    natural_history:     natural_history_id,
    civil_engineering:   civil_engineering_id,
    nationalism:         nationalism_id,
    scorched_earth:      scorched_earth_id,
    urbanization:        urbanization_id,
    conservation:        conservation_id,
    mass_media:          mass_media_id,
    mobilization:        mobilization_id,
    ideology:            ideology_id,
    capitalism:          capitalism_id,
    nuclear_program:     nuclear_program_id,
    suffrage:            suffrage_id,
    totalitarianism:     totalitarianism_id,
    class_struggle:      class_struggle_id,
    cultural_heritage:   cultural_heritage_id,
    cold_war:            cold_war_id,
    professional_sports: professional_sports_id,
    rapid_deployment:    rapid_deployment_id,
    space_race:          space_race_id,
    globalization:       globalization_id,
    social_media:        social_media_id,
    future_civic:        future_civic_id,
    unreachable:         unreachable_id,
}
}

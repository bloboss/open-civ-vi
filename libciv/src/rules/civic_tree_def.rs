// Ancient Era -- civic tree definition.
// Included verbatim inside `build_civic_tree`; `tree`, `ids`, `CivicRefs`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression
// that evaluates to `CivicRefs`.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
let code_of_laws_id  = CivicId::from_ulid(ids.next_ulid());
let craftsmanship_id = CivicId::from_ulid(ids.next_ulid());
let foreign_trade_id = CivicId::from_ulid(ids.next_ulid());
let early_empire_id  = CivicId::from_ulid(ids.next_ulid());
let mysticism_id     = CivicId::from_ulid(ids.next_ulid());
let unreachable_id   = CivicId::from_ulid(ids.next_ulid());

// ── Node definitions ──────────────────────────────────────────────────────────

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

let early_empire = CivicNode {
    id:                      early_empire_id,
    name:                    "Early Empire",
    cost:                    70,
    prerequisites:           vec![craftsmanship_id, foreign_trade_id],
    effects:                 vec![UnlockGovernment("Autocracy"), UnlockPolicy("Urban Planning")],
    inspiration_description: "Reach a population of 6.",
    inspiration_effects:     vec![],
};

let mysticism = CivicNode {
    id:                      mysticism_id,
    name:                    "Mysticism",
    cost:                    50,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockPolicy("Revelation"), UnlockBuilding("Temple")],
    inspiration_description: "Found a pantheon.",
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

tree.add_node(code_of_laws);
tree.add_node(craftsmanship);
tree.add_node(foreign_trade);
tree.add_node(early_empire);
tree.add_node(mysticism);
tree.add_node(unreachable);

// ── Return named ID handles ───────────────────────────────────────────────────

CivicRefs {
    code_of_laws:  code_of_laws_id,
    craftsmanship: craftsmanship_id,
    foreign_trade: foreign_trade_id,
    early_empire:  early_empire_id,
    mysticism:     mysticism_id,
    unreachable:   unreachable_id,
}
}

// Ancient Era -- civic tree definition.
// Included verbatim inside `build_civic_tree`; `tree`, `ids`, and `OneShotEffect::*`
// are all in scope from the enclosing function. This file must be a single block expression.
{
// ── Generate IDs in a fixed order (never reorder) ────────────────────────────
let code_of_laws_id  = CivicId::from_ulid(ids.next_ulid());
let craftsmanship_id = CivicId::from_ulid(ids.next_ulid());
let foreign_trade_id = CivicId::from_ulid(ids.next_ulid());
let early_empire_id  = CivicId::from_ulid(ids.next_ulid());
let mysticism_id     = CivicId::from_ulid(ids.next_ulid());

// ── Node definitions ──────────────────────────────────────────────────────────

tree.add_node(CivicNode {
    id:                      code_of_laws_id,
    name:                    "Code of Laws",
    cost:                    20,
    prerequisites:           vec![],
    effects:                 vec![UnlockGovernment("Chiefdom"), UnlockPolicy("Discipline")],
    inspiration_description: "Found your first city.",
    inspiration_effects:     vec![],
});

tree.add_node(CivicNode {
    id:                      craftsmanship_id,
    name:                    "Craftsmanship",
    cost:                    40,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockPolicy("Ilkum")],
    inspiration_description: "Build 3 tile improvements.",
    inspiration_effects:     vec![],
});

tree.add_node(CivicNode {
    id:                      foreign_trade_id,
    name:                    "Foreign Trade",
    cost:                    40,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockUnit("Trader")],
    inspiration_description: "Find a second continent.",
    inspiration_effects:     vec![],
});

tree.add_node(CivicNode {
    id:                      early_empire_id,
    name:                    "Early Empire",
    cost:                    70,
    prerequisites:           vec![craftsmanship_id, foreign_trade_id],
    effects:                 vec![UnlockGovernment("Autocracy"), UnlockPolicy("Urban Planning")],
    inspiration_description: "Reach a population of 6.",
    inspiration_effects:     vec![],
});

tree.add_node(CivicNode {
    id:                      mysticism_id,
    name:                    "Mysticism",
    cost:                    50,
    prerequisites:           vec![code_of_laws_id],
    effects:                 vec![UnlockPolicy("Revelation")],
    inspiration_description: "Found a pantheon.",
    inspiration_effects:     vec![],
});
}

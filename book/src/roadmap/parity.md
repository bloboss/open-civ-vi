# Civ VI Parity Report

This document is the master index for the systematic comparison between the
open-civ-vi engine and the original Civilization VI base game (no DLC/expansions).

## Methodology

The authoritative reference is the set of XML files shipped with the base game,
located at `original-xml/Base/Assets/Gameplay/Data/`. Every category below was
compared field-by-field against the corresponding XML table. Values marked
"wrong" mean the implementation ships a different number than the XML; "missing"
means the item does not exist in Rust at all.

> **Scope**: Base game only. Items from Rise & Fall, Gathering Storm, or New
> Frontier Pass are noted where the implementation includes them but the base
> game does not.

## Summary Matrix

> **Last updated**: 2026-04-04 — after P0 fixes and P2/P3/P4/P5(districts)/P6/P10 content drops.

| Category | Implemented | Civ VI Base | Missing | Status | Detail Doc |
|---|---|---|---|---|---|
| Terrains | 8 base types | 15 land + 2 water | ~0 (arch. diff) | **Done** | — |
| Features | 8 | 6 | 0 (+2 GS) | **Done** | — |
| Natural Wonders | 15 (12 base + 3 DLC) | 12 | 0 | **P3 Done** | [content](parity-content.md#natural-wonders) |
| Bonus Resources | 10 | 10 | 0 | **P2 Done** | [content](parity-content.md#resources) |
| Luxury Resources | 24 | 24 | 0 | **P2 Done** | [values](parity-values.md#resource-yields) |
| Strategic Resources | 7 | 7 | 0 | **P0+P2 Done** | [values](parity-values.md#resource-yields) |
| Improvements (std) | 15 | 15 | 0 | **P4 Done** | [content](parity-content.md#improvements) |
| Unique Improvements | 9 | 9 | 0 | **P4 Done** | [content](parity-content.md#unique-improvements) |
| Technologies | 12 (Ancient) | 67 (8 eras) | 55 | **P1 Blocked** | [trees](parity-trees.md#technologies) |
| Civics | 8 (Ancient + Theology) | 50 (8 eras) | 42 | **P1 Blocked** | [trees](parity-trees.md#civics) |
| Districts | 16 std + 4 UQ | 13 std + 8 UQ | 0 std, 4 UQ | **P5 Districts Done** | [content](parity-content.md#districts) |
| Buildings | ~6 | 66 | ~60 | **P5 Blocked (P1)** | [content](parity-content.md#buildings) |
| World Wonders | 0 | 29 | 29 | **P7 Not started** | [content](parity-content.md#world-wonders) |
| Units | 71 (incl. unique) | 77 + 21 UQ | ~27 | **P6 ~75%** | [content](parity-content.md#units) |
| Civilizations | 8 | 19 | 11 | **P9 Not started** | [systems](parity-systems.md#civilizations) |
| Governments | 2 | 10 | 8 | **P8 Not started** | [systems](parity-systems.md#governments) |
| Policies | 4 | 113 | 109 | **P8 Not started** | [systems](parity-systems.md#policies) |
| Promotions | 118 (16 classes) | 122 | ~4 | **P10 ~97%** | [systems](parity-systems.md#promotions) |
| City-States | 0 | 24 | 24 | **P11 Not started** | [systems](parity-systems.md#city-states) |
| Great People | ~72 | ~177 | ~105 | **P12 Partial** | [systems](parity-systems.md#great-people) |

## Implementation Phases

Each phase is an independently dispatchable unit of work. Dependencies between
phases are shown in the graph below.

| Phase | Title | Scope | Status |
|---|---|---|---|
| ~~P0~~ | ~~[Fix Value Discrepancies](parity-values.md)~~ | ~~15 yield/cost/prereq corrections~~ | **DONE** |
| **P1** | [Complete Tech & Civic Trees](parity-trees.md) | Add 55 techs + 42 civics across 7 eras | **CRITICAL PATH** |
| ~~P2~~ | ~~[Missing Resources](parity-content.md#resources)~~ | ~~2 bonus + 16 luxury resources~~ | **DONE** |
| ~~P3~~ | ~~[Missing Natural Wonders](parity-content.md#natural-wonders)~~ | ~~10 natural wonders with yields~~ | **DONE** |
| ~~P4~~ | ~~[Standard Improvements](parity-content.md#improvements)~~ | ~~3 std + 9 UQ improvements~~ | **DONE** |
| **P5** | [Districts & Buildings](parity-content.md#districts) | Districts done; ~60 buildings remain (blocked by P1) | **Partial** |
| **P6** | [Units](parity-content.md#units) | 71/98 done; ~27 remaining | **~75%** |
| **P7** | [World Wonders](parity-content.md#world-wonders) | 29 wonder definitions | Not started |
| **P8** | [Governments & Policies](parity-systems.md#governments) | 8 governments + 109 policies | Not started |
| **P9** | [Civilizations & Leaders](parity-systems.md#civilizations) | 11 missing civs with abilities | Not started |
| ~~P10~~ | ~~[Promotions](parity-systems.md#promotions)~~ | ~~118/122 promotions (16 classes)~~ | **~97% DONE** |
| **P11** | [City-States](parity-systems.md#city-states) | 24 city-states with types and bonuses | Not started |
| **P12** | [Great People Expansion](parity-systems.md#great-people) | ~105 missing individuals across all types | Not started |

## Dependency Graph

The following Graphviz DOT graph encodes the task dependencies. An edge `A → B`
means "A must be completed before B can start (or B benefits from A being done
first)."

```dot
digraph parity_phases {
    rankdir=LR;
    node [shape=box, style="rounded,filled", fontname="Helvetica", fontsize=10];

    // ── Phase nodes ──────────────────────────────────────────────────
    P0  [label="P0\nFix Value\nDiscrepancies",  fillcolor="#b6d7a8"];
    P1  [label="P1\nComplete Tech &\nCivic Trees",    fillcolor="#a4c2f4"];
    P2  [label="P2\nMissing\nResources",         fillcolor="#a4c2f4"];
    P3  [label="P3\nNatural\nWonders",           fillcolor="#a4c2f4"];
    P4  [label="P4\nImprovements\n(Std + UQ)",   fillcolor="#a4c2f4"];
    P5  [label="P5\nDistricts &\nBuildings",     fillcolor="#d5a6bd"];
    P6  [label="P6\nUnits\n(Generic + UQ)",      fillcolor="#d5a6bd"];
    P7  [label="P7\nWorld\nWonders",             fillcolor="#d5a6bd"];
    P8  [label="P8\nGovernments\n& Policies",    fillcolor="#ffe599"];
    P9  [label="P9\nCivilizations\n& Leaders",   fillcolor="#ea9999"];
    P10 [label="P10\nPromotions",                fillcolor="#ffe599"];
    P11 [label="P11\nCity-States",               fillcolor="#ffe599"];
    P12 [label="P12\nGreat People\nExpansion",   fillcolor="#ffe599"];

    // ── Dependencies ─────────────────────────────────────────────────
    //
    // P0 is a prerequisite for everything — it fixes base data that all
    // downstream content relies on.
    P0 -> P1;
    P0 -> P2;
    P0 -> P3;
    P0 -> P4;

    // Tech/civic trees gate unlock requirements for later content.
    P1 -> P5  [label="prereq techs/civics"];
    P1 -> P6  [label="prereq techs"];
    P1 -> P7  [label="prereq techs/civics"];
    P1 -> P8  [label="civic unlocks"];
    P1 -> P10 [label="unit classes need techs"];

    // Resources must exist before improvements that harvest them.
    P2 -> P4  [label="harvest targets"];

    // Districts & buildings must exist before wonders (placed in districts)
    // and before units that require buildings (e.g. Barracks → XP).
    P5 -> P6  [label="training buildings"];
    P5 -> P7  [label="wonder placement"];

    // Units must exist before promotions can reference them.
    P6 -> P10 [label="promotion classes"];

    // Governments/policies should exist before civ abilities that
    // reference extra policy slots.
    P8 -> P9  [label="policy slot abilities"];

    // Units + buildings must exist before civ-specific replacements.
    P6 -> P9  [label="UQ unit replacements"];
    P5 -> P9  [label="UQ building replacements"];

    // City-states need their unique improvements (P4) and suzerainty
    // bonuses that reference policies/buildings.
    P4 -> P11 [label="UQ improvements"];
    P5 -> P11 [label="bonus buildings"];

    // Great people reference buildings, units, techs.
    P1 -> P12 [label="era gating"];
    P5 -> P12 [label="engineer wonders"];
    P6 -> P12 [label="general/admiral units"];

    // ── Parallelism hints (subgraph clusters) ────────────────────────
    subgraph cluster_independent {
        label="Can run in parallel after P0";
        style=dashed; color=gray;
        P2; P3;
    }

    subgraph cluster_content {
        label="Core content (after P1)";
        style=dashed; color=gray;
        P5; P6; P7;
    }
}
```

### Reading the graph

- **Green (P0)**: Start here. Pure data fixes, no new files.
- **Blue (P1–P4)**: Foundation content. P2 and P3 are independent of each other
  and can be dispatched in parallel. P4 waits on P2 for resource harvest targets.
- **Purple (P5–P7)**: Core content that depends on the complete tech/civic tree.
  P5 (districts/buildings) should land before P6 (units) and P7 (wonders).
- **Yellow (P8, P10–P12)**: Systems that reference content from earlier phases.
- **Red (P9)**: Civilizations are last because each civ's unique abilities,
  units, and buildings reference items from nearly every other phase.

### Parallel dispatch guide (updated 2026-04-04)

Completed phases (P0, P2, P3, P4, P10) are struck through. Remaining work:

| Wave | Phases | Notes |
|---|---|---|
| ~~1~~ | ~~P0~~ | **DONE** |
| ~~2a~~ | ~~P2, P3~~ | **DONE** |
| 2b | **P1** | **CRITICAL PATH** — all remaining phases depend on this |
| ~~3~~ | ~~P4~~ | **DONE** |
| 4 | P5 (buildings), P8 | Both need P1 tech/civic tree; can run in parallel |
| 5 | P6 (remaining ~27 units), P7 | P7 needs P5 buildings; P6 can run independently |
| 6 | P11, P12 | Need P5/P6 |
| 7 | P9 | Needs P5, P6, P8 (last because civs reference everything) |

# Implementation Status

> **Last updated**: 2026-04-04

## Current Status

**214 passing tests**, 0 failures, 0 clippy warnings. All core Civ VI gameplay
systems are functional end-to-end. The civsim TUI exposes all 38 RulesEngine
methods as interactive commands. Save/load (JSON via serde), apply_delta (state
reconstruction), and the full combat modifier pipeline are operational.

## Civ VI Base-Game Content Parity

| Category | Count | Civ VI Base | Status |
|---|---|---|---|
| Terrains | 8 | 17 | **Done** (arch. diff — hills/mountains are elevation) |
| Features | 8 | 6 (+2 GS) | **Done** |
| Natural Wonders | 15 | 12 | **Done** (+3 DLC) |
| Resources | 41 | 41 | **Done** |
| Improvements | 24 | 24 | **Done** |
| Technologies | 69 | 67 | **Done** |
| Civics | 52 | 50 | **Done** |
| Districts | 20 | 21 | **~95%** (1 UQ district gap) |
| Buildings | 45 | 45 | **Done** (with prereqs + mutual exclusions) |
| World Wonders | 29 | 29 | **Done** |
| Units | 89 | 89 | **Done** (with tech gating + civ replacement) |
| Civilizations | 19 | 19 | **Done** |
| Governments | 10 | 10 | **Done** |
| Policies | 113 | 113 | **Done** |
| Promotions | 118 | 118 | **Done** |
| City-States | 25 | 24 | **Done** |
| Great People | 177 | 177 | **Done** |
| Victory Types | 6 | 6 | **Done** (Score, Culture, Domination, Religious, Science, Diplomatic) |

## Completed Systems

| System | Tests | Notes |
|--------|-------|-------|
| Hex grid | libhexgrid | Coordinates, pathfinding, LOS, edges |
| Map generation | 6 | Continents, zones, features, rivers, resources, starts |
| Terrain, features, resources | 5 | Concealment, yields, movement costs |
| Improvements + builder | 23 | 24 improvements, charges, roads, upgrades |
| Districts | 12 | 20 types, placement validation |
| Buildings | via tests | 45 defs with prereq chains + mutual exclusions |
| Cities, territory, borders | 16 | Founding, claiming, cultural expansion |
| Combat + modifier pipeline | 21+ | Melee/ranged/siege, 6-source modifier resolution, terrain |
| XP + promotions | 9 | Era-scaled XP, 118 promotions, +50 HP heal, class validation |
| City capture + domination | 6 | Walls, bombardment, ownership transfer |
| Tech + civic trees | via tests | 69 techs, 52 civics, eureka/inspiration |
| Trade routes | 11 | Domestic/international, autonomous traders, expiry |
| Diplomacy | via tests | War/peace, grievances, opinion-based status |
| Religion | 55+ | Pantheon, founding, spread, theological combat, inquisition |
| Great people | 22 | 177 individuals, recruitment, patronage, retirement, great works |
| Governors | 17 | Assignment, establishment, promotions, loyalty bonus |
| Era system | 11 | Historic moments, era advancement, ages |
| Loyalty | 13 | Pressure, revolt, occupation, governor bonus |
| Tourism + culture | 15 | Great works, tourism, cultural dominance |
| Victory conditions | 12+ | Score, Culture, Domination, Religious, Science, Diplomatic |
| Barbarians | 20+ | Camps, scouts, clans (hire/bribe/incite/raid), conversion |
| AI agent | 14 | HeuristicAgent, determinism, multi-turn simulation |
| Production gating | 10 | Tech gates, civ exclusivity, unit replacement at completion |
| Save/Load | 3 | JSON serialization, round-trip tests (serde feature) |
| apply_delta | 8 | State reconstruction from diffs, round-trip |
| Civ abilities | 13 | 19 civs with unique units/districts/improvements |
| World wonders | — | 29 wonders with costs, prereqs, effects |
| City-states | — | 25 with envoy/suzerain bonuses |
| Governments + policies | — | 10 governments, 113 policies with slot types |
| civsim TUI | — | 38+ commands, 100% RulesEngine coverage |
| Multiplayer server | — | Axum WebSocket, auth, game rooms |
| WASM frontend | — | Leptos app, hex renderer, tech/civic trees |

## Remaining Work

See [next-steps.md](next-steps.md) for details. The engine has reached full
base-game content parity. Remaining work is infrastructure:

| Item | Priority |
|---|---|
| RL training harness | High (core project goal) |
| Replay viewer UI | Medium |
| Natural wonder discovery event | Low |
| Performance optimization | Deferred |

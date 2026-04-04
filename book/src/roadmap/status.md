# Implementation Status

> **539 tests | 0 failures | 0 clippy warnings**

Open-civ-vi implements a complete Civ VI game engine in Rust with full content
parity across the base game, Rise & Fall, Gathering Storm, and all DLC
civilization packs.

## Content Inventory

| Category | Count | Sources |
|---|---|---|
| Civilizations | 50 | Base 19 · R&F 8 · GS 8 · DLC 15 |
| Units | 132 | Generic + all civ-unique |
| Buildings | 56 | All district chains with prereqs |
| World Wonders | 53 | Base 29 · R&F 8 · GS 7 · DLC 9 |
| Natural Wonders | 26 | Base 15 · GS 7 · DLC 4 |
| Improvements | 31 | Base 24 · GS 7 |
| Terrain Features | 12 | Base 8 · GS 4 |
| Technologies | 76 | 8 eras + GS Future Era |
| Civics | 60 | 8 eras + GS Future Era |
| Policies | 127 | All slot types + GS |
| Governments | 13 | Base 10 · GS Future 3 |
| City-States | 46 | Base 25 · GS 9 · DLC 12 |
| Great People | 177 | All 9 types × 8 eras |
| Promotions | 118 | 16 classes |
| Projects | 7 | Space race · climate · civic |
| Victory Types | 6 | Score · Culture · Domination · Religious · Science · Diplomatic |
| Alliance Types | 5 | Research · Military · Economic · Cultural · Religious |

## Systems

| System | Notes |
|---|---|
| Hex grid + pathfinding | libhexgrid crate — coordinates, Dijkstra, LOS |
| Map generation | Continents, climate zones, features, rivers, resources, starts |
| Combat | 6-source modifier pipeline, XP (era-scaled), promotions (+50 HP heal) |
| Production | Tech gating, civ exclusivity, unit/building/wonder/project completion |
| Trade routes | Domestic/international, autonomous traders, expiry |
| Diplomacy | War/peace, grievances, opinion status, 5 typed alliances with leveling |
| Religion | Pantheon, founding, spread, pressure, theological combat, inquisition |
| Great people | Points, auto-recruitment, gold/faith patronage, great works |
| Governors | Assignment, 5-turn establishment, promotions, yield/loyalty bonus |
| Era system | Historic moments, era advancement, Golden/Dark/Heroic ages |
| Loyalty | Pressure, revolt, occupation penalty, governor bonus |
| Tourism + culture | Great works, tourism accumulation, cultural dominance |
| Barbarian clans | Camps, scouts, hire/bribe/incite/raid, city-state conversion |
| Power grid (GS) | Power balance per city, CO2 tracking, fossil vs renewable |
| Climate + disasters (GS) | 7-stage sea level rise, 7 disaster types |
| World Congress (GS) | Periodic sessions, favor voting, diplomatic VP |
| City projects (GS) | Space race milestones, carbon recapture |
| Rock Band (GS) | Cultural combat, tourism generation, disband mechanic |
| Save/Load | JSON via serde feature flag (`save_game`/`load_game`) |
| apply_delta | State reconstruction from diffs (~70 delta variants) |
| Replay viewer | ReplayRecorder + ReplayViewer (serde-gated) |
| RL training harness | CivEnv gym-like API (reset, step, available_actions, reward) |
| AI agent | HeuristicAgent — deterministic baseline |
| open4x-cli | Non-REPL file-backed CLI + legacy TUI, 42 action types |
| Multiplayer server | Axum WebSocket, auth, game rooms (basic) |
| WASM frontend | Leptos hex renderer, tech/civic tree views (basic) |

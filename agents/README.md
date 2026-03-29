# Agent Skills

This directory contains reusable skill guides for AI coding agents working on the Open Civ VI codebase. Each file describes a common development pattern with step-by-step instructions, code templates, and checklists.

## Available Skills

| Skill | When to use |
|-------|-------------|
| [add-rules-engine-method](./add-rules-engine-method.md) | Adding a new game action to the `RulesEngine` trait |
| [write-integration-test](./write-integration-test.md) | Writing integration tests using the `Scenario` pattern |
| [add-game-content](./add-game-content.md) | Adding civilizations, units, buildings, improvements |
| [implement-roadmap-feature](./implement-roadmap-feature.md) | Picking up a feature from the implementation roadmap |
| [advance-turn-phase](./advance-turn-phase.md) | Adding per-turn processing to `advance_turn` |

## How Agents Use These

Reference the relevant skill file before starting a task. For example, if asked to implement governor assignment, read both `implement-roadmap-feature.md` and `add-rules-engine-method.md` first.

These skills complement `AGENTS.md` in the repository root, which provides project-wide architecture and conventions.

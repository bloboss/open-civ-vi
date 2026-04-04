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
| [make-todo](./make-todo.md) | Adding a tracked TODO (code comment + todo.md entry) |
| [modify-todo](./modify-todo.md) | Updating an existing TODO (description, priority, location) |
| [delete-todo](./delete-todo.md) | Removing a completed or obsolete TODO |

## How Agents Use These

Reference the relevant skill file before starting a task. For example, if asked to implement governor assignment, read both `implement-roadmap-feature.md` and `add-rules-engine-method.md` first.

The TODO skills ensure that code TODOs and the global todo list (`book/src/roadmap/todo.md`) stay in sync. Always use the skills rather than adding ad-hoc TODO comments.

These skills complement `AGENTS.md` in the repository root, which provides project-wide architecture and conventions.

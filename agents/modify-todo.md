# Skill: Modify TODO

Use this skill to update an existing TODO item — change its description,
priority, location, or status.

## Usage

Provide: the existing TODO identifier (file:line or description), and what
to change.

## Steps

### 1. Find the existing TODO

Search for the TODO in the codebase:
```bash
grep -rn "TODO.*<keyword>" libciv/src/
```

Also check `book/src/roadmap/todo.md` for the corresponding entry.

### 2. Update the code comment

Edit the `// TODO(<tag>): ...` comment at the source location. If the TODO
has moved to a different line/file, update both the old and new locations.

### 3. Update todo.md

Find the matching row in `book/src/roadmap/todo.md` and update:
- File:line reference (if moved)
- Description (if clarified)
- Priority (if changed)

### 4. If the TODO is partially done

Add a note to the description:
```rust
// TODO(GAMEPLAY): Implement eureka conditions — basic framework exists, needs trigger wiring
```

Update the todo.md entry to reflect partial progress.

## Example

**Input**: "Change the eureka TODO in tech.rs to high priority and note that the framework exists"

**Before**:
```rust
// TODO(GAMEPLAY): Implement eureka condition checking
```
**After**:
```rust
// TODO(GAMEPLAY): Wire eureka trigger conditions into advance_turn (framework exists in TechNode)
```

**todo.md**: Update priority from Medium to High, update description.

## Conventions

- Always update BOTH the code comment AND todo.md simultaneously
- If a TODO moves files, update the file:line reference in todo.md
- Never leave stale references — if the line number changes, fix it

# Skill: Delete TODO

Use this skill to remove a TODO item when the work is complete or the TODO
is no longer relevant.

## Usage

Provide: the TODO identifier (file:line, tag, or description keyword).

## Steps

### 1. Find the TODO

```bash
grep -rn "TODO.*<keyword>" libciv/src/
```

### 2. Remove or replace the code comment

**If the work is done**: Remove the `// TODO(...)` comment entirely, or
replace it with a brief implementation note:

```rust
// Before:
// TODO(GAMEPLAY): Implement eureka condition checking

// After (work done):
// Eureka conditions are checked in advance_turn Phase 3.
```

Or simply delete the comment if the code is self-explanatory.

**If the TODO is no longer relevant** (e.g., design changed): Delete the
comment with no replacement.

### 3. Remove from todo.md

Open `book/src/roadmap/todo.md` and delete the corresponding row from the
table.

### 4. Verify

- `grep -rn "TODO.*<keyword>" libciv/src/` should return no results
- The todo.md entry should be gone
- `cargo build --workspace` still passes

## Example

**Input**: "Delete the eureka TODO in tech.rs — it's implemented"

**Action**:
1. Remove `// TODO(GAMEPLAY): Implement eureka conditions` from `tech.rs:15`
2. Remove the `rules/tech.rs:15` row from `todo.md`
3. Verify build passes

## Conventions

- Always remove BOTH the code comment AND the todo.md entry
- Don't leave orphaned todo.md entries pointing to deleted comments
- Don't leave code TODOs that have no todo.md entry
- When completing a TODO, consider adding a test that validates the work

# Skill: Make TODO

Use this skill to add a tracked TODO item. It creates a code comment at the
specified location AND adds an entry to the global todo list at
`book/src/roadmap/todo.md`.

## Usage

Provide: file path, line number (or nearby context), description, and priority.

## Steps

### 1. Add code comment

Insert a `// TODO(<tag>): <description>` comment at the specified file:line.
Use a short tag that categorizes the work (e.g., `GAMEPLAY`, `PERF`, `CLEANUP`,
`CONTENT`, `SYSTEM`).

```rust
// TODO(GAMEPLAY): Implement eureka condition checking in advance_turn
```

If the line is inside a function body, place the comment on the line above.
If it's about a struct field or function signature, place it as a doc comment
or inline comment.

### 2. Add entry to todo.md

Open `book/src/roadmap/todo.md` and add a row to the appropriate table section.

```markdown
| `path/to/file.rs:42` | Description of the TODO | Priority |
```

Priority levels: **High**, **Medium**, **Low**, **Cleanup**

### 3. Verify consistency

- The tag in the code comment should match the section in todo.md
- The file:line in todo.md should point to the actual comment

## Example

**Input**: "Add eureka condition checking to tech.rs line 15, medium priority"

**Code** (`libciv/src/rules/tech.rs:15`):
```rust
// TODO(GAMEPLAY): Implement eureka condition checking in advance_turn
```

**todo.md** entry:
```markdown
| `rules/tech.rs:15` | Implement eureka condition checking in advance_turn | Medium |
```

## Conventions

- Tags: `GAMEPLAY`, `PERF`, `CLEANUP`, `CONTENT`, `SYSTEM`, `UI`, `TEST`
- Keep descriptions concise (one line, imperative mood)
- Never add TODOs for work that is already tracked elsewhere
- Run `cargo build --workspace` after to ensure the comment doesn't break syntax

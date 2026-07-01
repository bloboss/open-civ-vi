# Commit Types (TAGS)

Every commit in this repository uses [Conventional
Commits](https://www.conventionalcommits.org/) — `type(scope): subject`
— with a **mandatory scope** (see [SCOPES.md](./SCOPES.md)). The
allowed `type` values are the machine-readable list below, enforced by
`.githooks/commit-msg`.

<!-- TAGS:BEGIN -->
feat
fix
docs
style
refactor
perf
test
build
ci
chore
revert
<!-- TAGS:END -->

| type | when to use |
|------|-------------|
| `feat` | a new user- or API-facing capability |
| `fix` | a bug fix |
| `docs` | documentation only (book, README, comments-as-docs) |
| `style` | formatting / whitespace / lint-only, no behaviour change |
| `refactor` | code change that neither fixes a bug nor adds a feature |
| `perf` | a performance improvement |
| `test` | adding or correcting tests only |
| `build` | build system, dependencies, Docker, xtask recipes |
| `ci` | CI configuration / pipelines |
| `chore` | maintenance that doesn't fit above (deps bumps, ignores) |
| `revert` | reverts a previous commit |

Breaking changes: append `!` before the colon (`feat(protocol)!: …`)
and/or add a `BREAKING CHANGE:` footer.

## Enforcement

`.githooks/commit-msg` parses the first line and rejects the commit
unless the `type` is in the list above and the `scope` is in
`SCOPES.md`. It runs as the `commit-msg` hook via the pre-commit
framework (`.pre-commit-config.yaml`). Activate once per clone:

```
uv sync
uv run pre-commit install
```

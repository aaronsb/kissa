---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-100, ADR-101, ADR-103]
---

# ADR-105: Testing strategy

## Context

kissa operates on real filesystem state and real git repos. Testing needs to cover:

- Core logic (classification, filtering, graph traversal)
- Git operations via libgit2 (status, branches, remotes)
- SQLite persistence (index CRUD, schema migrations)
- CLI output formatting
- MCP protocol compliance
- Filesystem scanning (exclusions, mount boundaries, symlinks)

The challenge is that many of these involve side effects — real `.git` directories, real SQLite databases, real filesystem trees.

## Decision

### Framework

Use Rust's built-in test framework (`#[test]`, `#[cfg(test)]`) with `cargo test`. No external test runner.

Supplement with:
- `tempdir` / `tempfile` crates for isolated filesystem fixtures
- `git2` to programmatically create test repos (not shell out to `git init`)
- `assert_cmd` + `predicates` for CLI integration tests
- `insta` for snapshot testing of CLI and MCP output formats

### Test tiers

**Unit tests** (in-module `#[cfg(test)]`):
- Classification logic (category/ownership/intention inference)
- Filter matching and composition
- Graph edge detection (submodule parsing, dependency extraction)
- Difficulty level permission checks
- Config parsing and validation

**Integration tests** (`tests/` directory):
- Create temp directories with programmatic git repos via `git2`
- Run full scan → index → query cycles against real SQLite
- Test CLI commands via `assert_cmd` and snapshot output with `insta`
- Test MCP tool responses for format compliance (state tags, next hints)

**Fixture repos:**
- Built programmatically in test setup, not checked into the repo
- Represent specific scenarios: dirty repo, fork with upstream, nested repos, submodules, orphan, ancient repo
- Torn down automatically via `tempdir` drop

### What we don't test

- System `git` behavior (that's git's problem, not ours)
- libgit2 correctness (that's the `git2` crate's problem)
- Actual filesystem scan performance (benchmarks, not tests)

## Consequences

### Positive

- `cargo test` is the single test command — no test infrastructure to maintain
- Programmatic repo creation via `git2` is fast and deterministic
- Snapshot tests catch unintended output format changes (important for MCP contract stability)
- Temp directories mean tests are isolated and parallel-safe

### Negative

- Integration tests that create git repos are slower than pure unit tests
- Snapshot tests need manual review when output intentionally changes (`cargo insta review`)
- No end-to-end MCP client test (would require an MCP client harness)

### Neutral

- CI runs `cargo test` — no special test environment needed
- `insta` snapshots are checked into the repo as `.snap` files — they serve as documentation of expected output

## Alternatives Considered

- **External test framework (nextest)** — faster parallel execution, but adds a dependency for marginal benefit at kissa's scale. Can adopt later if test suite grows large.
- **Docker-based integration tests** — overkill for a local CLI tool. Temp directories are sufficient.
- **Mock git operations** — possible but fragile. Using real `git2` operations against temp repos is more reliable and tests the actual code path.
- **Property-based testing (proptest)** — useful for classification edge cases in the future, but not needed for initial development.

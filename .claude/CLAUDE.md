# kissa

**Finally herd your repos.**

A Rust CLI tool + MCP server that discovers, catalogues, and manages the topology of git repositories across your filesystem.

## Project Structure

```
docs/
  kissa-spec.md              # Original design spec (reference, not source of truth)
  architecture/              # ADRs (source of truth for decisions)
    adr.yaml                 # ADR domain config
  scripts/
    adr                      # ADR management tool
.claude/
  ways/                      # Project-local ways (contextual guidance)
  CLAUDE.md                  # This file
```

## Tech Stack

- **Language:** Rust
- **Git operations:** `git2` crate (libgit2 bindings) — never shell out to system git except via `exec` pass-through
- **Persistence:** SQLite (`rusqlite`)
- **CLI:** `clap`
- **Async:** `tokio` (for MCP server)

## Conventions

- ADR-driven development: architectural decisions get an ADR before implementation
- Conventional commits with scope
- PR-based workflow (even solo)
- The spec (`docs/kissa-spec.md`) is the vision document; ADRs are the binding decisions

## ADR Tool

```bash
docs/scripts/adr new core "Decision Title"    # Create new ADR in a domain
docs/scripts/adr list                         # List all ADRs
docs/scripts/adr status ADR-100 Accepted      # Update status
```

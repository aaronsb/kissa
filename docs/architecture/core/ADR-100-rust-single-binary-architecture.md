---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-101]
---

# ADR-100: Rust single-binary architecture

## Context

kissa is a CLI tool and MCP server that walks filesystems, parses git state, and manages a SQLite index. It needs to be fast on large directory trees, safe around git state, and trivially distributable (especially on Arch Linux via AUR).

The tool has two consumption modes — human CLI and machine MCP — sharing a common core. The interface layers should be thin wrappers.

## Decision

Build kissa in Rust as a single statically-linked binary with two entry modes:

```
kissa <command> [args]       # CLI mode
kissa --mcp                  # MCP server mode (stdio)
```

Both modes dispatch to the same core library. No logic lives in the interface layer.

### Key crate dependencies

- `git2` — libgit2 bindings (see ADR-101)
- `clap` — CLI parsing
- `rusqlite` — SQLite persistence (see ADR-103)
- `tokio` — async runtime for MCP server
- `serde` / `toml` / `serde_json` — config and protocol
- `walkdir` — filesystem traversal
- `colored` / `tabled` / `indicatif` — terminal output

## Consequences

### Positive

- Single binary distribution — ideal for AUR, GitHub releases, `cargo install`
- No runtime dependency on system git for core operations (libgit2 linked statically)
- Rust's ownership model is appropriate for a tool that manages filesystem and git state
- Mature CLI ecosystem (clap, colored, tabled, indicatif)
- Claude Code writes competent Rust

### Negative

- Compile times are slower than Go or TypeScript
- Smaller contributor pool than Python/TypeScript tooling
- FFI complexity with libgit2 (though `git2` crate handles this well)

### Neutral

- Targets Linux first (Arch), cross-platform is a non-goal for v0.x

## Alternatives Considered

- **Go** — faster compile, single binary, but `go-git` is less mature than libgit2 and the CLI ecosystem is thinner
- **Python** — fastest iteration, but distribution is painful (venvs, pip, system python conflicts) and performance on filesystem walks is inadequate
- **TypeScript/Node** — good MCP ecosystem, but runtime dependency, no static binary, poor filesystem performance at scale

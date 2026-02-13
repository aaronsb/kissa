---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-100, ADR-500]
---

# ADR-101: libgit2 for internal operations

## Context

kissa needs to inspect git repositories: status, branches, remotes, commit history, ahead/behind counts, dirty state. There are two approaches — shell out to system `git` or use libgit2 (via the `git2` Rust crate).

Security matters: kissa scans arbitrary directories that may contain malicious repos. A malicious `.git/hooks/` directory could execute code if git operations trigger hooks.

## Decision

All kissa-internal operations use `git2` (libgit2 bindings). kissa never shells out to system `git` for its own operations.

The single exception is the `exec` command, which is an explicit pass-through where the user deliberately requests a git command. `exec` uses `execvp`-style argument arrays (never `sh -c`) and filters commands against the repo's difficulty level (ADR-500) before executing.

### Boundary

| Operation type | Implementation | Hooks fire? |
|---|---|---|
| Scanning, status, log, diff, branches, remotes | `git2` (libgit2) | No |
| `kissa exec <repo> -- <git command>` | system `git` via arg array | Yes — intentional |

## Consequences

### Positive

- No git hooks fire during scan/status/inspection — safe against malicious repos
- No dependency on system git version or user aliases
- No shell injection surface — libgit2 uses native APIs, not string interpolation
- Consistent behavior across systems (no git config variation)

### Negative

- libgit2 doesn't support every git feature (e.g., sparse checkout, some rebase modes)
- `exec` requires system `git` to be installed; kissa warns if missing when `exec` is used
- Two code paths for git operations (libgit2 vs system git) adds conceptual complexity

### Neutral

- `exec` is documented as the boundary where hooks and aliases become active — users opt into this explicitly

## Alternatives Considered

- **System git for everything** — simpler single code path, but hooks fire on every operation, shell injection risk, depends on user's git version and config
- **libgit2 for everything, no exec** — safest, but users sometimes need real git commands (interactive rebase, complex merge, etc.) and kissa shouldn't try to reimplement those

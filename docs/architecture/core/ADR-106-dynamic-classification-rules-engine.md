---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related:
  - ADR-104
---

# ADR-106: Dynamic Classification Rules Engine

## Context

A first scan of a typical developer home directory reveals that nearly half the discovered repos are **tool-managed clones** — neovim plugins (lazy.nvim), SuperCollider quarks, FreeCAD mods, shader caches, etc. These repos are real git clones but the user has no direct relationship with them: they didn't clone them, don't commit to them, and don't care about their dirty/ahead/behind status.

ADR-104 defines a three-axis classification taxonomy (Category, Ownership, Intention) that can describe these repos (`ThirdParty` / `Clone` / `Dependency`), but there's no mechanism to **apply** those classifications automatically. Manually tagging 30+ neovim plugins is exactly the kind of tedium kissa exists to eliminate.

Beyond tool-managed repos, users have other classification patterns that are path-based, org-based, or heuristic-based:
- "Everything under `~/work/` is `ownership: work:acme`"
- "Repos owned by `rust-lang` org are `ownership: community`"
- "Repos with no commits in 2 years and no remote are candidates for archival"

The system needs to support **user-defined rules** that automatically classify repos, and it should be able to **propose rules** by detecting patterns in the index.

## Decision

Introduce a classification rules engine with three layers:

### 1. Config-defined rules (`[[classify]]` sections)

Users define rules in `config.toml` that match repos by pattern and set classification fields:

```toml
[[classify]]
match = { path = "~/.local/share/nvim/lazy/*" }
set = { intention = "dependency", ownership = "third-party" }
managed_by = "lazy.nvim"
tags = ["nvim", "plugin"]

[[classify]]
match = { path = "~/work/*", org = "acme-corp" }
set = { ownership = "work:acme" }

[[classify]]
match = { org = "rust-lang" }
set = { ownership = "community", intention = "reference" }
```

**Match criteria** (AND-combined within a rule):
- `path` — glob pattern against repo path
- `org` — remote org/owner name
- `name` — repo name pattern
- `has_remote` — boolean
- `is_bare` — boolean

**Settable fields:**
- `category`, `ownership`, `intention` — the ADR-104 axes
- `tags` — appended (not replaced)
- `managed_by` — new field: name of the tool managing this repo
- `state` — can set to `Active` or override to a custom lifecycle state

Rules are evaluated in order; first match per field wins (later rules can set fields that earlier rules left unset, but can't override).

### 2. Built-in heuristics

kissa ships with a set of well-known managed-repo path patterns that are applied as low-priority defaults (user rules always win):

| Pattern | Classification |
|---------|---------------|
| `~/.local/share/nvim/lazy/*` | managed_by: lazy.nvim |
| `~/.local/share/nvim/site/pack/*/start/*` | managed_by: nvim-pack |
| `~/.vim/plugged/*` | managed_by: vim-plug |
| `~/.local/share/SuperCollider/downloaded-quarks/*` | managed_by: SuperCollider |
| `~/.cargo/git/checkouts/*` | managed_by: cargo |
| `*/.git/modules/*` | managed_by: git-submodule |

These heuristics are code, not config — they're maintained as the tool matures and community patterns emerge.

### 3. Rule suggestion engine (`kissa classify --suggest`)

kissa can analyze the index and propose rules by detecting:

- **Path clustering**: many repos sharing a common parent directory (e.g., 32 repos under `~/.local/share/nvim/lazy/`)
- **No user commits**: repos where the user's configured identity never appears in the commit log
- **Shallow clones or detached HEADs**: typical of package manager behavior
- **Uniform freshness**: cluster of repos all at the same stale/dormant tier

The suggest command outputs proposed `[[classify]]` blocks that the user can review and add to their config.

### 4. `managed_by` as a first-class field

Add `managed_by: Option<String>` to the `Repo` struct. This captures which tool owns the repo's lifecycle. When set:
- Default list output hides managed repos (show with `--managed` or `--all`)
- Freshness/dirty/ahead warnings are suppressed (the tool manages updates)
- `kissa list --managed-by lazy.nvim` shows just that tool's repos
- Summary output shows managed repo count separately

### 5. Classification application timing

Rules are applied:
- **On scan** — newly discovered repos get classified immediately
- **On `kissa classify`** — re-runs rules against all indexed repos
- **On config change** — `kissa classify --reapply` re-evaluates everything

User-set classifications (via future `kissa tag` / `kissa set` commands) take precedence and are marked as `user_override: true` so rule re-application doesn't clobber them.

## Consequences

### Positive

- First scan produces a useful, organized index instead of a flat list polluted with plugin noise
- Users define classification once; every re-scan applies it automatically
- `managed_by` gives kissa genuine topology insight — "you have 32 nvim plugins, 4 SuperCollider quarks, 3 cargo checkouts"
- Rule suggestion makes the system self-bootstrapping for new users
- Extensible: community can share rule sets for common tool ecosystems

### Negative

- Rule evaluation order matters — needs clear documentation
- Built-in heuristics need maintenance as plugin managers evolve
- Risk of over-classification: rules might misclassify repos at pattern boundaries

### Neutral

- Requires new config schema (`[[classify]]` table array)
- `managed_by` adds a column to the repos table (schema migration)
- Filter system needs `managed_by` and `--managed`/`--all` flags
- MCP tools need corresponding parameters

## Alternatives Considered

- **Exclusion-only approach**: Add managed paths to scan exclusions. Simpler but loses topology information — kissa can't tell you what plugins you have installed. Rejected because tracking is more valuable than ignoring.

- **Manual tagging only**: Let users tag repos one by one. Doesn't scale — the whole point is that 38 out of 81 repos needed classification and doing it manually defeats the purpose.

- **Regex-based rules**: More powerful than globs but harder to write and read. Globs cover the path-matching use case well enough; org/name matching is exact string. Could add regex support later if needed.

- **ML-based auto-classification**: Interesting for the future but overkill for v0.x. The pattern-based approach handles the 90% case. Could layer ML suggestions into the `--suggest` engine later.

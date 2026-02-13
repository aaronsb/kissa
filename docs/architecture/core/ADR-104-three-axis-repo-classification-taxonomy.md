---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-102]
---

# ADR-104: Three-axis repo classification taxonomy

## Context

A flat list of repos is useless for organization. kissa needs to understand *what kind of repo this is*, *whose it is*, and *why you have it* to propose sensible filesystem layouts. These classifications drive the `organize` feature — kissa's primary value proposition.

## Decision

Every repo gets three independent classifications, inferred automatically from git metadata and enrichable via `.kissa` files:

### Category: What is it?

| Category | Detection |
|----------|-----------|
| `origin` | Remote URL contains your username, or no remote |
| `clone` | Remote origin doesn't match your username, no fork relationship |
| `fork` | Your remote + an `upstream` remote pointing elsewhere |
| `mirror` | Bare repo or mirror config |

### Ownership: Whose is it?

| Ownership | Detection |
|-----------|-----------|
| `personal` | Remote URL matches configured personal username/org |
| `work:<label>` | Remote URL matches a configured work org |
| `community` | Remote URL belongs to a known OSS org you contribute to |
| `third-party` | Remote URL doesn't match any configured identity |
| `local` | No remote |

Ownership relies on an identity config where users declare their usernames and work orgs.

### Intention: Why do you have it?

| Intention | Detection heuristic |
|-----------|-------------------|
| `developing` | Your commits on non-default branches, active working tree |
| `contributing` | Fork + diverging branches from upstream |
| `reference` | Third-party clone, no local branches or commits |
| `dependency` | Another repo's manifest references this path |
| `dotfiles` | Config files at repo root, or in `~/.config` |
| `infrastructure` | Contains terraform, ansible, k8s, docker-compose |
| `experiment` | Local-only, few commits, no tags, no CI config |
| `archived` | No commits in 6+ months, clean tree, default branch only |

Intention is probabilistic — kissa picks the best match and surfaces confidence. `.kissa` files can override.

## Consequences

### Positive

- Organization patterns can match on any axis or combination: "all work:initech repos that I'm actively developing"
- Classification is queryable via structured filters (ADR-102)
- Inference-first means kissa is useful with zero configuration
- `.kissa` overrides let users correct wrong inferences without fighting the tool

### Negative

- Intention inference will be wrong sometimes — especially for repos that straddle categories (is it an experiment or a personal project?)
- Identity config is required for ownership to be meaningful beyond personal/local
- Three axes means more complexity in the matching logic for organization patterns

### Neutral

- Classifications are node properties in the graph model (ADR-102), stored in the SQLite index (ADR-103)
- The taxonomy can be extended with new values without breaking existing classifications

## Alternatives Considered

- **Single classification axis** (e.g., just "work/personal/oss") — too coarse. A work fork you're contributing to and a work repo you created need different handling.
- **Free-form tags only** — flexible but doesn't enable automatic inference or pattern-based organization. Tags complement classification but don't replace it.
- **LLM-based classification** — interesting for v2+ but adds a dependency and latency. The heuristic approach covers the common cases.

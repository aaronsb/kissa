---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-101, ADR-301]
---

# ADR-500: Difficulty levels as permission model

## Context

kissa operates on git repos that may contain critical code. An MCP-connected agent shouldn't have the same permissions as a human at a terminal. A production repo shouldn't allow the same operations as an experiment.

This is a safety model, not an auth model — it controls what operations kissa will perform, not who can invoke them.

## Decision

Five difficulty levels control allowed operations per repo:

| Level | Allowed | Use case |
|-------|---------|----------|
| **readonly** | status, log, diff, branch list, remote list | Safe browsing. Default for MCP. |
| **fetch** | + fetch, pull, checkout existing branches | Updating local state. |
| **commit** | + add, commit, push, create branch, merge (ff-only) | Normal development. Default for CLI. |
| **force** | + rebase, force push, reset, delete branches | Power user. Explicit opt-in. |
| **unsafe** | + clean -fdx, repo deletion, destructive rewrites | Per-repo opt-in only. Confirmation required. |

### Defaults

- CLI: `commit`
- MCP: `readonly`
- Per-path overrides via glob patterns in config

### Always blocked (regardless of level)

- Delete a repo with unpushed commits without multi-step confirmation
- Force push to `main`, `master`, or `production` without explicit confirmation
- `clean -fdx` on a repo with untracked files that look important (non-generated, non-build-artifact heuristic)
- Operate on repos outside configured scan roots

### Cat mode

When `cat_mode = true`, difficulty names are remapped cosmetically: napping, purring, hunting, zoomies, knocking-things-off-the-counter. Behavior is identical.

## Consequences

### Positive

- MCP agents are safe by default — they can observe everything but modify nothing without config escalation
- Per-repo overrides let users protect production repos while allowing experimentation elsewhere
- The "always blocked" rules catch genuinely dangerous operations even at high difficulty levels

### Negative

- Users who want an MCP agent to actually do things must adjust config (intentional friction)
- The heuristic for "important-looking untracked files" may be wrong sometimes

### Neutral

- `exec` command filters against difficulty level before invoking system git — a force push attempt at `commit` level is rejected before git is called
- Difficulty escalation is a natural elicitation point for agents ("this needs force level — approve?")

## Alternatives Considered

- **No permission model** — trust the user to not do anything stupid. Fine for a personal tool, dangerous when agents are involved.
- **Capability-based model** (fine-grained: can_push, can_rebase, can_delete_branch) — more flexible but harder to reason about. The tiered model is simpler and covers real usage patterns well.
- **Separate user/agent permission systems** — adds complexity. A single model with different defaults (CLI=commit, MCP=readonly) achieves the same outcome.

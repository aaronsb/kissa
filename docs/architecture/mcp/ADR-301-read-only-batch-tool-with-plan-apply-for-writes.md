---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-300, ADR-500]
---

# ADR-301: Read-only batch tool with plan/apply for writes

## Context

MCP tool calls are expensive in round-trip latency. An agent triaging dirty repos might need 4-5 sequential tool calls (list, status, status, freshness) just to build a picture. Batching reads into a single call reduces this to one round-trip.

However, batching *writes* is dangerous. If command 3 of 8 fails in a write batch, the system is in a partially-mutated state. Rollback logic for arbitrary command sequences is complex and error-prone.

## Decision

The `run` MCP tool accepts **only read-only commands** and executes them in sequence within a single tool call.

### Read-only commands (allowed in `run`)

`list_repos`, `repo_status`, `freshness`, `search`, `related`, `deps`, `doctor`, `get_config`

### Write commands (rejected by `run`)

`exec`, `tag`, `organize`, `apply_plan`, `move`, `scan` — these are rejected with an explanation directing the agent to use the dedicated tool.

### Write operations go through their own paths

- **Single writes**: `exec`, `tag`, `move` — individual tool calls with explicit visibility
- **Batch writes**: `organize` → `apply_plan` — the plan/review/execute cycle with a named transaction, user approval, and atomic execution with rollback

## Consequences

### Positive

- Read batches are safe — idempotent, no partial failure risk
- Agents build a complete picture in one round-trip, then reason about what to do
- Write operations are individually visible to the user
- Batch writes go through the plan/apply cycle which has proper rollback semantics

### Negative

- Agents can't batch "fetch all 5 repos" in one MCP call — they use `exec` individually or use CLI via shell access
- Slightly more tool calls for write-heavy workflows

### Neutral

- Agents with shell access (Claude Code, Cursor) can always bypass MCP and run `kissa exec --all -- fetch` directly (the "coding agent escape hatch")

## Alternatives Considered

- **Unrestricted batch** (`run` allows reads and writes with `on_error: stop|continue`) — flexible but partial-failure states are hard to reason about and harder to recover from. The complexity isn't worth it when the plan/apply pattern already handles batch writes properly.
- **No batch tool** — forces agents to make N sequential tool calls for reconnaissance. Significant latency penalty for a common workflow (triage).

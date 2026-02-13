---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-301, ADR-102]
---

# ADR-300: Terse text MCP responses with state tags

## Context

kissa's MCP server needs to return tool responses that LLMs can reason about efficiently. LLMs read text; they don't parse JSON efficiently. Every token spent navigating structure is a token not spent reasoning about what to do next.

The MCP protocol allows tool responses to be arbitrary text. The question is what format.

## Decision

MCP tool responses return **terse, human-readable text with semantic hints** — not structured JSON.

### Response pattern

```
[state_tag] concise natural language summary
key details, one per line, minimal decoration
only what the agent needs to reason about the next step

→ next: suggested_tool_1 | suggested_tool_2
? ask user: decision that can't be inferred
```

- **`[state_tag]`** — lightweight state machine signal (e.g., `[listing]`, `[status]`, `[blocked]`, `[plan_ready]`)
- **`→ next:`** — suggests logical follow-up tools without enforcing a flow
- **`? ask user:`** — hints that this is a good moment for elicitation

### State tags

| Tag | Meaning |
|-----|---------|
| `[scan_complete]` | Scan finished, index updated |
| `[listing]` | Filtered repo list follows |
| `[status]` | Single repo detail |
| `[deps]` | Dependency graph follows |
| `[related]` | Relationship data follows |
| `[plan_ready]` | Organization plan generated, awaiting review |
| `[plan_applied]` | Plan executed successfully |
| `[moved]` | Repo move completed |
| `[executed]` | Git command ran successfully |
| `[blocked]` | Operation denied by difficulty or safety rule |
| `[warning]` | Something looks wrong, agent should investigate |
| `[error]` | Something failed |
| `[batch]` | Batch of read commands completed |

### CLI vs MCP

The CLI renders the same underlying data with semantic terminal colors and formatting. The MCP responses are a text-only projection of the same output.

## Consequences

### Positive

- Minimal token overhead — agents get straight to reasoning
- State tags give agents enough structure to track workflow state
- Next-step hints reduce unnecessary tool exploration
- Elicitation hints (`? ask user:`) cue agents to pause for user input at the right moments

### Negative

- Not machine-parseable by non-LLM tools (CLI `--format json` covers this case)
- State tags are a convention, not a schema — no compile-time guarantees on format

### Neutral

- The CLI `--format json` flag provides structured output for scripting and piping; the MCP path doesn't need to duplicate this

## Alternatives Considered

- **JSON responses** — standard for APIs, but LLMs waste tokens parsing nested structure. Structured output is available via CLI `--format json` for non-LLM consumers.
- **Markdown** — richer formatting but adds noise (headers, tables) that doesn't help agent reasoning. kissa's responses are short enough that plain text with conventions suffices.

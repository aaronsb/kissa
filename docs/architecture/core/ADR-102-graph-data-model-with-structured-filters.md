---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-103, ADR-300]
---

# ADR-102: Graph data model with structured filters

## Context

kissa's differentiator is that it sees relationships between repos — submodules, dependencies, forks, duplicates, siblings by org. A flat list of repos with properties is insufficient; the data model needs to express edges between nodes.

The question is how to expose this graph to users and agents. Options range from a full query language (openCypher parser) to simple CLI flags.

## Decision

Model the data as an openCypher-inspired graph internally (nodes = repos, typed edges = relationships) but expose it through **structured filters and dedicated relationship commands**, not a query language parser.

### Data model (internal)

**Nodes:** `(:Repo)` with properties — name, path, dirty, freshness, org, ownership, intention, category, etc.

**Edges:** `[:SUBMODULE]`, `[:NESTED]`, `[:SIBLING]`, `[:DEPENDS_ON]`, `[:FORK_OF]`, `[:DUPLICATE]`

### Interface (external)

- **Property filters** via CLI flags and MCP tool parameters: `--dirty`, `--org initech`, `--freshness stale`, `--unpushed`
- **Relationship commands**: `kissa deps <repo>`, `kissa related <repo>`, `kissa list --duplicates`
- Filters compose with AND semantics

### Future migration path

The graph data model stored in SQLite (nodes table + edges table) maps directly to a graph backend like Apache AGE if the complexity ever warrants it. The interface stays the same — structured filters compile to SQL today and could compile to Cypher/GQL tomorrow.

## Consequences

### Positive

- No custom parser to build or maintain
- CLI flags are discoverable (`--help`) and composable
- MCP tool parameters are self-documenting (JSON schema)
- Covers 90%+ of real-world queries
- Data model is graph-native from day one — no schema migration needed if we add a real graph backend

### Negative

- Complex multi-hop traversals (e.g., "transitive dependencies 3 levels deep") require dedicated commands rather than ad-hoc queries
- Adding a new filter requires code changes, not just a query string

### Neutral

- The graph vocabulary (nodes, edges, properties) is used in documentation and internal code even though there's no query language — it's a thinking tool, not an API

## Alternatives Considered

- **openCypher subset parser** — agents know the syntax from training data, but no Rust crate exists for standalone parsing. Building a hand-rolled parser is significant effort for marginal benefit over structured filters. Revisit if Apache AGE enters the picture.
- **Custom filter DSL** — `kissa query "dirty AND org:initech"` — invents a language nobody knows and still requires a parser. Worse than both alternatives.
- **GraphQL** — overkill for a local CLI tool. Appropriate for a hosted service, not a single-binary unix tool.

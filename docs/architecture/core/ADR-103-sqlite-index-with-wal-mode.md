---
status: Draft
date: 2026-02-13
deciders:
  - aaronsb
related: [ADR-102]
---

# ADR-103: SQLite index with WAL mode

## Context

kissa needs persistent state: the repo inventory, graph edges, scan metadata, plan history, user tags. This state must survive restarts, support concurrent reads and writes (MCP server answering queries while a scan updates), and be trivially portable (single file, no server process).

## Decision

Use SQLite via `rusqlite` for all persistent state, stored at `~/.local/share/kissa/index.db` (XDG Base Directory spec). Enable WAL (Write-Ahead Logging) mode for concurrent read/write access.

### Schema design

The schema models the graph explicitly:

- **repos table** — node properties (path, name, org, dirty, freshness, classification, etc.)
- **edges table** — typed relationships (source_id, target_id, edge_type, metadata)
- **scans table** — scan history and per-repo last-verified timestamps
- **plans table** — organization plan history (for audit and rollback)
- **tags table** — user-defined tags (many-to-many)

Graph queries (ADR-102 structured filters) compile to SQL WHERE clauses on repos, with JOINs to edges for relationship queries.

## Consequences

### Positive

- Survives restarts with no cold-start penalty
- Single file — easy to back up, nuke and rebuild, move between machines
- Handles hundreds of repos without issue
- WAL mode allows MCP reads concurrent with scan writes
- Graph queries compile naturally to SQL joins

### Negative

- Not a real graph database — multi-hop traversals need application-level code or recursive CTEs
- Single-writer semantics (WAL allows one writer at a time) — fine for kissa's workload but would be a bottleneck in a multi-user scenario

### Neutral

- XDG paths mean the database location is predictable and discoverable
- Future migration to Apache AGE + PostgreSQL would change the persistence layer but not the interface (ADR-102)

## Alternatives Considered

- **JSON file** — simpler, but doesn't support concurrent access, queries become linear scans, and grows unwieldy past ~100 repos
- **PostgreSQL / Apache AGE** — proper graph database, but requires a running server process. Appropriate for a future hosted version, overkill for a local CLI tool. The SQLite schema is designed so this migration is clean if needed.
- **sled / RocksDB** — embedded key-value stores, but the data is naturally relational/tabular and SQL is the right query interface for structured filters

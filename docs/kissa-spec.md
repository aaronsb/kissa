# kissa

**Finally herd your repos.**

`kissa` is a standalone tool that discovers, catalogues, and manages the topology of git repositories scattered across your filesystem. It serves both as a CLI for humans and an MCP server for AI agents, with a shared core that treats your local git universe as a queryable, actionable graph.

---

## Core Purpose

Engineers accumulate repositories like entropy. They clone into `~/Downloads`, start experiments in `/tmp`, nest projects inside projects, and forget about repos for months. GitHub knows what you pushed. Your filesystem knows where files are. Neither sees the full picture — the dirty working trees, the abandoned experiments, the thing you started at 2am and never committed.

kissa is the catalogue of your local git universe. It sees everything, it maps relationships, and it gives both you and your AI agents a structured way to understand and act on the landscape.

### The Killer Feature

kissa's primary value proposition is **AI-assisted repo organization**. An agent connects via MCP, scans your filesystem, classifies every repo it finds (by remote origin, language, activity, relationships), and proposes a structured layout. It builds a migration plan as a kissa transaction — a batch of moves you review and approve before anything happens.

This isn't a one-time operation. kissa watches for new repos appearing in chaotic locations and can suggest where they belong.

---

## Architecture

### Dual Interface Pattern

kissa is a single compiled binary with two consumption modes:

```
kissa <command> [args]       # CLI mode — human at a terminal
kissa --mcp                  # MCP server mode — agent over stdio
kissa --mcp --transport tcp  # MCP server mode — remote over TCP (v2)
```

Both interfaces share a common core. The CLI renders human-readable output with semantic terminal colors. The MCP server exposes the same operations as tools and resources.

### Internal Structure

```
src/
  core/
    scanner.rs        # filesystem walking, .git discovery
    index.rs          # repo inventory, SQLite persistence, queries
    repo.rs           # per-repo operations via git2/libgit2 (status, log, refs, remotes)
    graph.rs          # relationship mapping, topology, structured filters
    planner.rs        # transaction/plan builder for batch operations
    permissions.rs    # difficulty levels, command filtering
    exec.rs           # git pass-through (the ONLY place that shells out to git)

  cli/
    main.rs           # clap CLI, command dispatch
    display.rs        # terminal rendering, semantic colors, tables

  mcp/
    server.rs         # MCP protocol, tool/resource definitions
    transport.rs      # stdio, tcp (v2)

  config/
    mod.rs            # XDG config loading, per-repo overrides
```

### Language

Rust. Rationale:

- Compiles to a single static binary (ideal for AUR, distribution)
- `git2` crate provides mature, safe libgit2 bindings — no system `git` dependency for core operations
- Excellent CLI ecosystem (clap, colored, tabled, indicatif)
- Claude Code writes competent Rust
- Appropriate for a tool that walks filesystems and manages git state
- The unix tool vibe

### Dependencies (crate-level)

- `git2` (libgit2 bindings) — **first-class git interface**. All repo inspection, status, diff, log, branch operations, ref walking, and remote URL parsing go through `git2`. kissa never shells out to `git` for its own operations. This is a deliberate security and correctness choice: libgit2 doesn't execute hooks, doesn't read user git aliases, and doesn't depend on the system git version.
- `clap` — CLI argument parsing
- `serde` / `toml` — config and .kissa file parsing
- `serde_json` — JSON round-tripping, MCP protocol
- `colored` / `owo-colors` — semantic terminal colors
- `tabled` — terminal tables
- `indicatif` — progress bars for scanning
- `walkdir` — filesystem traversal
- `dirs` — XDG path resolution
- `tokio` — async runtime (for MCP server)
- `rusqlite` — SQLite index

---

## The Repo Graph

kissa doesn't maintain a flat list — it builds a graph. Every discovered repo is a node with vitals and edges.

### Node: Repo Vitals

Each repo in the index carries the following properties, all extracted via `git2` (libgit2) — no system `git` required:

| Property | Source | Description |
|----------|--------|-------------|
| `path` | filesystem | Absolute path to repo root |
| `remotes` | git2 | Remote names and URLs |
| `origin_org` | inferred | GitHub/GitLab org or user from remote URL |
| `default_branch` | git2 | HEAD reference |
| `current_branch` | git2 | Currently checked out branch |
| `branches` | git2 | Local and remote branch count |
| `stale_branches` | git2 | Branches fully merged into default |
| `dirty` | git2 | Working tree has uncommitted changes |
| `staged` | git2 | Index has staged changes |
| `untracked` | git2 | Has untracked files |
| `ahead` | git2 | Commits ahead of remote tracking branch |
| `behind` | git2 | Commits behind remote tracking branch |
| `last_commit` | git2 | Timestamp of most recent commit |
| `freshness` | derived | Human category: active / stale / dormant / ancient |
| `languages` | inferred | Detected from file extensions, config files |
| `size_kb` | filesystem | Working tree size |
| `has_dotkissa` | filesystem | Whether a .kissa file exists |
| `tags` | .kissa | User-defined tags |
| `project` | .kissa | Logical project grouping |
| `role` | .kissa | Role within a project (api, frontend, infra, etc.) |

### Freshness Tiers

| Tier | Criteria | Terminal Color |
|------|----------|---------------|
| **active** | Commit within 7 days | green |
| **recent** | Commit within 30 days | cyan |
| **stale** | Commit within 90 days | yellow |
| **dormant** | Commit within 365 days | red |
| **ancient** | No commit in over a year | dim / gray |

### Edges: Repo Relationships

| Relationship | Detection Method |
|-------------|-----------------|
| **submodule** | `.gitmodules` parsing |
| **nested** | Repo physically inside another repo (not a submodule) |
| **sibling** | Same remote org/owner |
| **dependency** | Cross-references in Cargo.toml, package.json, go.mod, etc. pointing to local paths |
| **fork** | Remote URL matches another repo's remote (different fork) |
| **duplicate** | Same remote URL, different local paths |

---

## The .kissa File

An optional TOML file at the repo root. kissa can infer most things, but this lets you correct or enrich.

```toml
[identity]
project = "initech-platform"
role = "api-gateway"
tags = ["work", "production", "rust", "critical"]

[relationships]
depends-on = ["initech-shared-lib", "initech-proto"]

[organization]
# Hint for the AI organizer — where this repo "wants" to live
preferred-path = "~/code/work/initech/"

[permissions]
# Override the global difficulty for this specific repo
difficulty = "readonly"
```

kissa should never *require* this file. It's enrichment, not configuration.

---

## Repo Classification & Organization

### The Problem

A flat list of repos is useless for organization. kissa needs to understand *what kind of repo this is*, *whose it is*, and *why you have it* in order to propose sensible filesystem layouts. These classifications are inferred first from git metadata, enriched by `.kissa` files, and overridden by user preferences.

### Classification Taxonomy

Every repo gets three independent classifications: **category**, **ownership**, and **intention**. These are inferred automatically but can be overridden.

#### Category: What Is It?

Derived from remote URLs, file analysis, and git topology.

| Category | Detection | Notes |
|----------|-----------|-------|
| `origin` | You created it. Remote URL contains your username, or no remote. | Your own project. |
| `clone` | Cloned from someone else's repo. Remote origin doesn't match your username. No fork relationship. | You're reading, building, or using it locally. |
| `fork` | Remote origin is yours, but an `upstream` remote exists pointing elsewhere. Or: GitHub API fork metadata if `.kissa` provides it. | You forked it to contribute or customize. |
| `mirror` | Bare repo, or remote configured as mirror. | Reference copy. Rare for most users. |

#### Ownership: Whose Is It?

Derived from remote URLs and configured identity mapping.

| Ownership | Detection | Notes |
|-----------|-----------|-------|
| `personal` | Remote URL matches a configured personal username/org. | Your stuff. |
| `work:<org>` | Remote URL matches a configured work org. | Employer's stuff. Users can configure multiple work orgs. |
| `community` | Remote URL belongs to a known open-source org you contribute to but don't own. | OSS work. |
| `third-party` | Remote URL doesn't match any configured identity. | Someone else's code on your machine. |
| `local` | No remote at all. | Experiments, scratch work, offline projects. |

The key enabler is the `[identity]` config section where users declare "these are my usernames, these are my work orgs":

```toml
[identity]
# Your usernames across platforms
usernames = ["myuser", "myuser-work"]

# Organizations you consider "work" (multiple employers, contracts, etc.)
work_orgs = [
  { name = "initech", platform = "github.com", label = "initech" },
  { name = "initech", platform = "gitlab.com", label = "initech" },
  { name = "initrode", platform = "github.com", label = "initrode" },
  { name = "vandelay-industries", platform = "github.com", label = "vandelay" },
]

# Organizations you contribute to but don't own
community_orgs = [
  "rust-lang",
  "tokio-rs",
  "apache",
]
```

With this, kissa can look at a remote URL like `git@github.com:initech/api-gateway.git` and classify it as `work:initech`, while `git@github.com:initrode/migration-tool.git` becomes `work:initrode` and `git@github.com:vandelay-industries/latex-exporter.git` becomes `work:vandelay`. Different contracts, different orgs, one catalogue.

#### Intention: Why Do You Have It?

This is the trickiest to infer, but the most useful for organization. Derived from activity patterns and topology.

| Intention | Detection Heuristic | Notes |
|-----------|-------------------|-------|
| `developing` | You have commits on non-default branches, or recent commits by your identity. Working tree is active. | You're building this. |
| `contributing` | Fork category + commits on branches diverging from upstream. | You're sending PRs. |
| `reference` | Third-party clone, no local branches beyond default, no local commits. | You cloned it to read or build. |
| `dependency` | Another repo's package manifest references this path. | Local dependency for a build. |
| `dotfiles` | Repo root contains config files (.bashrc, .zshrc, etc.) or is in `~/.config`. | Your dotfiles. |
| `infrastructure` | Contains terraform, ansible, kubernetes, docker-compose, CI configs. | Infra-as-code. |
| `experiment` | Local-only, few commits, no tags, no CI config. Short commit history. | A scratch repo. |
| `archived` | No commits in 6+ months, clean working tree, default branch only. | You're probably done with it. |

These are probabilistic — kissa picks the best match and surfaces its confidence. The `.kissa` file can override:

```toml
[identity]
intention = "developing"    # override inference
```

### Organization Patterns

Patterns are opinionated filesystem layouts. Each pattern is a template that maps classification → path. Users pick a pattern (or build their own), and kissa uses it to generate organization plans.

#### Built-in Patterns

**Pattern: `platform`** — mirrors the git hosting platform structure

```
~/Projects/
  github.com/
    myuser/
      my-project/
      my-other-project/
    initech/
      api-gateway/
      frontend/
    initrode/
      migration-tool/
    vandelay-industries/
      latex-exporter/
    rust-lang/
      rust/           # third-party clone
  gitlab.com/
    initech/
      internal-tool/
```

Good for: people who work across many platforms and orgs, want a 1:1 map to remote URLs. Similar to `ghq` layout.

**Pattern: `role`** — groups by your relationship to the code

```
~/Projects/
  work/
    initech/
      api-gateway/
      frontend/
      infra/
    initrode/
      migration-tool/
      legacy-app/
    vandelay/
      latex-exporter/
  personal/
    my-project/
    my-other-project/
  contrib/
    rust-lang/
      rust/
    tokio-rs/
      tokio/
  reference/
    some-cool-lib/
  experiments/
    scratch-thing/
  archive/
    old-project/
```

Good for: people who think in terms of "work vs personal vs OSS contributions." Clear mental model.

**Pattern: `project`** — groups by logical project, collocating related repos

```
~/Projects/
  initech-platform/       # logical project grouping
    api/
    frontend/
    shared-lib/
    proto/
    infra/
  personal-site/
    site/
    content/
  experiments/
    rusty-thing/
  dotfiles/
```

Good for: people who work on multi-repo projects and want related repos adjacent. Requires `.kissa` project metadata or inference from shared org + naming patterns.

**Pattern: `hybrid`** (default) — combines role at the top level with platform/org underneath

```
~/Projects/
  work/
    initech/                # label from work_orgs config
      api-gateway/
      frontend/
      shared-lib/
    initrode/               # the contract gig
      migration-tool/
      legacy-app/
    vandelay/               # import/export, obviously
      latex-exporter/
  personal/
    github.com/
      myuser/
        my-project/
    gitlab.com/
      myuser/
        private-thing/
  contrib/
    github.com/
      rust-lang/
        rust/
  reference/
    some-lib/
  experiments/
    scratch/
  dotfiles/
  archive/
```

Good for: most people. Separates concerns at the top, preserves platform context underneath.

#### Custom Patterns

Users can define their own pattern in config using a template syntax:

```toml
[organization]
pattern = "hybrid"          # use a built-in pattern, OR:
base_path = "~/Projects"

# Custom rules (evaluated top to bottom, first match wins)
[[organization.rules]]
match = { ownership = "work:initech" }
path = "work/initech/{repo_name}"

[[organization.rules]]
match = { ownership = "work:initech" }
path = "work/initech/{repo_name}"

[[organization.rules]]
match = { ownership = "personal", intention = "dotfiles" }
path = "dotfiles/{repo_name}"

[[organization.rules]]
match = { ownership = "personal" }
path = "personal/{platform}/{username}/{repo_name}"

[[organization.rules]]
match = { intention = "contributing" }
path = "contrib/{platform}/{org}/{repo_name}"

[[organization.rules]]
match = { intention = "reference" }
path = "reference/{repo_name}"

[[organization.rules]]
match = { intention = "experiment" }
path = "experiments/{repo_name}"

[[organization.rules]]
match = { intention = "archived" }
path = "archive/{repo_name}"

# Catch-all (required)
[[organization.rules]]
match = {}
path = "other/{platform}/{org}/{repo_name}"
```

**Template variables** available in path templates:

| Variable | Source | Example |
|----------|--------|---------|
| `{repo_name}` | Last segment of remote URL or directory name | `api-gateway` |
| `{org}` | Organization/owner from remote URL | `initech` |
| `{platform}` | Hostname from remote URL | `github.com` |
| `{username}` | Your username on this platform (from identity config) | `myuser` |
| `{label}` | Label from work_orgs config | `initech` |
| `{project}` | Project name from .kissa or inference | `initech-platform` |
| `{category}` | Inferred category | `fork` |
| `{ownership}` | Inferred ownership | `personal` |
| `{intention}` | Inferred intention | `developing` |
| `{languages.0}` | Primary detected language | `rust` |

### Upstream & Fork Awareness

kissa understands the topology of forks, upstreams, and clones. This is critical for correct classification and organization.

| Scenario | Remote Config | Classification | Organization Behavior |
|----------|--------------|----------------|----------------------|
| **Your repo, no fork** | `origin` → your URL | origin / personal / developing | Goes to personal or work based on org |
| **Your fork of someone else's** | `origin` → your URL, `upstream` → their URL | fork / personal or community / contributing | Goes to contrib or personal based on activity |
| **Direct clone, no fork** | `origin` → their URL | clone / third-party / reference | Goes to reference unless you have local commits |
| **Clone you've started committing to** | `origin` → their URL, local branches with your commits | clone / third-party / developing | kissa warns: "you're committing to a clone, not a fork. Want to fork it?" |
| **Work repo** | `origin` → org URL matching work_orgs | origin or clone / work:label / developing | Goes to work/{label}/ |
| **Bare mirror** | `origin` → any, `--mirror` config | mirror / any / reference | Goes to reference or mirrors/ |
| **Local only** | No remotes | origin / local / experiment | Goes to experiments/ unless .kissa says otherwise |

### Classification in the Graph

Classifications are node properties, queryable through the same structured filters as everything else:

```bash
# all your work repos for initech that you're actively developing
kissa list --ownership work:initech --intention developing

# forks where you have unpushed work
kissa list --category fork --unpushed

# third-party repos you cloned but never looked at
kissa list --ownership third-party --freshness ancient

# experiments that grew up (lots of commits, maybe should be reclassified)
kissa list --intention experiment --min-branches 3

# all repos in the initech project cluster
kissa list --project initech-platform

# repos that are dependencies of something else
kissa deps --reverse --all                    # show all repos that are depended upon
```

### How Organization Plans Use Classifications

When you run `kissa organize`, here's what happens:

1. **Classify**: Every repo gets category + ownership + intention (inferred or from `.kissa`).
2. **Match**: Each repo is matched against the active pattern's rules.
3. **Compare**: Current path vs proposed path. If they match, no action.
4. **Conflict**: If two repos would land in the same path, flag for elicitation.
5. **Plan**: Generate the move/tag/archive transaction.

The agent can also use classification to have a *conversation* about organization:

```
[organize] analyzing 57 repos

ownership breakdown:
  work:initech     23 repos (12 developing, 8 reference, 3 archived)
  work:initrode    7 repos (4 developing, 3 reference)
  work:vandelay    3 repos (2 developing, 1 infrastructure)
  personal      14 repos (6 developing, 4 experiment, 2 dotfiles, 2 archived)
  third-party    6 repos (all reference)
  community      4 repos (3 contributing, 1 reference)

currently 31 repos are not where the "hybrid" pattern would put them

? ask user: use "hybrid" pattern, or would you prefer "platform" or "role"?
? ask user: base path ~/Projects ok, or somewhere else?
→ next: apply pattern after preferences confirmed
```

---

## Difficulty Levels (Permission Model)

Difficulty levels control what operations kissa will perform against a repo. This is a safety model, not an auth model.

### Standard Levels

| Level | Allowed Operations | Use Case |
|-------|-------------------|----------|
| **readonly** | status, log, diff, branch list, remote list | Safe browsing. Default for MCP agents. |
| **fetch** | + fetch, pull, checkout existing branches | Updating local state. No new content created. |
| **commit** | + add, commit, push, create branch, merge (ff-only) | Normal development. The sensible default for CLI. |
| **force** | + rebase, force push, reset, delete branches | Power user operations. Requires explicit opt-in. |
| **unsafe** | + clean -fdx, repo deletion, destructive rewrites | Scorched earth. Per-repo opt-in only. Confirmation required. |

### Alternate Mode: `--cat-mode`

When invoked with `kissa --cat-mode` or configured in settings, difficulty level names are remapped:

| Standard | Cat Mode |
|----------|----------|
| readonly | 😴 napping |
| fetch | 🐱 purring |
| commit | 🐾 hunting |
| force | 😼 zoomies |
| unsafe | 🙀 knocking-things-off-the-counter |

Behavior is identical. This is purely cosmetic for people who want joy in their terminal.

### Always Blocked

Regardless of difficulty, kissa should never:

- Delete a repo that has unpushed commits without explicit multi-step confirmation
- Force push to a branch named `main`, `master`, or `production` without explicit confirmation
- Run `clean -fdx` on a repo with untracked files that look important (heuristic: non-generated, non-build-artifact files)
- Operate on repos outside of configured scan roots

### Configuration

```toml
# ~/.config/kissa/config.toml

[defaults]
difficulty = "commit"

[defaults.mcp]
difficulty = "readonly"

[overrides]
# Glob patterns for per-repo or per-directory overrides
"/home/me/code/work/production-*" = "readonly"
"/home/me/experiments/*" = "force"
```

---

## CLI Commands

### Discovery & Indexing

```
kissa scan                         # Quick verify: stat known repos, refresh changed ones
kissa scan --full [--roots <p>]    # Full scan: walk filesystem, find new repos
kissa scan --watch                 # Watch mode: inotify daemon for real-time detection
kissa list                         # List all catalogued repos
kissa list --dirty                 # Filter: repos with uncommitted changes
kissa list --stale [--days 90]     # Filter: repos not committed to in N days
kissa list --unpushed              # Filter: repos with commits not pushed
kissa list --orphan                # Filter: repos with no remote
kissa list --duplicates            # Filter: same remote, multiple paths
kissa list --lost                  # Filter: repos whose paths no longer exist
kissa list --format json           # Output as JSON (for piping / scripting)
kissa forget <repo>                # Remove a lost repo entry from the index permanently
```

### Inspection

```
kissa status <path|name>           # Detailed status of a single repo
kissa graph                        # Show repo topology / relationships
kissa graph --project <name>       # Filter graph to a project cluster
kissa info <path|name>             # Full vitals dump
kissa freshness                    # Freshness overview across all repos
kissa related <path|name>          # Show repos connected by any relationship
kissa deps <path|name>             # Show dependency graph for a repo
```

**CLI filter examples:**

```bash
# find all dirty work repos
kissa list --dirty --org initech

# what depends on shared-lib?
kissa deps shared-lib

# orphans with no remote
kissa list --orphan

# ancient repos as JSON for scripting
kissa list --freshness ancient --format json

# stale repos with unpushed commits (the danger zone)
kissa list --stale --unpushed

# everything in ~/Downloads that looks like work
kissa list --path-prefix ~/Downloads --has-remote
```

### Organization (The Killer Feature)

```
kissa organize --plan              # AI generates a reorganization plan
kissa organize --apply <plan>      # Execute a previously generated plan
kissa organize --dry-run <plan>    # Show what would happen
kissa move <repo> <destination>    # Move a single repo (updates index, warns about deps)
kissa tag <repo> <tags...>         # Add tags to a repo
kissa init-dotkissa <repo>        # Generate a .kissa file from inferred data
```

### Git Pass-through: `exec` (The Only Shell-Out)

```
kissa exec <repo|glob> -- <git command>   # Run a git command against matched repos
kissa exec --all -- fetch --prune         # Run across all repos
kissa exec --dirty -- stash               # Run against all dirty repos
```

**`exec` is the only place kissa invokes system `git`.** Everything else uses `libgit2`. This is an intentional boundary:

- `exec` runs real `git` via `execvp`-style argument arrays (never `sh -c`).
- `exec` **will** trigger git hooks. This is expected — you're explicitly asking for a git command.
- `exec` filters the command against the repo's difficulty level before executing. If the command would exceed the permission level, kissa refuses and explains why.
- `exec` requires system `git` to be installed. All other kissa commands do not.

### Utility

```
kissa config                       # Show current configuration
kissa config --edit                # Open config in $EDITOR
kissa export                       # Export full index as JSON
kissa doctor                       # Check for common issues (nested repos, missing remotes, etc.)
```

---

## MCP Server Interface

### Invocation

```
kissa --mcp                        # Start MCP server over stdio
kissa --mcp --transport tcp        # MCP server over TCP (v2)
```

Designed for use with Claude Code, Claude Desktop, or any MCP-compatible client.

### Design Philosophy: Text Over Structure

MCP tool responses return **terse, human-readable text with semantic hints** — not structured JSON. LLMs read text; they don't parse JSON efficiently. Every token spent navigating structure is a token not spent reasoning.

Tool responses follow this pattern:

```
[state_tag] concise natural language summary
key details, one per line, minimal decoration
only what the agent needs to reason about the next step

→ next: suggested_tool_1 | suggested_tool_2
```

The `[state_tag]` is a lightweight state machine signal — it tells the agent where it is in a workflow. The `→ next:` line suggests logical follow-up tools without enforcing a flow. The agent can always ignore hints and do something else.

**Example: scan response**

```
[scan_complete] 57 repos across 3 roots (~, ~/code, /opt/projects)
18 active · 12 recent · 9 stale · 8 dormant · 10 ancient
14 dirty · 5 unpushed · 2 duplicates · 6 orphans (no remote)

→ next: list_repos dirty | freshness | doctor | organize
```

**Example: list_repos dirty**

```
[listing] 14 dirty repos

initech-api         ~/code/work/initech/api          3 changed  main ↑2
personal-site    ~/code/personal/site          1 changed  develop
rusty-exp        ~/experiments/rusty           12 changed feat/thing ↑5 ↓3
...9 more

→ next: repo_status <name> | exec <glob> -- stash | organize
```

**Example: repo_status**

```
[status] initech-api — ~/code/work/initech/api
branch: main ↑2 (ahead of origin/main)
dirty: 3 files modified, 0 staged, 1 untracked
last commit: 2h ago "fix: auth middleware race condition"
remotes: origin → github.com/initech/api.git
branches: 4 local (2 merged/stale)
languages: rust (92%) toml (8%)
freshness: active
difficulty: commit

→ next: exec initech-api -- diff | exec initech-api -- push | list_repos
```

**Example: difficulty violation**

```
[blocked] cannot force-push initech-api — difficulty is "commit", needs "force"
the branch main is also in protected_branches

→ next: repo_status initech-api | get_config
? ask user: upgrade difficulty for this repo?
```

The `? ask user:` line is a hint to the agent that this is a good moment to use elicitation.

### State Tags Reference

| Tag | Meaning |
|-----|---------|
| `[scan_complete]` | Scan finished, index updated |
| `[listing]` | Filtered repo list follows |
| `[status]` | Single repo detail |
| `[graph]` | Relationship data follows |
| `[plan_ready]` | Organization plan generated, awaiting review |
| `[plan_applied]` | Plan executed successfully |
| `[executed]` | Git command ran successfully |
| `[blocked]` | Operation denied by difficulty or safety rule |
| `[warning]` | Something looks wrong, agent should investigate |
| `[error]` | Something failed |

### Tools

| Tool | Description | Inputs |
|------|-------------|--------|
| `scan` | Trigger a filesystem scan | `roots?: string[]` |
| `list_repos` | Filter and list repos by properties and relationships | `filters: object` (see below) |
| `related` | Show repos connected to a given repo by any relationship | `name_or_path: string` |
| `deps` | Show dependency graph for a repo | `name_or_path: string` |
| `repo_status` | Get detailed status for a repo | `name_or_path: string` |
| `freshness` | Freshness overview across all repos | — |
| `search` | Fuzzy search by name/path/tag | `query: string` |
| `doctor` | Run diagnostics, find problems | — |
| `organize` | Generate a reorganization plan | `hint?: string` (optional user intent) |
| `apply_plan` | Execute a reviewed plan | `plan_id: string` |
| `exec` | Run filtered git command against repos | `target: string, command: string` |
| `tag` | Set tags on a repo | `name_or_path: string, tags: string[]` |
| `get_config` | Read current config | — |
| `run` | Execute a batch of **read-only** commands in one call | `commands: string[]` |

### Graph Data Model (openCypher-Inspired)

kissa thinks in graphs internally — repos are nodes, relationships are edges. This section describes the **data model**, not a query language. The interface to this model is structured filters (CLI flags, MCP tool parameters) and dedicated relationship commands (`related`, `deps`), not a query parser.

The graph vocabulary is borrowed from openCypher because it's a clear way to think about repo topology. If a future version needs a richer query interface (e.g., backed by Apache AGE + PostgreSQL), the data model is already graph-native and the migration path is clean.

**Nodes:**

```
(:Repo)     — a git repository
```

**Node properties** are the repo vitals: `name`, `path`, `dirty`, `staged`, `untracked`, `freshness`, `org`, `branch`, `ahead`, `behind`, `last_commit`, `languages`, `tags`, `project`, `role`, `branches_count`, `has_remote`, `category`, `ownership`, `intention`

**Edge types:**

```
[:SUBMODULE]    — git submodule relationship
[:NESTED]       — physically inside another repo, not a submodule
[:SIBLING]      — same remote org/owner
[:DEPENDS_ON]   — local dependency reference
[:FORK_OF]      — different fork of same upstream
[:DUPLICATE]    — same remote, different local path
```

**How the graph is queried today:**

The CLI and MCP expose the graph through composable filters and relationship-aware commands, not raw graph queries:

```bash
# Node property filters (combinable, AND semantics)
kissa list --dirty --org initech              # dirty repos in initech
kissa list --freshness stale --unpushed       # stale repos with unpushed commits
kissa list --orphan --freshness ancient       # ancient repos with no remote
kissa list --path-prefix ~/Downloads --has-remote  # strays in Downloads

# Relationship traversal via dedicated commands
kissa deps shared-lib                         # what depends on shared-lib?
kissa related initech-api                     # all connected repos (any edge type)
kissa list --duplicates                       # same remote, multiple paths

# Combine with output modes for scripting
kissa list --org initech --paths              # just paths, one per line
kissa list --dirty --format json              # full vitals as JSON
```

**MCP `list_repos` tool filters:**

| Filter | Type | Description |
|--------|------|-------------|
| `dirty` | bool | Has uncommitted changes |
| `unpushed` | bool | Ahead of remote tracking branch |
| `orphan` | bool | No remote configured |
| `org` | string | Remote org/owner matches |
| `freshness` | string | Freshness tier: active, recent, stale, dormant, ancient |
| `ownership` | string | Ownership classification (personal, work:label, community, third-party, local) |
| `intention` | string | Intention classification (developing, contributing, reference, etc.) |
| `category` | string | Category classification (origin, clone, fork, mirror) |
| `tags` | string[] | Has all specified tags |
| `path_prefix` | string | Path starts with this prefix |
| `has_remote` | bool | Has at least one remote |

Filters compose with AND semantics. An agent calling `list_repos` with `{dirty: true, org: "initech"}` gets the same result as `kissa list --dirty --org initech`.

**Response format** follows the terse text pattern:

```
[listing] 5 dirty repos in org:initech

initech-api        ~/code/work/initech/api        3 changed  main ↑2
initech-frontend   ~/code/work/initech/frontend   1 changed  develop
initech-deploy     ~/code/work/initech/infra      2 changed  main
initech-docs       ~/code/work/initech/docs       1 changed  main
initech-proto      ~/code/work/initech/proto      4 changed  feat/v2 ↑1

→ next: repo_status <name> | exec <glob> -- <cmd>
```

**For relationship commands, edges are shown inline:**

```
[deps] 3 repos depend on initech-shared-lib

initech-api        ←[depends_on]  (Cargo.toml path dep)
initech-frontend   ←[depends_on]  (Cargo.toml path dep)
initech-cli        ←[depends_on]  (Cargo.toml path dep)

→ next: repo_status initech-shared-lib | related initech-shared-lib
```

**Why graph-shaped thinking matters even without a query language:**

1. The SQLite schema models nodes and edges explicitly — this is the foundation for relationship-aware operations like `organize`, `deps`, and `related`.
2. Structured filters cover 90%+ of what agents and humans actually ask for.
3. Dedicated relationship commands (`deps`, `related`) handle the topology queries that flat filters can't express.
4. The data model is ready for a real graph backend (Apache AGE, etc.) if the complexity ever warrants it — the migration is schema-level, not interface-level.

### Move Operations

A move isn't just `mv` — it's a multi-step operation handled atomically by the `move` tool:

1. Verify the source repo exists and is clean (or force)
2. Verify the destination doesn't collide
3. Move the directory
4. Update the kissa index
5. Verify the repo still works at the new location
6. Report any repos that had dependency references to the old path

**Single move via MCP:**

```
[moved] initech-api: ~/Downloads/api-thing → ~/code/work/initech/api
index updated · repo verified at new location
⚠ 2 repos had path dependencies on old location:
  initech-frontend (package.json) · initech-cli (Cargo.toml)

→ next: repo_status initech-api | deps initech-api
? ask user: update path references in initech-frontend and initech-cli?
```

**Batch moves go through the plan/apply cycle**, not the `run` tool. The `organize` command generates a named plan with all proposed moves, the user reviews it, and `apply_plan` executes it as an atomic transaction with rollback on failure:

### Resources

Resources give the agent **ambient context** — it reads these before the conversation starts, so it already understands the landscape without needing discovery tool calls.

| Resource URI | Description | When it matters |
|-------------|-------------|-----------------|
| `kissa://summary` | High-level stats: total repos, dirty count, stale count, freshness distribution, top orgs | Always. Agent reads this first to understand the landscape. |
| `kissa://config` | Current difficulty defaults, scan roots, protected branches | When agent needs to understand what it can/can't do. |
| `kissa://problems` | Output of `doctor` — known issues like duplicates, nested repos, orphans | On first load. Lets agent proactively mention issues. |

Resources are terse text, same format as tool responses:

```
[summary] 57 repos catalogued
roots: ~ ~/code /opt/projects
freshness: 18 active · 9 recent · 7 stale · 6 dormant · 7 ancient
health: 14 dirty · 5 unpushed · 2 duplicates · 6 orphans
orgs: initech (23) · initrode (7) · vandelay (3) · personal (14) · no-remote (6) · forks (4)
last scan: 12 min ago
```

The agent reads this and immediately knows: "you've got 57 repos across three orgs, 14 are dirty, and there are some orphans we should talk about."

### Elicitation

Elicitation is how kissa (via the agent) asks the user for decisions that can't be inferred. This is critical for the organize workflow — the agent shouldn't guess your preferred directory structure.

Elicitation points are signaled in tool responses with `? ask user:` hints:

```
[plan_ready] proposed reorganization for 33 work repos across 3 orgs

move 18 initech repos → ~/code/work/initech/{role}/
move 5 initrode repos → ~/code/work/initrode/{role}/
move 2 vandelay repos → ~/code/work/vandelay/{role}/
archive 4 repos → ~/code/archive/ (dormant 12+ months)
tag 2 repos as personal (forked from initech but personal remote)

? ask user: preferred base path for work repos (currently ~/code/work/)
? ask user: keep initrode separate or merge under work/consulting/?
? ask user: archive dormant repos or leave in place
? ask user: review full plan before apply
```

The agent uses these hints to ask targeted questions before proceeding. The human stays in control of every structural decision.

Key elicitation moments:
- **First scan**: "I found N repos across 3 orgs. Where do you want your organized code to live?"
- **Organization**: "You've got repos from initech, initrode, and vandelay. Separate dirs per org? Group by project? By language?"
- **Difficulty escalation**: "This operation needs force level on this repo. Allow it?"
- **Ambiguous classification**: "This repo has an initech remote but your personal email in commits. Work or personal?"

### Transaction Plans

The `organize` and `apply_plan` tools implement a two-phase pattern:

1. Agent calls `organize`. kissa generates a plan and returns it as readable text with a `plan_id`.
2. The agent presents the plan to the user conversationally, using elicitation to clarify ambiguities.
3. On approval, the agent calls `apply_plan` with the `plan_id`. kissa executes it, updating its index as it goes.

Plans are stored internally. The agent never needs to hold or replay the plan data — it just references the ID.

```
[plan_ready] plan abc123 — "organize work repos by org and role"
18 moves · 4 archives · 5 tags

  move  initech-api        → ~/code/work/initech/api         (remote matches org)
  move  initech-frontend   → ~/code/work/initech/frontend    (remote matches org)
  move  initech-deploy     → ~/code/work/initech/infra       (terraform detected)
  move  initrode-migrator  → ~/code/work/initrode/migration  (remote matches org)
  move  vandelay-exporter  → ~/code/work/vandelay/exporter   (remote matches org)
  ...13 more moves
  archive old-experiment → ~/code/archive/              (dormant 14 months)
  ...3 more archives
  tag   rusty-thing       personal, rust                (no remote, rust project)
  ...4 more tags

? ask user: approve / modify / reject
→ next: apply_plan abc123 | organize (regenerate)
```

### Coding Agent Escape Hatch

The MCP server should include a hint in its initial resource or server description:

```
if you have shell access (e.g. Claude Code, Cursor, etc.), you can invoke
kissa directly on the command line for complex operations. the CLI supports
--format json for structured output. MCP tools are best for conversational
exploration; the CLI is best for scripting and advanced git operations.

examples:
  kissa list --dirty --format json
  kissa exec initech-api -- log --oneline -10
  kissa organize --plan --format json > plan.json
```

This prevents the pathological case where a coding agent with full shell access makes 15 MCP tool calls to do something it could do in one `kissa exec --all -- fetch` command. The MCP interface is for *conversation*. The CLI is for *doing*. A smart agent uses both.

### Batch Execution: The `run` Tool

The `run` tool accepts an array of **read-only** kissa commands and executes them in sequence within a single tool call. This dramatically reduces round-trips for reconnaissance and triage operations.

**Read-only constraint:** `run` only accepts commands that don't mutate state — `list_repos`, `repo_status`, `freshness`, `search`, `related`, `deps`, `doctor`, `get_config`. Write operations (`exec`, `tag`, `organize`, `apply_plan`, `move`) are rejected with an explanation. This prevents partial-failure spaghetti where command 3 of 8 fails and leaves the index in a half-mutated state. Writes go through their own tools with explicit review and confirmation.

**Inputs:**

| Field | Type | Description |
|-------|------|-------------|
| `commands` | `string[]` | Array of read-only kissa commands (without the `kissa` prefix) |

**Example: agent wants to triage dirty repos**

Instead of 4 separate tool calls:

```json
{
  "commands": [
    "list_repos dirty",
    "repo_status initech-api",
    "repo_status personal-site",
    "freshness"
  ]
}
```

**Response:**

```
[batch] 4 commands · 4 ok

--- list_repos dirty ---
[listing] 14 dirty repos
initech-api         ~/code/work/initech/api          3 changed  main ↑2
personal-site    ~/code/personal/site          1 changed  develop
...10 more

--- repo_status initech-api ---
[status] initech-api — ~/code/work/initech/api
branch: main ↑2 (ahead of origin/main)
dirty: 3 files modified, 0 staged, 1 untracked
last commit: 2h ago "fix: auth middleware race condition"
...

--- repo_status personal-site ---
[status] personal-site — ~/code/personal/site
branch: develop
dirty: 1 file modified
last commit: 3d ago "update about page"
...

--- freshness ---
[freshness] 57 repos
active: 18 · recent: 9 · stale: 7 · dormant: 6 · ancient: 7

→ next: repo_status <name> | organize | exec <glob> -- <cmd>
```

**Example: write command rejected**

```json
{
  "commands": [
    "repo_status initech-api",
    "exec initech-api -- push"
  ]
}
```

```
[error] run only accepts read-only commands
rejected: exec initech-api -- push (use the exec tool directly)
```

The `run` tool is for building a picture, not for taking action. An agent gathers context in one batch, reasons about it, then takes individual write actions through the appropriate tools with user visibility at each step.

While MCP speaks terse text, the CLI supports JSON I/O for scripting:

```bash
# CLI can still output JSON for scripts and piping
kissa list --dirty --format json > dirty-repos.json

# Plans can be exported as JSON for manual editing
kissa organize --plan --format json > plan.json
kissa organize --apply plan.json

# Accept JSON commands on stdin for automation
echo '{"command": "list", "filter": "dirty"}' | kissa --json
```

This keeps the scripting story clean without polluting the MCP interface with structure the LLM doesn't need.

---

## State Management & Scanning

### The Index

kissa's central state is a SQLite database at `~/.local/share/kissa/index.db`. This is the single source of truth for everything kissa knows. All queries, graph traversals, and tool responses read from the index. All mutations (scans, moves, tags) write to it.

SQLite because:
- Survives restarts. No cold-start penalty.
- Graph queries compile to SQL joins efficiently.
- Single file. Easy to back up, move between machines, nuke and rebuild.
- Handles hundreds of repos without breaking a sweat.
- WAL mode lets reads and writes coexist (MCP server serving queries while a scan updates).

The index stores: repo vitals (all node properties), edges (relationships), scan metadata (when each repo was last verified), plan history, and user tags/overrides.

### Scanning Tiers

Not every operation walks the filesystem. kissa uses four tiers of freshness, escalating in cost:

#### Tier 0: Index Only (free)
Read straight from SQLite. No filesystem access. This is what most `query`, `list`, `freshness` commands do. The data might be minutes or hours old, and that's fine. The response includes a `last verified: 12 min ago` line so the user/agent knows.

#### Tier 1: Quick Verify (cheap)
Stat the `.git/HEAD` file for every known repo. If mtime hasn't changed, the repo hasn't changed. If the path is gone, mark `[lost]`. If mtime changed, do a targeted refresh of that repo's vitals via `git2` (branch, dirty, ahead/behind).

Cost: one `stat()` per known repo. For 200 repos, this takes milliseconds.

Triggers: `kissa scan` (no flags), or automatically if the index is older than a configurable threshold (default: 5 minutes for MCP, 1 hour for CLI).

#### Tier 2: Full Scan (expensive)
Walk configured root directories, find every `.git` directory. Compare against the index. New repos get added, missing repos get marked `[lost]`, existing repos get refreshed.

Cost: depends on filesystem size. Hundreds of milliseconds to several seconds for a typical home directory with proper exclusions. Can be slow on network mounts.

Triggers: `kissa scan --full`, first run, or `kissa doctor` (which also does a full scan to find problems).

#### Tier 3: Watch (continuous, low overhead)
Use `inotify` (Linux) to watch configured root directories for `.git` directory creation, deletion, or rename. This catches:
- `git clone <url>` → new `.git` appears → index updated
- `git init` → new `.git` appears → index updated
- `rm -rf some-repo/` → `.git` disappears → marked `[lost]`
- `mv some-repo/ other-place/` → disappearance + appearance → kissa can correlate

Watch mode runs when kissa is operating as a daemon (`kissa --watch` or via systemd service). It doesn't run for one-shot CLI commands.

**inotify budget:** Linux has a per-user inotify watch limit (default 8192, often raised to 65536). kissa watches *directories*, not individual files, and only at configured roots. Typical usage is tens of watches, not thousands. kissa reports its watch count in `kissa doctor` and warns if it's approaching the limit.

#### Tier 4: Opportunistic (free, per-repo)
Any command that touches a specific repo (`kissa status foo`, `kissa exec foo -- diff`) refreshes that repo's index entry as a side effect. You asked about it, so you get current data, and the index gets updated for free.

### Lost Repo Handling

When a known repo's `.git` path no longer exists:

1. Mark as `[lost]` in the index. Do NOT delete it.
2. Preserve all metadata — tags, project, relationships, last known state.
3. If a new repo appears with a matching remote URL, suggest: "initech-api was at ~/Downloads/api-thing (lost) and a match appeared at ~/code/initech/api. Same repo?"
4. User confirms → index updates the path, clears `[lost]`. User denies → both entries exist.
5. `kissa list --lost` shows all lost repos. `kissa forget <repo>` removes a lost entry permanently.

This matters because people move repos with `mv` and shouldn't lose their kissa metadata when they do.

### Stray Repo Detection

New repos appearing outside of a `kissa move`:

- **In watched roots:** inotify catches it immediately (Tier 3).
- **Outside watched roots:** Only found on `kissa scan --full`.
- `kissa doctor` runs a full scan and reports: "Found 3 repos outside configured roots: ~/Desktop/random-thing, /tmp/quick-test, ~/Documents/oops"
- The agent can use this proactively: "I see some repos in unusual places. Want me to organize them?"

---

## Filesystem Boundaries

### The Problem

Scanning `/` would be insane. Scanning `/dev`, `/proc`, `/sys` is pointless. Scanning a FUSE mount to a remote server could be slow or trigger auth prompts. Scanning `/home/me/.local/share/flatpak` will find vendor repos you don't care about. You need explicit control.

### Scan Roots

kissa only scans explicitly configured roots. It never walks outside of them.

```toml
[scan]
# Only these directories are scanned. Nothing else. Ever.
roots = [
  "~/code",
  "~/projects",
  "~/Documents",
  "~"               # home dir — only if you want the full chaos picture
]
```

If `roots` is unset, kissa defaults to `$HOME` on first run and asks if you want to narrow it.

### Exclusions

Directories to skip during traversal. Applied as prefix matches against absolute paths.

```toml
[scan]
exclude = [
  # Build artifacts / package managers (never contain user repos)
  "node_modules",
  ".cargo/registry",
  ".rustup",
  "target/",
  ".gradle",
  ".m2",
  "__pycache__",
  ".venv",
  "venv",

  # System / runtime (pointless to scan)
  ".cache",
  ".local/share/Trash",
  ".local/share/flatpak",
  ".local/share/Steam",
  "snap/",
  ".npm",
  ".nvm/versions",

  # Dangerous / slow (network mounts, virtual filesystems)
  # These are explicitly called out so users think about them
]
```

kissa ships with a sensible default exclusion list. Users extend it, they don't have to build it from scratch.

### Filesystem Boundary Rules

Beyond simple path exclusions, kissa respects filesystem boundaries:

```toml
[scan.boundaries]
# Don't cross filesystem mount boundaries during scan.
# Prevents accidentally walking into network mounts, USB drives, etc.
cross_mounts = false        # default: false

# Explicitly permitted mount points (overrides cross_mounts = false)
# Use this for intentional remote mounts you DO want scanned.
allow_mounts = [
  "/mnt/nas/code",          # NAS with code on it — scan it
]

# Explicitly blocked mount points (overrides everything)
# Use this for mounts that should NEVER be scanned even if under a root.
block_mounts = [
  "/mnt/bastion",           # SSH mount to bastion — absolutely not
  "/mnt/backup",            # Backup drive — old snapshots, not live repos
]

# Timeout for stat operations (catches hung network mounts)
stat_timeout_ms = 500       # default: 500ms. If a stat takes longer, skip.
```

**`cross_mounts = false`** is the key default. When kissa is walking `~/code` and encounters a mount point (detected via `stat()` device ID change), it stops. This prevents the nightmare scenario of accidentally walking an NFS mount to a server with a million files.

If you intentionally have code on a NAS or a mounted volume, `allow_mounts` lets you explicitly opt in.

**`stat_timeout_ms`** is a safety valve. If a single `stat()` call takes more than 500ms, something is wrong (hung NFS, disconnected SSHFS). kissa skips that path and logs a warning: `[warning] stat timeout on /mnt/bastion/some/path — skipping (mount may be unavailable)`

### How Boundary Detection Works

During any filesystem walk (Tier 2 full scan):

1. Stat the root directory, record its device ID.
2. For each child directory:
   a. Stat it.
   b. If stat times out → skip, warn.
   c. If device ID differs from parent → mount boundary detected.
   d. Check against `allow_mounts` → if listed, continue. If not, and `cross_mounts` is false → skip.
   e. Check against `block_mounts` → if listed, always skip.
   f. Check against `exclude` → if matched, skip.
   g. If the directory contains `.git` → found a repo.
   h. Otherwise → recurse (up to `max_depth`).

3. Report scan summary:
```
[scan_complete] scanned 3 roots in 1.2s
found: 57 repos (2 new, 1 lost)
skipped: 4 mount boundaries, 842 excluded dirs
warnings: 1 stat timeout (/mnt/bastion)

→ next: list_repos | doctor | list_repos --lost
```

### First Run Experience

On first run with no config:

```
$ kissa scan

no config found — running first-time setup

scan root? [~/] › ~

scanning home directory...
[████████████████████░░░░░] 73%
skipping: .cache, node_modules, .cargo/registry, .local/share/Trash
detected mount boundary: /mnt/nas (different filesystem)
detected mount boundary: /mnt/bastion (different filesystem)

found 57 repos in 1.8s
skipped 2 mount boundaries (add to allow_mounts in config to include)

config written to ~/.config/kissa/config.toml
index written to ~/.local/share/kissa/index.db

---

## Configuration

### Location

Follows XDG Base Directory spec:

```
~/.config/kissa/config.toml      # Main configuration
~/.local/share/kissa/index.db    # Repo index (SQLite, WAL mode)
~/.cache/kissa/                   # Scan cache, temp data
```

### Full Config Reference

```toml
[scan]
# Root directories to scan for .git folders
roots = ["~", "~/code", "/opt/projects"]

# Directories to skip during scanning (prefix match on dir names)
exclude = [
  "node_modules",
  ".cargo/registry",
  ".rustup",
  "target/",
  ".cache",
  ".local/share/Trash",
  ".local/share/flatpak",
  ".local/share/Steam",
  "snap/",
  ".npm",
  ".nvm/versions",
  "__pycache__",
  ".venv",
]

# Maximum depth to walk
max_depth = 10

# Auto-verify threshold: how stale the index can be before auto-refreshing
auto_verify_seconds = 300       # 5 min for MCP, overridden below for CLI

[scan.boundaries]
# Don't cross filesystem mount boundaries
cross_mounts = false

# Explicitly allowed mounts (overrides cross_mounts = false)
allow_mounts = []

# Explicitly blocked mounts (overrides everything)
block_mounts = []

# Timeout per stat call (catches hung network mounts)
stat_timeout_ms = 500

[identity]
# Your usernames across git platforms
usernames = ["myuser", "myuser-work"]

# Organizations that are "work"
work_orgs = [
  { name = "initech", platform = "github.com", label = "initech" },
  { name = "initech", platform = "gitlab.com", label = "initech" },
]

# Organizations you contribute to (open source, community)
community_orgs = ["rust-lang", "tokio-rs"]

[organization]
# Built-in pattern: "platform", "role", "project", "hybrid"
pattern = "hybrid"
base_path = "~/Projects"

# Custom rules override the pattern (evaluated top to bottom, first match wins)
# [[organization.rules]]
# match = { ownership = "work:initech" }
# path = "work/initech/{repo_name}"

[defaults]
# Default difficulty for CLI usage
difficulty = "commit"

[defaults.mcp]
# Default difficulty for MCP connections
difficulty = "readonly"

[display]
# Terminal color theme
# "auto" respects NO_COLOR and TERM
color = "auto"

# Use nerd font icons if available
nerd_fonts = false

# Cat mode difficulty names
cat_mode = false

[overrides]
# Per-path difficulty overrides (glob patterns supported)
"/home/me/code/work/production-*" = "readonly"
"/home/me/experiments/*" = "force"

[safety]
# Branches that are always protected from force push / deletion
protected_branches = ["main", "master", "production", "release/*"]

# Require confirmation for destructive operations even at appropriate difficulty
always_confirm_destructive = true

# Maximum number of repos a single plan can affect (sanity check)
max_plan_size = 50

[mcp]
# Future: TCP transport settings
# transport = "stdio"
# bind = "127.0.0.1:9999"
# auth = "pam"
```

---

## Terminal Display

kissa should be visually clear without being noisy. Semantic colors carry meaning, not decoration.

### Color Semantics

| Color | Meaning |
|-------|---------|
| green | clean, active, safe |
| cyan | informational, recent |
| yellow | warning: stale, dirty, diverged |
| red | danger: dormant, unpushed to protected branch |
| dim/gray | ancient, archived, low priority |
| bold white | repo names, emphasis |

### Example Output

```
$ kissa list --dirty

 ● initech-api           ~/code/work/initech/api           3 files changed   main ↑2
 ● personal-site      ~/code/personal/site           1 file changed    develop
 ● experiments/rusty   ~/code/experiments/rusty       12 files changed  feat/thing ↑5 ↓3

3 dirty repos out of 57 catalogued
```

```
$ kissa freshness

 active (7d)    ██████████████████░░░░░░░░  18 repos
 recent (30d)   ████████░░░░░░░░░░░░░░░░░  9 repos
 stale (90d)    █████░░░░░░░░░░░░░░░░░░░░  7 repos
 dormant (1y)   ████░░░░░░░░░░░░░░░░░░░░░  6 repos
 ancient (1y+)  ███████░░░░░░░░░░░░░░░░░░  7 repos
```

---

## Distribution

### Build

```bash
cargo build --release
# Output: target/release/kissa
```

Single static binary. **No runtime dependency on `git` for core operations** — all scanning, status, log, diff, branch, and remote operations use `libgit2` linked statically via the `git2` crate. System `git` is only invoked by the `exec` pass-through command, and kissa will warn if `git` is not found when `exec` is used.

### AUR

PKGBUILD targeting:
- `kissa` — binary package from crates.io or release tarball
- `kissa-git` — build from latest git source

### Other

- crates.io publication
- GitHub releases with prebuilt binaries (Linux x86_64, aarch64)
- Homebrew formula (future)
- Nix flake (future)

---

## Versioning & Scope

### v0.1 — Foundation

- Filesystem scanning and index persistence
- CLI: scan, list (with filters), status, info, freshness
- Terminal display with semantic colors
- XDG configuration
- Difficulty levels with command filtering
- `kissa --mcp` over stdio with core read tools
- JSON round-tripping on CLI
- AUR package

### v0.2 — Organization

- .kissa file support
- Repo graph / relationship mapping
- `organize --plan` with transaction model
- MCP plan/execute tools
- `exec` pass-through with filtering

### v0.3 — Intelligence

- AI-assisted classification and organization proposals
- Language/framework detection
- Dependency-based relationship inference
- `doctor` diagnostics

### v1.0 — Full Product

- Stable CLI and MCP interfaces
- Watch mode for new repo detection
- systemd user service support
- TCP transport with authentication (PAM or token)
- Remote MCP connections
- Comprehensive test suite

---

## Security Posture

kissa is not a security scanner. It does not analyze repo contents for malware, secrets, or vulnerabilities. But it must not be a *vector* itself, and it should be a good pipe source for tools that are scanners.

### Threat: kissa as a target

A malicious repo on the filesystem could try to exploit kissa during scanning or inspection.

| Threat | Mitigation |
|--------|------------|
| **Git hooks** | kissa uses `libgit2` (via `git2` crate) for all read operations. libgit2 does **not** execute hooks. No scan, status check, or query will ever trigger a hook. The `exec` pass-through command runs real `git` and **will** fire hooks — this is expected and documented, since the user is explicitly requesting git commands. |
| **Malicious `.kissa` files** | Strict TOML parsing via `serde`. All path values validated: must be relative or resolve under `$HOME`. String fields capped at sane lengths (e.g., 256 chars for tags, 1024 for paths). Unknown keys ignored, not executed. |
| **Symlink `.git`** | Use `lstat()` during scanning, not `stat()`. If `.git` is a symlink, resolve it and verify the target is a real git directory on the same filesystem. Log a `[warning]` if the symlink points outside scan roots. |
| **Path injection** | Repo paths containing shell metacharacters (`$`, `` ` ``, `|`, `;`, newlines) are common in the wild (people name things badly). kissa never interpolates paths into shell commands. All `git2` calls use native APIs, not string concatenation. `exec` uses `execvp`-style argument arrays, never `sh -c`. |
| **Resource exhaustion** | A repo with millions of refs, enormous objects, or deeply nested submodules could slow kissa. Timeouts on all `git2` operations (configurable, default 5s). If a repo times out, mark it `[timeout]` in the index and move on. |

### Pipeline Integration

kissa should be a first-class citizen in unix pipelines. The primary use case: feed a list of repo paths to an external security scanner, linter, or analysis tool.

**Output modes for piping:**

```bash
# Default: human-readable, one repo per line
kissa list --dirty
# initech-api         ~/code/work/initech/api          3 changed  main ↑2

# Paths only: just the repo root paths, one per line
kissa list --dirty --paths
# /home/me/code/work/initech/api
# /home/me/code/personal/site

# Null-delimited: safe for paths with spaces/special chars
kissa list --dirty --paths -0
# /home/me/code/work/initech/api\0/home/me/code/personal/site\0

# JSON: structured, one object per line (jsonlines)
kissa list --dirty --json
# {"name":"initech-api","path":"/home/me/code/work/initech/api","dirty":true,...}

# JSON array: single array of all results
kissa list --dirty --json --array
# [{"name":"initech-api",...},{"name":"personal-site",...}]

# filter by org with paths output
kissa list --org initech --paths
```

**Example pipelines:**

```bash
# scan all dirty repos for secrets with gitleaks
kissa list --dirty --paths | xargs -I{} gitleaks detect --source {}

# check all repos for credential exposure
kissa list --paths -0 | xargs -0 -P4 -I{} trufflehog filesystem {}

# find large files across all repos
kissa list --paths | while read repo; do
  echo "=== $repo ==="
  git -C "$repo" rev-list --objects --all | \
    git -C "$repo" cat-file --batch-check | \
    sort -k3 -n -r | head -5
done

# feed repo list to a custom scanner via jq
kissa list --json | jq -r 'select(.org == "initech") | .path' | myscanner

# active repos with remotes → scanner
kissa list --has-remote --freshness active --json | \
  jq -r '.path' | xargs -P8 -I{} my-security-scan {}

# find repos that might have been compromised (foreign hooks)
kissa list --paths -0 | xargs -0 -I{} sh -c '
  if [ -d "{}/.git/hooks" ] && ls "{}/.git/hooks/" | grep -qv ".sample$"; then
    echo "ACTIVE HOOKS: {}"
  fi
'
```

The `--paths` and `-0` flags are the key enablers. They turn kissa from a pretty terminal tool into a repo-path emitter that plugs into any unix pipeline. The `--json` output gives richer data for tools like `jq` that can filter on any repo property.

---

## Design Principles

1. **Infer first, configure second.** kissa should be useful the moment you run `kissa scan` with zero configuration. The .kissa file and config.toml exist for when inference isn't enough.

2. **Plans before actions.** Any operation that changes the filesystem goes through the plan/review/execute cycle. No surprises.

3. **The CLI is for you. The MCP is for agents. The core is for both.** No logic lives in the interface layer. Both the CLI and MCP server are thin wrappers over the same core.

4. **Safety by default, power by choice.** The default difficulty is safe. Escalation is explicit. Some things always require confirmation.

5. **libgit2 first, system git only on demand.** All kissa-internal operations use `git2` (libgit2). This means no hooks fire, no aliases resolve, no dependency on system git version. The `exec` command is the explicit, documented boundary where system `git` is invoked — and it's the user's choice to cross it.

6. **A repo catalogue, not a git replacement.** kissa doesn't try to be a better git. It's the layer above git that sees your whole landscape. It launders commands through to git via `exec` when needed, with guardrails.

7. **Standalone and portable.** One binary. No daemon required (though it can run as one). Works on any Linux box, targets Arch first.

mod types;

pub use types::{FreshnessSummary, IndexSummary};
use types::RepoRow;

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use super::filter::RepoFilter;
use super::repo::{Ownership, Remote, Repo, RepoId};
use crate::error::Result;

const SCHEMA_VERSION: i32 = 2;

/// The persistent repo index backed by SQLite (ADR-103).
pub struct Index {
    conn: rusqlite::Connection,
}

impl Index {
    /// Open or create the index database at the given path. Enables WAL mode.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::error::KissaError::Config(format!(
                    "failed to create data directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        let conn = rusqlite::Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "wal")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let index = Self { conn };
        index.migrate()?;
        Ok(index)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let index = Self { conn };
        index.migrate()?;
        Ok(index)
    }

    /// Run schema migrations to latest version.
    pub fn migrate(&self) -> Result<()> {
        let current = self.schema_version();

        if current < 1 {
            self.conn.execute_batch(
                "
                CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER NOT NULL
                );

                CREATE TABLE IF NOT EXISTS repos (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    path TEXT NOT NULL UNIQUE,
                    state TEXT NOT NULL DEFAULT 'active',
                    default_branch TEXT,
                    current_branch TEXT,
                    branch_count INTEGER NOT NULL DEFAULT 0,
                    stale_branch_count INTEGER NOT NULL DEFAULT 0,
                    dirty INTEGER NOT NULL DEFAULT 0,
                    staged INTEGER NOT NULL DEFAULT 0,
                    untracked INTEGER NOT NULL DEFAULT 0,
                    ahead INTEGER NOT NULL DEFAULT 0,
                    behind INTEGER NOT NULL DEFAULT 0,
                    last_commit TEXT,
                    last_verified TEXT,
                    first_seen TEXT NOT NULL,
                    freshness TEXT NOT NULL DEFAULT 'ancient',
                    category TEXT,
                    ownership_type TEXT,
                    ownership_label TEXT,
                    intention TEXT,
                    project TEXT,
                    role TEXT
                );

                CREATE TABLE IF NOT EXISTS remotes (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
                    name TEXT NOT NULL,
                    url TEXT NOT NULL,
                    push_url TEXT
                );

                CREATE TABLE IF NOT EXISTS tags (
                    repo_id INTEGER NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
                    tag TEXT NOT NULL,
                    PRIMARY KEY (repo_id, tag)
                );

                CREATE TABLE IF NOT EXISTS scans (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    completed_at TEXT NOT NULL,
                    roots TEXT NOT NULL,
                    repo_count INTEGER NOT NULL
                );

                CREATE INDEX IF NOT EXISTS idx_repos_path ON repos(path);
                CREATE INDEX IF NOT EXISTS idx_repos_name ON repos(name);
                CREATE INDEX IF NOT EXISTS idx_repos_state ON repos(state);
                CREATE INDEX IF NOT EXISTS idx_remotes_repo_id ON remotes(repo_id);
                CREATE INDEX IF NOT EXISTS idx_tags_repo_id ON tags(repo_id);
                ",
            )?;

            self.conn.execute(
                "INSERT INTO schema_version (version) VALUES (?1)",
                [1],
            )?;
        }

        if current < 2 {
            self.conn.execute_batch(
                "ALTER TABLE repos ADD COLUMN managed_by TEXT;"
            )?;
            self.conn.execute(
                "UPDATE schema_version SET version = ?1",
                [SCHEMA_VERSION],
            )?;
        }

        Ok(())
    }

    fn schema_version(&self) -> i32 {
        self.conn
            .query_row(
                "SELECT version FROM schema_version LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
    }

    /// Insert or update a repo in the index.
    pub fn upsert_repo(&self, repo: &Repo) -> Result<RepoId> {
        let (ownership_type, ownership_label) = match &repo.ownership {
            Some(Ownership::Personal) => (Some("personal"), None),
            Some(Ownership::Work { label }) => (Some("work"), Some(label.as_str())),
            Some(Ownership::Community) => (Some("community"), None),
            Some(Ownership::ThirdParty) => (Some("thirdparty"), None),
            Some(Ownership::Local) => (Some("local"), None),
            None => (None, None),
        };

        let state_str = serde_plain::to_string(&repo.state).unwrap_or_else(|_| "active".into());
        let freshness_str =
            serde_plain::to_string(&repo.freshness).unwrap_or_else(|_| "ancient".into());
        let category_str = repo.category.as_ref().and_then(|c| serde_plain::to_string(c).ok());
        let intention_str = repo
            .intention
            .as_ref()
            .and_then(|i| serde_plain::to_string(i).ok());
        let last_commit_str = repo.last_commit.map(|dt| dt.to_rfc3339());
        let last_verified_str = repo.last_verified.map(|dt| dt.to_rfc3339());
        let first_seen_str = repo.first_seen.to_rfc3339();
        let path_str = repo.path.to_string_lossy();

        self.conn.execute(
            "INSERT INTO repos (
                name, path, state, default_branch, current_branch,
                branch_count, stale_branch_count, dirty, staged, untracked,
                ahead, behind, last_commit, last_verified, first_seen,
                freshness, category, ownership_type, ownership_label,
                intention, project, role, managed_by
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15,
                ?16, ?17, ?18, ?19,
                ?20, ?21, ?22, ?23
            )
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                state = excluded.state,
                default_branch = excluded.default_branch,
                current_branch = excluded.current_branch,
                branch_count = excluded.branch_count,
                stale_branch_count = excluded.stale_branch_count,
                dirty = excluded.dirty,
                staged = excluded.staged,
                untracked = excluded.untracked,
                ahead = excluded.ahead,
                behind = excluded.behind,
                last_commit = excluded.last_commit,
                last_verified = excluded.last_verified,
                freshness = excluded.freshness,
                category = excluded.category,
                ownership_type = excluded.ownership_type,
                ownership_label = excluded.ownership_label,
                intention = excluded.intention,
                project = excluded.project,
                role = excluded.role,
                managed_by = excluded.managed_by
            ",
            rusqlite::params![
                repo.name,
                path_str,
                state_str,
                repo.default_branch,
                repo.current_branch,
                repo.branch_count,
                repo.stale_branch_count,
                repo.dirty,
                repo.staged,
                repo.untracked,
                repo.ahead,
                repo.behind,
                last_commit_str,
                last_verified_str,
                first_seen_str,
                freshness_str,
                category_str,
                ownership_type,
                ownership_label,
                intention_str,
                repo.project,
                repo.role,
                repo.managed_by,
            ],
        )?;

        // Always query for the canonical id (UPSERT doesn't reliably set last_insert_rowid)
        let repo_id: i64 = self.conn.query_row(
            "SELECT id FROM repos WHERE path = ?1",
            [path_str.as_ref()],
            |row| row.get(0),
        )?;

        // Replace remotes
        self.conn
            .execute("DELETE FROM remotes WHERE repo_id = ?1", [repo_id])?;
        for remote in &repo.remotes {
            self.conn.execute(
                "INSERT INTO remotes (repo_id, name, url, push_url) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![repo_id, remote.name, remote.url, remote.push_url],
            )?;
        }

        // Replace tags
        self.conn
            .execute("DELETE FROM tags WHERE repo_id = ?1", [repo_id])?;
        for tag in &repo.tags {
            self.conn.execute(
                "INSERT INTO tags (repo_id, tag) VALUES (?1, ?2)",
                rusqlite::params![repo_id, tag],
            )?;
        }

        Ok(repo_id)
    }

    /// Get a repo by its absolute path.
    pub fn get_repo_by_path(&self, path: &Path) -> Result<Option<Repo>> {
        let path_str = path.to_string_lossy();
        let result = self.conn.query_row(
            "SELECT id FROM repos WHERE path = ?1",
            [path_str.as_ref()],
            |row| row.get::<_, i64>(0),
        );
        match result {
            Ok(id) => Ok(Some(self.load_repo(id)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Get a repo by name (exact match first, then prefix, then contains).
    pub fn get_repo_by_name(&self, name: &str) -> Result<Option<Repo>> {
        // Exact match
        let result = self.conn.query_row(
            "SELECT id FROM repos WHERE name = ?1 AND state != 'lost' LIMIT 1",
            [name],
            |row| row.get::<_, i64>(0),
        );
        if let Ok(id) = result {
            return Ok(Some(self.load_repo(id)?));
        }

        // Prefix match
        let like_prefix = format!("{}%", name);
        let result = self.conn.query_row(
            "SELECT id FROM repos WHERE name LIKE ?1 AND state != 'lost' LIMIT 1",
            [&like_prefix],
            |row| row.get::<_, i64>(0),
        );
        if let Ok(id) = result {
            return Ok(Some(self.load_repo(id)?));
        }

        // Contains match
        let like_contains = format!("%{}%", name);
        let result = self.conn.query_row(
            "SELECT id FROM repos WHERE name LIKE ?1 AND state != 'lost' LIMIT 1",
            [&like_contains],
            |row| row.get::<_, i64>(0),
        );
        match result {
            Ok(id) => Ok(Some(self.load_repo(id)?)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List repos matching the given filter.
    /// Uses SQL for basic column filters, then applies RepoFilter::matches() for complex ones.
    pub fn list_repos(&self, filter: &RepoFilter) -> Result<Vec<Repo>> {
        let mut where_clauses = vec!["1=1".to_string()];
        let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // Simple SQL-expressible filters
        if let Some(dirty) = filter.dirty {
            where_clauses.push(format!("dirty = ?{}", params.len() + 1));
            params.push(Box::new(dirty));
        }
        if let Some(ref state) = filter.state {
            let s = serde_plain::to_string(state).unwrap_or_else(|_| "active".into());
            where_clauses.push(format!("state = ?{}", params.len() + 1));
            params.push(Box::new(s));
        }
        if let Some(ref freshness) = filter.freshness {
            let s = serde_plain::to_string(freshness).unwrap_or_else(|_| "ancient".into());
            where_clauses.push(format!("freshness = ?{}", params.len() + 1));
            params.push(Box::new(s));
        }
        if let Some(ref prefix) = filter.path_prefix {
            where_clauses.push(format!("path LIKE ?{}", params.len() + 1));
            params.push(Box::new(format!("{}%", prefix)));
        }
        if let Some(ref name) = filter.name_contains {
            where_clauses.push(format!("name LIKE ?{}", params.len() + 1));
            params.push(Box::new(format!("%{}%", name)));
        }
        if let Some(ref mb) = filter.managed_by {
            where_clauses.push(format!("managed_by = ?{}", params.len() + 1));
            params.push(Box::new(mb.clone()));
        }
        if let Some(show) = filter.show_managed {
            if show {
                where_clauses.push("managed_by IS NOT NULL".to_string());
            } else {
                where_clauses.push("managed_by IS NULL".to_string());
            }
        }

        let sql = format!(
            "SELECT id FROM repos WHERE {}",
            where_clauses.join(" AND ")
        );

        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let ids: Vec<i64> = stmt
            .query_map(param_refs.as_slice(), |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut repos = Vec::new();
        for id in ids {
            let repo = self.load_repo(id)?;
            // Apply complex in-memory filters (org, ownership, tags, orphan, etc.)
            if filter.matches(&repo) {
                repos.push(repo);
            }
        }

        Ok(repos)
    }

    /// Get all repos (unfiltered).
    pub fn all_repos(&self) -> Result<Vec<Repo>> {
        self.list_repos(&RepoFilter::default())
    }

    /// Mark a repo as lost (path no longer exists).
    pub fn mark_lost(&self, id: RepoId) -> Result<()> {
        self.conn
            .execute("UPDATE repos SET state = 'lost' WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Remove a repo from the index permanently.
    pub fn forget_repo(&self, id: RepoId) -> Result<()> {
        self.conn.execute("DELETE FROM repos WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Get summary statistics for the entire index.
    pub fn summary(&self) -> Result<IndexSummary> {
        let total_repos: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))?;
        let dirty_count: usize = self
            .conn
            .query_row("SELECT COUNT(*) FROM repos WHERE dirty = 1", [], |row| {
                row.get(0)
            })?;
        let unpushed_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM repos WHERE ahead > 0",
            [],
            |row| row.get(0),
        )?;
        let orphan_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM repos WHERE id NOT IN (SELECT DISTINCT repo_id FROM remotes)",
            [],
            |row| row.get(0),
        )?;
        let lost_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM repos WHERE state = 'lost'",
            [],
            |row| row.get(0),
        )?;

        let managed_count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM repos WHERE managed_by IS NOT NULL",
            [],
            |row| row.get(0),
        )?;

        let freshness = self.freshness_summary()?;
        let last_scan = self.last_scan_time()?;

        // Collect unique roots from scan config — for now just use recent scan roots
        let roots = self.last_scan_roots()?;

        Ok(IndexSummary {
            total_repos,
            dirty_count,
            unpushed_count,
            orphan_count,
            lost_count,
            managed_count,
            freshness,
            last_scan,
            roots,
        })
    }

    /// Get counts per freshness tier.
    pub fn freshness_summary(&self) -> Result<FreshnessSummary> {
        let count = |tier: &str| -> Result<usize> {
            Ok(self.conn.query_row(
                "SELECT COUNT(*) FROM repos WHERE freshness = ?1",
                [tier],
                |row| row.get(0),
            )?)
        };

        Ok(FreshnessSummary {
            active: count("active")?,
            recent: count("recent")?,
            stale: count("stale")?,
            dormant: count("dormant")?,
            ancient: count("ancient")?,
        })
    }

    /// Record that a scan completed.
    pub fn record_scan(&self, roots: &[PathBuf], repo_count: usize) -> Result<()> {
        let roots_json = serde_json::to_string(roots).unwrap_or_else(|_| "[]".into());
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO scans (completed_at, roots, repo_count) VALUES (?1, ?2, ?3)",
            rusqlite::params![now, roots_json, repo_count],
        )?;
        Ok(())
    }

    /// Get the timestamp of the last completed scan.
    pub fn last_scan_time(&self) -> Result<Option<DateTime<Utc>>> {
        let result = self.conn.query_row(
            "SELECT completed_at FROM scans ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(DateTime::parse_from_rfc3339(&s).ok().map(|dt| dt.to_utc())),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn last_scan_roots(&self) -> Result<Vec<PathBuf>> {
        let result = self.conn.query_row(
            "SELECT roots FROM scans ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        );
        match result {
            Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Vec::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Load a full Repo from its id, including remotes and tags.
    fn load_repo(&self, id: i64) -> Result<Repo> {
        let row = self.conn.query_row(
            "SELECT
                id, name, path, state, default_branch, current_branch,
                branch_count, stale_branch_count, dirty, staged, untracked,
                ahead, behind, last_commit, last_verified, first_seen,
                freshness, category, ownership_type, ownership_label,
                intention, project, role, managed_by
            FROM repos WHERE id = ?1",
            [id],
            |row| {
                Ok(RepoRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    state: row.get(3)?,
                    default_branch: row.get(4)?,
                    current_branch: row.get(5)?,
                    branch_count: row.get(6)?,
                    stale_branch_count: row.get(7)?,
                    dirty: row.get(8)?,
                    staged: row.get(9)?,
                    untracked: row.get(10)?,
                    ahead: row.get(11)?,
                    behind: row.get(12)?,
                    last_commit: row.get(13)?,
                    last_verified: row.get(14)?,
                    first_seen: row.get(15)?,
                    freshness: row.get(16)?,
                    category: row.get(17)?,
                    ownership_type: row.get(18)?,
                    ownership_label: row.get(19)?,
                    intention: row.get(20)?,
                    project: row.get(21)?,
                    role: row.get(22)?,
                    managed_by: row.get(23)?,
                })
            },
        )?;

        let remotes = self.load_remotes(id)?;
        let tags = self.load_tags(id)?;

        Ok(row.into_repo(remotes, tags))
    }

    fn load_remotes(&self, repo_id: i64) -> Result<Vec<Remote>> {
        let mut stmt = self
            .conn
            .prepare("SELECT name, url, push_url FROM remotes WHERE repo_id = ?1")?;
        let remotes = stmt
            .query_map([repo_id], |row| {
                Ok(Remote {
                    name: row.get(0)?,
                    url: row.get(1)?,
                    push_url: row.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(remotes)
    }

    fn load_tags(&self, repo_id: i64) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM tags WHERE repo_id = ?1")?;
        let tags = stmt
            .query_map([repo_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::repo::*;

    fn make_repo(name: &str, path: &str) -> Repo {
        Repo {
            id: 0,
            name: name.to_string(),
            path: PathBuf::from(path),
            state: RepoState::Active,
            remotes: vec![Remote {
                name: "origin".into(),
                url: "git@github.com:initech/api-gateway.git".into(),
                push_url: None,
            }],
            default_branch: Some("main".into()),
            current_branch: Some("feature/auth".into()),
            branch_count: 3,
            stale_branch_count: 1,
            dirty: true,
            staged: false,
            untracked: true,
            ahead: 2,
            behind: 0,
            last_commit: Some(Utc::now()),
            last_verified: Some(Utc::now()),
            first_seen: Utc::now(),
            freshness: Freshness::Active,
            category: Some(Category::Origin),
            ownership: Some(Ownership::Work {
                label: "initech".into(),
            }),
            intention: Some(Intention::Developing),
            managed_by: None,
            tags: vec!["rust".into(), "backend".into()],
            project: Some("platform".into()),
            role: Some("service".into()),
        }
    }

    #[test]
    fn open_in_memory_and_migrate() {
        let idx = Index::open_in_memory().unwrap();
        assert_eq!(idx.schema_version(), SCHEMA_VERSION);
    }

    #[test]
    fn upsert_and_get_by_path() {
        let idx = Index::open_in_memory().unwrap();
        let repo = make_repo("api-gateway", "/home/user/code/api-gateway");

        let id = idx.upsert_repo(&repo).unwrap();
        assert!(id > 0);

        let loaded = idx
            .get_repo_by_path(Path::new("/home/user/code/api-gateway"))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.name, "api-gateway");
        assert!(loaded.dirty);
        assert_eq!(loaded.ahead, 2);
        assert_eq!(loaded.remotes.len(), 1);
        assert_eq!(loaded.remotes[0].name, "origin");
        let mut tags = loaded.tags.clone();
        tags.sort();
        assert_eq!(tags, vec!["backend", "rust"]);
        assert_eq!(loaded.freshness, Freshness::Active);
        assert_eq!(loaded.state, RepoState::Active);
        assert_eq!(
            loaded.ownership,
            Some(Ownership::Work {
                label: "initech".into()
            })
        );
        assert_eq!(loaded.category, Some(Category::Origin));
        assert_eq!(loaded.intention, Some(Intention::Developing));
        assert_eq!(loaded.project, Some("platform".into()));
    }

    #[test]
    fn upsert_updates_existing() {
        let idx = Index::open_in_memory().unwrap();
        let mut repo = make_repo("api-gateway", "/home/user/code/api-gateway");

        idx.upsert_repo(&repo).unwrap();

        repo.dirty = false;
        repo.ahead = 0;
        repo.tags = vec!["rust".into(), "backend".into(), "v2".into()];

        idx.upsert_repo(&repo).unwrap();

        let loaded = idx
            .get_repo_by_path(Path::new("/home/user/code/api-gateway"))
            .unwrap()
            .unwrap();
        assert!(!loaded.dirty);
        assert_eq!(loaded.ahead, 0);
        assert_eq!(loaded.tags.len(), 3);
    }

    #[test]
    fn get_by_name_fuzzy() {
        let idx = Index::open_in_memory().unwrap();
        idx.upsert_repo(&make_repo("api-gateway", "/code/api-gateway"))
            .unwrap();
        idx.upsert_repo(&make_repo("web-frontend", "/code/web-frontend"))
            .unwrap();

        // Exact
        let r = idx.get_repo_by_name("api-gateway").unwrap().unwrap();
        assert_eq!(r.name, "api-gateway");

        // Prefix
        let r = idx.get_repo_by_name("api").unwrap().unwrap();
        assert_eq!(r.name, "api-gateway");

        // Contains
        let r = idx.get_repo_by_name("front").unwrap().unwrap();
        assert_eq!(r.name, "web-frontend");

        // No match
        assert!(idx.get_repo_by_name("nonexistent").unwrap().is_none());
    }

    #[test]
    fn list_repos_empty_filter() {
        let idx = Index::open_in_memory().unwrap();
        idx.upsert_repo(&make_repo("a", "/code/a")).unwrap();
        idx.upsert_repo(&make_repo("b", "/code/b")).unwrap();

        let repos = idx.list_repos(&RepoFilter::default()).unwrap();
        assert_eq!(repos.len(), 2);
    }

    #[test]
    fn list_repos_dirty_filter() {
        let idx = Index::open_in_memory().unwrap();
        let dirty = make_repo("dirty-repo", "/code/dirty");
        let mut clean = make_repo("clean-repo", "/code/clean");
        clean.dirty = false;

        idx.upsert_repo(&dirty).unwrap();
        idx.upsert_repo(&clean).unwrap();

        let filter = RepoFilter {
            dirty: Some(true),
            ..Default::default()
        };
        let repos = idx.list_repos(&filter).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "dirty-repo");
    }

    #[test]
    fn list_repos_name_filter() {
        let idx = Index::open_in_memory().unwrap();
        idx.upsert_repo(&make_repo("api-gateway", "/code/api")).unwrap();
        idx.upsert_repo(&make_repo("web-app", "/code/web")).unwrap();

        let filter = RepoFilter {
            name_contains: Some("api".into()),
            ..Default::default()
        };
        let repos = idx.list_repos(&filter).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "api-gateway");
    }

    #[test]
    fn list_repos_org_filter_in_memory() {
        let idx = Index::open_in_memory().unwrap();
        idx.upsert_repo(&make_repo("a", "/code/a")).unwrap();

        let mut other = make_repo("b", "/code/b");
        other.remotes = vec![Remote {
            name: "origin".into(),
            url: "git@github.com:vandelay/import.git".into(),
            push_url: None,
        }];
        idx.upsert_repo(&other).unwrap();

        let filter = RepoFilter {
            org: Some("initech".into()),
            ..Default::default()
        };
        let repos = idx.list_repos(&filter).unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].name, "a");
    }

    #[test]
    fn mark_lost_and_forget() {
        let idx = Index::open_in_memory().unwrap();
        let id = idx
            .upsert_repo(&make_repo("doomed", "/code/doomed"))
            .unwrap();

        idx.mark_lost(id).unwrap();
        let r = idx
            .get_repo_by_path(Path::new("/code/doomed"))
            .unwrap()
            .unwrap();
        assert_eq!(r.state, RepoState::Lost);

        idx.forget_repo(id).unwrap();
        assert!(idx
            .get_repo_by_path(Path::new("/code/doomed"))
            .unwrap()
            .is_none());
    }

    #[test]
    fn freshness_summary_counts() {
        let idx = Index::open_in_memory().unwrap();

        let mut r1 = make_repo("a", "/code/a");
        r1.freshness = Freshness::Active;
        let mut r2 = make_repo("b", "/code/b");
        r2.freshness = Freshness::Stale;
        let mut r3 = make_repo("c", "/code/c");
        r3.freshness = Freshness::Active;

        idx.upsert_repo(&r1).unwrap();
        idx.upsert_repo(&r2).unwrap();
        idx.upsert_repo(&r3).unwrap();

        let summary = idx.freshness_summary().unwrap();
        assert_eq!(summary.active, 2);
        assert_eq!(summary.stale, 1);
        assert_eq!(summary.recent, 0);
    }

    #[test]
    fn index_summary() {
        let idx = Index::open_in_memory().unwrap();

        let mut dirty_repo = make_repo("dirty", "/code/dirty");
        dirty_repo.dirty = true;
        dirty_repo.ahead = 1;

        let mut orphan_repo = make_repo("orphan", "/code/orphan");
        orphan_repo.remotes.clear();
        orphan_repo.dirty = false;
        orphan_repo.ahead = 0;

        idx.upsert_repo(&dirty_repo).unwrap();
        idx.upsert_repo(&orphan_repo).unwrap();

        let summary = idx.summary().unwrap();
        assert_eq!(summary.total_repos, 2);
        assert_eq!(summary.dirty_count, 1); // only dirty_repo
        assert_eq!(summary.unpushed_count, 1);
        assert_eq!(summary.orphan_count, 1);
    }

    #[test]
    fn record_and_get_scan() {
        let idx = Index::open_in_memory().unwrap();

        assert!(idx.last_scan_time().unwrap().is_none());

        idx.record_scan(&[PathBuf::from("/home/user")], 42).unwrap();

        let ts = idx.last_scan_time().unwrap().unwrap();
        assert!(ts <= Utc::now());

        let roots = idx.last_scan_roots().unwrap();
        assert_eq!(roots, vec![PathBuf::from("/home/user")]);
    }

    #[test]
    fn ownership_roundtrips() {
        let idx = Index::open_in_memory().unwrap();

        // Personal
        let mut r = make_repo("personal", "/code/personal");
        r.ownership = Some(Ownership::Personal);
        idx.upsert_repo(&r).unwrap();
        let loaded = idx
            .get_repo_by_path(Path::new("/code/personal"))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.ownership, Some(Ownership::Personal));

        // Community
        let mut r = make_repo("community", "/code/community");
        r.ownership = Some(Ownership::Community);
        idx.upsert_repo(&r).unwrap();
        let loaded = idx
            .get_repo_by_path(Path::new("/code/community"))
            .unwrap()
            .unwrap();
        assert_eq!(loaded.ownership, Some(Ownership::Community));

        // None
        let mut r = make_repo("none", "/code/none");
        r.ownership = None;
        idx.upsert_repo(&r).unwrap();
        let loaded = idx
            .get_repo_by_path(Path::new("/code/none"))
            .unwrap()
            .unwrap();
        assert!(loaded.ownership.is_none());
    }
}

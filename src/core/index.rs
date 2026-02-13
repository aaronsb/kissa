use std::path::Path;

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::filter::RepoFilter;
use super::repo::{Repo, RepoId};
use crate::error::Result;

/// The persistent repo index backed by SQLite (ADR-103).
pub struct Index {
    _conn: rusqlite::Connection,
}

impl Index {
    /// Open or create the index database at the given path. Enables WAL mode.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = rusqlite::Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "wal")?;
        let index = Self { _conn: conn };
        index.migrate()?;
        Ok(index)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = rusqlite::Connection::open_in_memory()?;
        let index = Self { _conn: conn };
        index.migrate()?;
        Ok(index)
    }

    /// Run schema migrations to latest version.
    pub fn migrate(&self) -> Result<()> {
        // TODO: Phase 2 - implement schema creation
        Ok(())
    }

    /// Insert or update a repo in the index.
    pub fn upsert_repo(&self, _repo: &Repo) -> Result<RepoId> {
        todo!("Phase 2: implement upsert")
    }

    /// Get a repo by its absolute path.
    pub fn get_repo_by_path(&self, _path: &Path) -> Result<Option<Repo>> {
        todo!("Phase 2: implement get_repo_by_path")
    }

    /// Get a repo by name (fuzzy: prefix match, then contains).
    pub fn get_repo_by_name(&self, _name: &str) -> Result<Option<Repo>> {
        todo!("Phase 2: implement get_repo_by_name")
    }

    /// List repos matching the given filter.
    pub fn list_repos(&self, _filter: &RepoFilter) -> Result<Vec<Repo>> {
        todo!("Phase 2: implement list_repos")
    }

    /// Get all repos (unfiltered).
    pub fn all_repos(&self) -> Result<Vec<Repo>> {
        self.list_repos(&RepoFilter::default())
    }

    /// Mark a repo as lost (path no longer exists).
    pub fn mark_lost(&self, _id: RepoId) -> Result<()> {
        todo!("Phase 2: implement mark_lost")
    }

    /// Remove a repo from the index permanently.
    pub fn forget_repo(&self, _id: RepoId) -> Result<()> {
        todo!("Phase 2: implement forget_repo")
    }

    /// Get summary statistics for the entire index.
    pub fn summary(&self) -> Result<IndexSummary> {
        todo!("Phase 2: implement summary")
    }

    /// Get counts per freshness tier.
    pub fn freshness_summary(&self) -> Result<FreshnessSummary> {
        todo!("Phase 2: implement freshness_summary")
    }

    /// Record that a scan completed.
    pub fn record_scan(
        &self,
        _roots: &[std::path::PathBuf],
        _repo_count: usize,
    ) -> Result<()> {
        todo!("Phase 2: implement record_scan")
    }

    /// Get the timestamp of the last completed scan.
    pub fn last_scan_time(&self) -> Result<Option<DateTime<Utc>>> {
        todo!("Phase 2: implement last_scan_time")
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FreshnessSummary {
    pub active: usize,
    pub recent: usize,
    pub stale: usize,
    pub dormant: usize,
    pub ancient: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexSummary {
    pub total_repos: usize,
    pub dirty_count: usize,
    pub unpushed_count: usize,
    pub orphan_count: usize,
    pub lost_count: usize,
    pub freshness: FreshnessSummary,
    pub last_scan: Option<DateTime<Utc>>,
    pub roots: Vec<std::path::PathBuf>,
}

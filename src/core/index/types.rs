use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::core::repo::{Freshness, Ownership, Remote, Repo, RepoState};

/// Internal row struct for mapping SQL columns to Repo.
pub(super) struct RepoRow {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub state: String,
    pub default_branch: Option<String>,
    pub current_branch: Option<String>,
    pub branch_count: u32,
    pub stale_branch_count: u32,
    pub dirty: bool,
    pub staged: bool,
    pub untracked: bool,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit: Option<String>,
    pub last_verified: Option<String>,
    pub first_seen: String,
    pub freshness: String,
    pub category: Option<String>,
    pub ownership_type: Option<String>,
    pub ownership_label: Option<String>,
    pub intention: Option<String>,
    pub project: Option<String>,
    pub role: Option<String>,
    pub managed_by: Option<String>,
}

impl RepoRow {
    pub fn into_repo(self, remotes: Vec<Remote>, tags: Vec<String>) -> Repo {
        let state = serde_plain::from_str(&self.state).unwrap_or(RepoState::Active);
        let freshness = serde_plain::from_str(&self.freshness).unwrap_or(Freshness::Ancient);
        let category = self
            .category
            .as_deref()
            .and_then(|s| serde_plain::from_str(s).ok());
        let intention = self
            .intention
            .as_deref()
            .and_then(|s| serde_plain::from_str(s).ok());
        let ownership = self.ownership_type.as_deref().and_then(|t| match t {
            "personal" => Some(Ownership::Personal),
            "work" => Some(Ownership::Work {
                label: self.ownership_label.clone().unwrap_or_default(),
            }),
            "community" => Some(Ownership::Community),
            "thirdparty" => Some(Ownership::ThirdParty),
            "local" => Some(Ownership::Local),
            _ => None,
        });

        fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
            DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.to_utc())
        }

        Repo {
            id: self.id,
            name: self.name,
            path: PathBuf::from(self.path),
            state,
            remotes,
            default_branch: self.default_branch,
            current_branch: self.current_branch,
            branch_count: self.branch_count,
            stale_branch_count: self.stale_branch_count,
            dirty: self.dirty,
            staged: self.staged,
            untracked: self.untracked,
            ahead: self.ahead,
            behind: self.behind,
            last_commit: self.last_commit.as_deref().and_then(parse_dt),
            last_verified: self.last_verified.as_deref().and_then(parse_dt),
            first_seen: parse_dt(&self.first_seen).unwrap_or_else(Utc::now),
            freshness,
            category,
            ownership,
            intention,
            managed_by: self.managed_by,
            tags,
            project: self.project,
            role: self.role,
        }
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
    pub managed_count: usize,
    pub freshness: FreshnessSummary,
    pub last_scan: Option<DateTime<Utc>>,
    pub roots: Vec<PathBuf>,
}

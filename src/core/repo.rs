use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique identifier for a repo in the index.
pub type RepoId = i64;

/// A discovered git repository with all extracted vitals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub id: RepoId,
    pub name: String,
    pub path: PathBuf,
    pub state: RepoState,

    // Git state
    pub remotes: Vec<Remote>,
    pub default_branch: Option<String>,
    pub current_branch: Option<String>,
    pub branch_count: u32,
    pub stale_branch_count: u32,

    // Working tree state
    pub dirty: bool,
    pub staged: bool,
    pub untracked: bool,
    pub ahead: u32,
    pub behind: u32,

    // Timestamps
    pub last_commit: Option<DateTime<Utc>>,
    pub last_verified: Option<DateTime<Utc>>,
    pub first_seen: DateTime<Utc>,

    // Classification (ADR-104)
    pub freshness: Freshness,
    pub category: Option<Category>,
    pub ownership: Option<Ownership>,
    pub intention: Option<Intention>,

    // Classification (ADR-106)
    pub managed_by: Option<String>,

    // User metadata
    pub tags: Vec<String>,
    pub project: Option<String>,
    pub role: Option<String>,
}

/// Lifecycle state of a repo in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoState {
    Active,
    Lost,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remote {
    pub name: String,
    pub url: String,
    pub push_url: Option<String>,
}

/// Freshness tiers based on last commit time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Freshness {
    Active,
    Recent,
    Stale,
    Dormant,
    Ancient,
}

impl Freshness {
    /// Compute freshness from a commit timestamp.
    pub fn from_commit_time(last_commit: Option<DateTime<Utc>>) -> Self {
        let Some(ts) = last_commit else {
            return Freshness::Ancient;
        };
        let days = (Utc::now() - ts).num_days();
        match days {
            0..=7 => Freshness::Active,
            8..=30 => Freshness::Recent,
            31..=90 => Freshness::Stale,
            91..=365 => Freshness::Dormant,
            _ => Freshness::Ancient,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Freshness::Active => "active",
            Freshness::Recent => "recent",
            Freshness::Stale => "stale",
            Freshness::Dormant => "dormant",
            Freshness::Ancient => "ancient",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Origin,
    Clone,
    Fork,
    Mirror,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ownership {
    Personal,
    #[serde(rename = "work")]
    Work { label: String },
    Community,
    ThirdParty,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Intention {
    Developing,
    Contributing,
    Reference,
    Dependency,
    Dotfiles,
    Infrastructure,
    Experiment,
    Archived,
}

impl Repo {
    /// Create a new Repo from extracted vitals and discovery path.
    pub fn from_vitals(vitals: RepoVitals, path: std::path::PathBuf) -> Self {
        Self {
            id: 0,
            name: vitals.name,
            path,
            state: RepoState::Active,
            remotes: vitals.remotes,
            default_branch: vitals.default_branch,
            current_branch: vitals.current_branch,
            branch_count: vitals.branch_count,
            stale_branch_count: vitals.stale_branch_count,
            dirty: vitals.dirty,
            staged: vitals.staged,
            untracked: vitals.untracked,
            ahead: vitals.ahead,
            behind: vitals.behind,
            last_commit: vitals.last_commit,
            last_verified: Some(chrono::Utc::now()),
            first_seen: chrono::Utc::now(),
            freshness: Freshness::from_commit_time(vitals.last_commit),
            category: None,
            ownership: None,
            intention: None,
            managed_by: None,
            tags: vec![],
            project: None,
            role: None,
        }
    }
}

/// Lightweight struct of git-extracted data before index enrichment.
#[derive(Debug, Clone)]
pub struct RepoVitals {
    pub name: String,
    pub remotes: Vec<Remote>,
    pub default_branch: Option<String>,
    pub current_branch: Option<String>,
    pub branch_count: u32,
    pub stale_branch_count: u32,
    pub dirty: bool,
    pub staged: bool,
    pub untracked: bool,
    pub ahead: u32,
    pub behind: u32,
    pub last_commit: Option<DateTime<Utc>>,
    pub is_bare: bool,
}

/// Parsed remote URL information.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub platform: String,
    pub org: String,
    pub repo_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_from_recent_commit() {
        let now = Utc::now();
        assert_eq!(Freshness::from_commit_time(Some(now)), Freshness::Active);
    }

    #[test]
    fn freshness_from_none() {
        assert_eq!(Freshness::from_commit_time(None), Freshness::Ancient);
    }

    #[test]
    fn freshness_ordering() {
        assert!(Freshness::Active < Freshness::Ancient);
    }
}

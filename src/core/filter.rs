use serde::{Deserialize, Serialize};

use super::repo::{Freshness, Repo, RepoState};

/// A composable set of repo filters. All fields are AND-combined.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoFilter {
    pub dirty: Option<bool>,
    pub unpushed: Option<bool>,
    pub orphan: Option<bool>,
    pub org: Option<String>,
    pub freshness: Option<Freshness>,
    pub ownership: Option<String>,
    pub intention: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub path_prefix: Option<String>,
    pub has_remote: Option<bool>,
    pub name_contains: Option<String>,
    pub state: Option<RepoState>,
}

impl RepoFilter {
    /// Test whether a Repo matches this filter in-memory.
    pub fn matches(&self, repo: &Repo) -> bool {
        if let Some(dirty) = self.dirty {
            if repo.dirty != dirty {
                return false;
            }
        }
        if let Some(true) = self.unpushed {
            if repo.ahead == 0 {
                return false;
            }
        }
        if let Some(true) = self.orphan {
            if !repo.remotes.is_empty() {
                return false;
            }
        }
        if let Some(ref freshness) = self.freshness {
            if repo.freshness != *freshness {
                return false;
            }
        }
        if let Some(ref prefix) = self.path_prefix {
            if !repo.path.to_string_lossy().starts_with(prefix.as_str()) {
                return false;
            }
        }
        if let Some(has_remote) = self.has_remote {
            if repo.remotes.is_empty() == has_remote {
                return false;
            }
        }
        if let Some(ref name) = self.name_contains {
            if !repo.name.contains(name.as_str()) {
                return false;
            }
        }
        if let Some(ref state) = self.state {
            if repo.state != *state {
                return false;
            }
        }
        // TODO: org, ownership, intention, category, tags filtering
        true
    }
}

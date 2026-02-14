use serde::{Deserialize, Serialize};

use super::git_ops::parse_remote_org;
use super::repo::{Freshness, Ownership, Repo, RepoState};

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
    pub managed_by: Option<String>,
    /// None = show all, Some(true) = only managed, Some(false) = only unmanaged
    pub show_managed: Option<bool>,
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
            if !repo.name.to_lowercase().contains(&name.to_lowercase()) {
                return false;
            }
        }
        if let Some(ref state) = self.state {
            if repo.state != *state {
                return false;
            }
        }
        if let Some(ref org) = self.org {
            if !repo_matches_org(repo, org) {
                return false;
            }
        }
        if let Some(ref ownership) = self.ownership {
            if !repo_matches_ownership(repo, ownership) {
                return false;
            }
        }
        if let Some(ref intention) = self.intention {
            if !repo_matches_enum_str(&repo.intention, intention) {
                return false;
            }
        }
        if let Some(ref category) = self.category {
            if !repo_matches_enum_str(&repo.category, category) {
                return false;
            }
        }
        if let Some(ref tags) = self.tags {
            // All specified tags must be present
            for tag in tags {
                if !repo.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
                    return false;
                }
            }
        }
        if let Some(ref mb) = self.managed_by {
            match &repo.managed_by {
                Some(rmb) => {
                    if !rmb.eq_ignore_ascii_case(mb) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if let Some(show) = self.show_managed {
            let is_managed = repo.managed_by.is_some();
            if show != is_managed {
                return false;
            }
        }
        true
    }

    /// Returns true if no filters are set.
    pub fn is_empty(&self) -> bool {
        self.dirty.is_none()
            && self.unpushed.is_none()
            && self.orphan.is_none()
            && self.org.is_none()
            && self.freshness.is_none()
            && self.ownership.is_none()
            && self.intention.is_none()
            && self.category.is_none()
            && self.tags.is_none()
            && self.path_prefix.is_none()
            && self.has_remote.is_none()
            && self.name_contains.is_none()
            && self.state.is_none()
            && self.managed_by.is_none()
            && self.show_managed.is_none()
    }
}

/// Check if any remote's org matches the filter value.
fn repo_matches_org(repo: &Repo, org_filter: &str) -> bool {
    repo.remotes.iter().any(|remote| {
        parse_remote_org(&remote.url)
            .is_some_and(|info| info.org.eq_ignore_ascii_case(org_filter))
    })
}

/// Check ownership classification matches filter string.
/// Accepts: "personal", "work:label", "community", "third-party", "local"
fn repo_matches_ownership(repo: &Repo, filter: &str) -> bool {
    let Some(ref ownership) = repo.ownership else {
        return false;
    };
    match ownership {
        Ownership::Personal => filter.eq_ignore_ascii_case("personal"),
        Ownership::Work { label } => {
            if let Some(work_label) = filter.strip_prefix("work:") {
                label.eq_ignore_ascii_case(work_label)
            } else {
                filter.eq_ignore_ascii_case("work")
            }
        }
        Ownership::Community => filter.eq_ignore_ascii_case("community"),
        Ownership::ThirdParty => {
            filter.eq_ignore_ascii_case("third-party")
                || filter.eq_ignore_ascii_case("thirdparty")
        }
        Ownership::Local => filter.eq_ignore_ascii_case("local"),
    }
}

/// Generic enum-to-string match for serde-renamed enums.
fn repo_matches_enum_str<T: serde::Serialize>(value: &Option<T>, filter: &str) -> bool {
    let Some(val) = value else {
        return false;
    };
    // Serialize to JSON string, strip quotes, compare case-insensitively
    if let Ok(json) = serde_json::to_string(val) {
        let serialized = json.trim_matches('"');
        serialized.eq_ignore_ascii_case(filter)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::repo::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn make_repo(name: &str) -> Repo {
        Repo {
            id: 1,
            name: name.to_string(),
            path: PathBuf::from(format!("/home/user/code/{}", name)),
            state: RepoState::Active,
            remotes: vec![Remote {
                name: "origin".into(),
                url: "git@github.com:initech/api-gateway.git".into(),
                push_url: None,
            }],
            default_branch: Some("main".into()),
            current_branch: Some("main".into()),
            branch_count: 1,
            stale_branch_count: 0,
            dirty: false,
            staged: false,
            untracked: false,
            ahead: 0,
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
            tags: vec!["rust".into(), "work".into()],
            project: None,
            role: None,
        }
    }

    #[test]
    fn empty_filter_matches_everything() {
        let filter = RepoFilter::default();
        assert!(filter.matches(&make_repo("test")));
        assert!(filter.is_empty());
    }

    #[test]
    fn dirty_filter() {
        let filter = RepoFilter {
            dirty: Some(true),
            ..Default::default()
        };
        let mut repo = make_repo("test");
        assert!(!filter.matches(&repo));
        repo.dirty = true;
        assert!(filter.matches(&repo));
    }

    #[test]
    fn unpushed_filter() {
        let filter = RepoFilter {
            unpushed: Some(true),
            ..Default::default()
        };
        let mut repo = make_repo("test");
        assert!(!filter.matches(&repo)); // ahead = 0
        repo.ahead = 3;
        assert!(filter.matches(&repo));
    }

    #[test]
    fn orphan_filter() {
        let filter = RepoFilter {
            orphan: Some(true),
            ..Default::default()
        };
        let repo = make_repo("test"); // has remotes
        assert!(!filter.matches(&repo));

        let mut orphan = make_repo("orphan");
        orphan.remotes.clear();
        assert!(filter.matches(&orphan));
    }

    #[test]
    fn org_filter() {
        let filter = RepoFilter {
            org: Some("initech".into()),
            ..Default::default()
        };
        assert!(filter.matches(&make_repo("test")));

        let filter_wrong = RepoFilter {
            org: Some("vandelay".into()),
            ..Default::default()
        };
        assert!(!filter_wrong.matches(&make_repo("test")));
    }

    #[test]
    fn ownership_filter() {
        let repo = make_repo("test"); // ownership = Work { label: "initech" }

        let filter = RepoFilter {
            ownership: Some("work:initech".into()),
            ..Default::default()
        };
        assert!(filter.matches(&repo));

        let filter_generic = RepoFilter {
            ownership: Some("work".into()),
            ..Default::default()
        };
        assert!(filter_generic.matches(&repo));

        let filter_wrong = RepoFilter {
            ownership: Some("personal".into()),
            ..Default::default()
        };
        assert!(!filter_wrong.matches(&repo));
    }

    #[test]
    fn tags_filter() {
        let repo = make_repo("test"); // tags: ["rust", "work"]

        let filter = RepoFilter {
            tags: Some(vec!["rust".into()]),
            ..Default::default()
        };
        assert!(filter.matches(&repo));

        let filter_both = RepoFilter {
            tags: Some(vec!["rust".into(), "work".into()]),
            ..Default::default()
        };
        assert!(filter_both.matches(&repo));

        let filter_missing = RepoFilter {
            tags: Some(vec!["python".into()]),
            ..Default::default()
        };
        assert!(!filter_missing.matches(&repo));
    }

    #[test]
    fn combined_filters() {
        let filter = RepoFilter {
            dirty: Some(true),
            org: Some("initech".into()),
            ..Default::default()
        };
        let mut repo = make_repo("test");
        assert!(!filter.matches(&repo)); // not dirty

        repo.dirty = true;
        assert!(filter.matches(&repo)); // dirty + initech
    }

    #[test]
    fn name_contains_case_insensitive() {
        let filter = RepoFilter {
            name_contains: Some("API".into()),
            ..Default::default()
        };
        assert!(filter.matches(&make_repo("api-gateway")));
        assert!(!filter.matches(&make_repo("frontend")));
    }
}

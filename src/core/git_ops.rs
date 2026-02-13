use std::path::Path;

use chrono::{DateTime, TimeZone, Utc};
use git2::{BranchType, Repository, StatusOptions};

use crate::error::{KissaError, Result};

use super::repo::{Remote, RemoteInfo, RepoVitals};

/// Extract full vitals from a git repo at the given path.
pub fn extract_vitals(path: &Path) -> Result<RepoVitals> {
    let repo = Repository::open(path).map_err(|e| KissaError::Git {
        path: path.to_path_buf(),
        source: e,
    })?;

    let remotes = extract_remotes(&repo);
    let name = infer_name(path, &remotes);
    let is_bare = repo.is_bare();

    let default_branch = detect_default_branch(&repo);
    let current_branch = if is_bare {
        None
    } else {
        repo.head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
    };

    let (branch_count, stale_branch_count) = count_branches(&repo);
    let (dirty, staged, untracked) = if is_bare {
        (false, false, false)
    } else {
        working_tree_status(&repo)
    };

    let (ahead, behind) = ahead_behind(&repo);
    let last_commit = last_commit_time(&repo);

    Ok(RepoVitals {
        name,
        remotes,
        default_branch,
        current_branch,
        branch_count,
        stale_branch_count,
        dirty,
        staged,
        untracked,
        ahead,
        behind,
        last_commit,
        is_bare,
    })
}

/// Extract all remotes from a repository.
fn extract_remotes(repo: &Repository) -> Vec<Remote> {
    let Ok(remote_names) = repo.remotes() else {
        return Vec::new();
    };
    remote_names
        .iter()
        .flatten()
        .filter_map(|name| {
            let remote = repo.find_remote(name).ok()?;
            Some(Remote {
                name: name.to_string(),
                url: remote.url().unwrap_or("").to_string(),
                push_url: remote.pushurl().map(String::from),
            })
        })
        .collect()
}

/// Detect the default branch (HEAD target or common names).
fn detect_default_branch(repo: &Repository) -> Option<String> {
    // Try HEAD's target
    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            return Some(name.to_string());
        }
    }
    // Try common default branch names
    for name in &["main", "master", "develop", "trunk"] {
        if repo
            .find_branch(name, BranchType::Local)
            .is_ok()
        {
            return Some(name.to_string());
        }
    }
    None
}

/// Count total local branches and stale branches (> 90 days since last commit).
fn count_branches(repo: &Repository) -> (u32, u32) {
    let Ok(branches) = repo.branches(Some(BranchType::Local)) else {
        return (0, 0);
    };

    let mut total = 0u32;
    let mut stale = 0u32;
    let ninety_days_ago = Utc::now() - chrono::Duration::days(90);

    for branch in branches.flatten() {
        total += 1;
        let (branch_ref, _) = branch;
        if let Ok(commit) = branch_ref.get().peel_to_commit() {
            let time = commit.time();
            if let Some(dt) = Utc.timestamp_opt(time.seconds(), 0).single() {
                if dt < ninety_days_ago {
                    stale += 1;
                }
            }
        }
    }
    (total, stale)
}

/// Check working tree status: (dirty, staged, untracked).
fn working_tree_status(repo: &Repository) -> (bool, bool, bool) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    let Ok(statuses) = repo.statuses(Some(&mut opts)) else {
        return (false, false, false);
    };

    let mut dirty = false;
    let mut staged = false;
    let mut untracked = false;

    for entry in statuses.iter() {
        let s = entry.status();

        if s.intersects(
            git2::Status::WT_MODIFIED
                | git2::Status::WT_DELETED
                | git2::Status::WT_RENAMED
                | git2::Status::WT_TYPECHANGE,
        ) {
            dirty = true;
        }

        if s.intersects(
            git2::Status::INDEX_NEW
                | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED
                | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE,
        ) {
            staged = true;
        }

        if s.contains(git2::Status::WT_NEW) {
            untracked = true;
        }
    }

    (dirty, staged, untracked)
}

/// Compute ahead/behind counts relative to upstream tracking branch.
fn ahead_behind(repo: &Repository) -> (u32, u32) {
    let Ok(head) = repo.head() else {
        return (0, 0);
    };
    let local_oid = match head.target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    // Find the upstream tracking branch
    let Ok(branch) = repo.find_branch(
        head.shorthand().unwrap_or(""),
        BranchType::Local,
    ) else {
        return (0, 0);
    };

    let Ok(upstream) = branch.upstream() else {
        return (0, 0);
    };

    let upstream_oid = match upstream.get().target() {
        Some(oid) => oid,
        None => return (0, 0),
    };

    repo.graph_ahead_behind(local_oid, upstream_oid)
        .map(|(a, b)| (a as u32, b as u32))
        .unwrap_or((0, 0))
}

/// Get the timestamp of the most recent commit on HEAD.
fn last_commit_time(repo: &Repository) -> Option<DateTime<Utc>> {
    let head = repo.head().ok()?;
    let commit = head.peel_to_commit().ok()?;
    let time = commit.time();
    Utc.timestamp_opt(time.seconds(), 0).single()
}

/// Infer the repo name from path or remote URL.
pub fn infer_name(path: &Path, remotes: &[Remote]) -> String {
    // Prefer remote URL repo name, fall back to directory name
    if let Some(remote) = remotes.iter().find(|r| r.name == "origin") {
        if let Some(info) = parse_remote_org(&remote.url) {
            return info.repo_name;
        }
    }
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".into())
}

/// Parse org/owner from a remote URL.
pub fn parse_remote_org(url: &str) -> Option<RemoteInfo> {
    // Handle SSH: git@github.com:org/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        let (platform, path) = rest.split_once(':')?;
        let parts: Vec<&str> = path.trim_end_matches(".git").split('/').collect();
        if parts.len() >= 2 {
            return Some(RemoteInfo {
                platform: platform.to_string(),
                org: parts[0].to_string(),
                repo_name: parts[1].to_string(),
            });
        }
    }

    // Handle HTTPS: https://github.com/org/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        let url = url.trim_end_matches(".git");
        let parts: Vec<&str> = url.split('/').collect();
        if parts.len() >= 5 {
            return Some(RemoteInfo {
                platform: parts[2].to_string(),
                org: parts[3].to_string(),
                repo_name: parts[4].to_string(),
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn parse_ssh_url() {
        let info = parse_remote_org("git@github.com:initech/api-gateway.git").unwrap();
        assert_eq!(info.platform, "github.com");
        assert_eq!(info.org, "initech");
        assert_eq!(info.repo_name, "api-gateway");
    }

    #[test]
    fn parse_https_url() {
        let info = parse_remote_org("https://github.com/aaronsb/kissa.git").unwrap();
        assert_eq!(info.platform, "github.com");
        assert_eq!(info.org, "aaronsb");
        assert_eq!(info.repo_name, "kissa");
    }

    #[test]
    fn parse_https_no_git_suffix() {
        let info = parse_remote_org("https://gitlab.com/myorg/myrepo").unwrap();
        assert_eq!(info.platform, "gitlab.com");
        assert_eq!(info.org, "myorg");
        assert_eq!(info.repo_name, "myrepo");
    }

    #[test]
    fn infer_name_from_remote() {
        let remotes = vec![Remote {
            name: "origin".into(),
            url: "git@github.com:aaronsb/kissa.git".into(),
            push_url: None,
        }];
        assert_eq!(infer_name(Path::new("/code/whatever"), &remotes), "kissa");
    }

    #[test]
    fn infer_name_from_path() {
        let remotes = vec![];
        assert_eq!(
            infer_name(Path::new("/home/user/code/my-project"), &remotes),
            "my-project"
        );
    }

    #[test]
    fn extract_vitals_from_real_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path();

        // Create a git repo with git2
        let repo = Repository::init(repo_path).unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            let test_file = repo_path.join("README.md");
            fs::write(&test_file, "# test").unwrap();
            index.add_path(Path::new("README.md")).unwrap();
            index.write().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Create a dirty file
        fs::write(repo_path.join("dirty.txt"), "uncommitted").unwrap();

        let vitals = extract_vitals(repo_path).unwrap();
        assert!(!vitals.name.is_empty());
        assert!(vitals.dirty || vitals.untracked); // dirty.txt is untracked
        assert!(!vitals.is_bare);
        assert!(vitals.last_commit.is_some());
        assert!(vitals.branch_count >= 1);
        assert_eq!(vitals.ahead, 0);
        assert_eq!(vitals.behind, 0);
    }

    #[test]
    fn extract_vitals_bare_repo() {
        let dir = tempfile::tempdir().unwrap();
        let repo_path = dir.path().join("bare.git");
        Repository::init_bare(&repo_path).unwrap();

        let vitals = extract_vitals(&repo_path).unwrap();
        assert!(vitals.is_bare);
        assert!(!vitals.dirty);
        assert!(!vitals.staged);
        assert!(!vitals.untracked);
    }

    #[test]
    fn extract_vitals_nonexistent_path() {
        let result = extract_vitals(Path::new("/nonexistent/repo"));
        assert!(result.is_err());
    }
}

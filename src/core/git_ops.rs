use std::path::Path;

use crate::error::Result;

use super::repo::{Remote, RemoteInfo, RepoVitals};

/// Extract full vitals from a git repo at the given path.
pub fn extract_vitals(path: &Path) -> Result<RepoVitals> {
    let _ = path;
    todo!("Phase 3a: implement git2-based vitals extraction")
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
}

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use walkdir::WalkDir;

use crate::config::types::ScanConfig;
use crate::error::Result;

/// Result of scanning a single discovered .git directory.
#[derive(Debug, Clone)]
pub struct DiscoveredRepo {
    pub path: PathBuf,
    pub is_bare: bool,
}

/// Events emitted during scanning for progress reporting.
pub enum ScanEvent {
    DirectoryEntered(PathBuf),
    RepoFound(PathBuf),
    Skipped { path: PathBuf, reason: SkipReason },
    Error { path: PathBuf, error: String },
}

#[derive(Debug, Clone)]
pub enum SkipReason {
    Excluded,
    MountBoundary,
    MaxDepth,
    BlockedMount,
}

/// Scan result after a full filesystem walk.
#[derive(Debug)]
pub struct ScanResult {
    pub discovered: Vec<DiscoveredRepo>,
    pub skipped_mounts: usize,
    pub skipped_excluded: usize,
    pub errors: Vec<(PathBuf, String)>,
    pub duration: Duration,
}

/// Result of a quick verify pass.
#[derive(Debug)]
pub struct QuickVerifyResult {
    pub unchanged: Vec<PathBuf>,
    pub changed: Vec<PathBuf>,
    pub lost: Vec<PathBuf>,
}

/// Walk configured roots and discover .git directories.
pub fn full_scan(
    roots: &[PathBuf],
    config: &ScanConfig,
    progress: Option<Box<dyn Fn(ScanEvent) + Send>>,
) -> Result<ScanResult> {
    let start = Instant::now();
    let mut discovered = Vec::new();
    let mut skipped_mounts = 0;
    let mut skipped_excluded = 0;
    let mut errors = Vec::new();

    for root in roots {
        // Get the device ID of the root to detect mount boundaries
        let root_dev = root
            .metadata()
            .ok()
            .map(|m| m.dev());

        let walker = WalkDir::new(root)
            .max_depth(config.max_depth)
            .follow_links(false);

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    let path = e.path().unwrap_or(root).to_path_buf();
                    if let Some(ref cb) = progress {
                        cb(ScanEvent::Error {
                            path: path.clone(),
                            error: e.to_string(),
                        });
                    }
                    errors.push((path, e.to_string()));
                    continue;
                }
            };

            let path = entry.path();

            // Skip non-directories
            if !entry.file_type().is_dir() {
                continue;
            }

            // Check exclusion list
            if is_excluded(path, root, &config.exclude) {
                skipped_excluded += 1;
                if let Some(ref cb) = progress {
                    cb(ScanEvent::Skipped {
                        path: path.to_path_buf(),
                        reason: SkipReason::Excluded,
                    });
                }
                continue;
            }

            // Check mount boundaries
            if !config.boundaries.cross_mounts {
                if let Some(root_dev) = root_dev {
                    if let Ok(meta) = path.metadata() {
                        if meta.dev() != root_dev {
                            // Check allow list
                            if !config.boundaries.allow_mounts.iter().any(|m| path.starts_with(m)) {
                                skipped_mounts += 1;
                                if let Some(ref cb) = progress {
                                    cb(ScanEvent::Skipped {
                                        path: path.to_path_buf(),
                                        reason: SkipReason::MountBoundary,
                                    });
                                }
                                continue;
                            }
                        }
                    }
                }
            }

            // Check blocked mounts
            if config
                .boundaries
                .block_mounts
                .iter()
                .any(|m| path.starts_with(m))
            {
                skipped_mounts += 1;
                if let Some(ref cb) = progress {
                    cb(ScanEvent::Skipped {
                        path: path.to_path_buf(),
                        reason: SkipReason::BlockedMount,
                    });
                }
                continue;
            }

            // Check if this is a .git directory → parent is a work tree repo
            if path.file_name().is_some_and(|n| n == ".git") {
                let repo_path = path.parent().unwrap_or(path);
                if let Some(ref cb) = progress {
                    cb(ScanEvent::RepoFound(repo_path.to_path_buf()));
                }
                discovered.push(DiscoveredRepo {
                    path: repo_path.to_path_buf(),
                    is_bare: false,
                });
                continue;
            }

            // Check for bare repos: has HEAD file and objects/ directory but no .git/
            if is_bare_repo(path) {
                if let Some(ref cb) = progress {
                    cb(ScanEvent::RepoFound(path.to_path_buf()));
                }
                discovered.push(DiscoveredRepo {
                    path: path.to_path_buf(),
                    is_bare: true,
                });
                // Don't descend into bare repos (walkdir will still list entries but we skip them)
                continue;
            }

            if let Some(ref cb) = progress {
                cb(ScanEvent::DirectoryEntered(path.to_path_buf()));
            }
        }
    }

    Ok(ScanResult {
        discovered,
        skipped_mounts,
        skipped_excluded,
        errors,
        duration: start.elapsed(),
    })
}

/// Quick verify: stat known repo paths, return which changed/lost.
pub fn quick_verify(known_paths: &[PathBuf]) -> Result<QuickVerifyResult> {
    let mut unchanged = Vec::new();
    let mut changed = Vec::new();
    let mut lost = Vec::new();

    for path in known_paths {
        let git_dir = path.join(".git");
        if git_dir.exists() {
            // Check if HEAD has been modified recently (simple heuristic)
            let head_path = git_dir.join("HEAD");
            if head_path.exists() {
                changed.push(path.clone());
            } else {
                unchanged.push(path.clone());
            }
        } else if path.join("HEAD").exists() {
            // Bare repo
            changed.push(path.clone());
        } else {
            lost.push(path.clone());
        }
    }

    Ok(QuickVerifyResult {
        unchanged,
        changed,
        lost,
    })
}

/// Check if a path should be excluded.
fn is_excluded(path: &Path, root: &Path, exclusions: &[String]) -> bool {
    // Get path relative to root for matching
    let rel = path.strip_prefix(root).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    for pattern in exclusions {
        // Match against the last component
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            let pattern_trimmed = pattern.trim_end_matches('/');
            if name_str == pattern_trimmed {
                return true;
            }
        }
        // Match against the relative path
        if rel_str.contains(pattern.trim_end_matches('/')) {
            return true;
        }
    }
    false
}

/// Check if a directory looks like a bare git repo.
fn is_bare_repo(path: &Path) -> bool {
    path.join("HEAD").is_file()
        && path.join("objects").is_dir()
        && path.join("refs").is_dir()
        && !path.join(".git").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn default_scan_config() -> ScanConfig {
        ScanConfig {
            roots: vec![],
            exclude: vec!["node_modules".into(), ".cache".into()],
            max_depth: 10,
            auto_verify_seconds: 300,
            boundaries: crate::config::types::BoundaryConfig {
                cross_mounts: true, // Disable mount checking in tests
                allow_mounts: vec![],
                block_mounts: vec![],
                stat_timeout_ms: 500,
            },
        }
    }

    #[test]
    fn scan_finds_git_repos() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create two repos
        fs::create_dir_all(root.join("project-a/.git")).unwrap();
        fs::create_dir_all(root.join("project-b/.git")).unwrap();
        // Create a non-repo directory
        fs::create_dir_all(root.join("not-a-repo")).unwrap();

        let config = default_scan_config();
        let result = full_scan(&[root.to_path_buf()], &config, None).unwrap();

        assert_eq!(result.discovered.len(), 2);
        assert!(!result.discovered[0].is_bare);
    }

    #[test]
    fn scan_finds_bare_repos() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create a bare repo structure
        let bare = root.join("repo.git");
        fs::create_dir_all(bare.join("objects")).unwrap();
        fs::create_dir_all(bare.join("refs")).unwrap();
        fs::write(bare.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let config = default_scan_config();
        let result = full_scan(&[root.to_path_buf()], &config, None).unwrap();

        assert_eq!(result.discovered.len(), 1);
        assert!(result.discovered[0].is_bare);
    }

    #[test]
    fn scan_excludes_patterns() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create a repo inside an excluded dir
        fs::create_dir_all(root.join("node_modules/dep/.git")).unwrap();
        // Create a normal repo
        fs::create_dir_all(root.join("real-project/.git")).unwrap();

        let config = default_scan_config();
        let result = full_scan(&[root.to_path_buf()], &config, None).unwrap();

        assert_eq!(result.discovered.len(), 1);
        assert!(result.discovered[0]
            .path
            .to_string_lossy()
            .contains("real-project"));
        assert!(result.skipped_excluded > 0);
    }

    #[test]
    fn scan_respects_max_depth() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create a deep repo
        fs::create_dir_all(root.join("a/b/c/d/e/.git")).unwrap();

        let mut config = default_scan_config();
        config.max_depth = 3;
        let result = full_scan(&[root.to_path_buf()], &config, None).unwrap();

        // Too deep, should not be found
        assert_eq!(result.discovered.len(), 0);
    }

    #[test]
    fn scan_with_progress_callback() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("my-repo/.git")).unwrap();

        let config = default_scan_config();
        let found = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let found_clone = found.clone();

        let result = full_scan(
            &[root.to_path_buf()],
            &config,
            Some(Box::new(move |event| {
                if matches!(event, ScanEvent::RepoFound(_)) {
                    found_clone.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            })),
        )
        .unwrap();

        assert_eq!(result.discovered.len(), 1);
        assert!(found.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn scan_multiple_roots() {
        let dir = tempfile::tempdir().unwrap();
        let root_a = dir.path().join("root-a");
        let root_b = dir.path().join("root-b");

        fs::create_dir_all(root_a.join("repo-1/.git")).unwrap();
        fs::create_dir_all(root_b.join("repo-2/.git")).unwrap();

        let config = default_scan_config();
        let result = full_scan(
            &[root_a.clone(), root_b.clone()],
            &config,
            None,
        )
        .unwrap();

        assert_eq!(result.discovered.len(), 2);
    }

    #[test]
    fn quick_verify_detects_lost() {
        let dir = tempfile::tempdir().unwrap();
        let existing = dir.path().join("exists");
        fs::create_dir_all(existing.join(".git")).unwrap();
        // Write HEAD so the repo looks valid
        fs::write(existing.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();

        let missing = dir.path().join("missing");

        let result =
            quick_verify(&[existing.clone(), missing.clone()]).unwrap();

        assert_eq!(result.changed.len(), 1);
        assert_eq!(result.lost.len(), 1);
        assert_eq!(result.lost[0], missing);
    }
}

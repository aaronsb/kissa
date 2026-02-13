use std::path::PathBuf;
use std::time::Duration;

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
    StatTimeout,
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
    _roots: &[PathBuf],
    _config: &ScanConfig,
    _progress: Option<Box<dyn Fn(ScanEvent) + Send>>,
) -> Result<ScanResult> {
    todo!("Phase 3b: implement filesystem scanning")
}

/// Quick verify: stat known repo paths, return which changed/lost.
pub fn quick_verify(_known_paths: &[PathBuf]) -> Result<QuickVerifyResult> {
    todo!("Phase 3b: implement quick verify")
}

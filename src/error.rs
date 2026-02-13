use std::path::PathBuf;
use thiserror::Error;

use crate::core::permissions::DifficultyLevel;

#[derive(Error, Debug)]
pub enum KissaError {
    #[error("git error at {path}: {source}")]
    Git {
        path: PathBuf,
        source: git2::Error,
    },

    #[error("index error: {0}")]
    Index(#[from] rusqlite::Error),

    #[error("config error: {0}")]
    Config(String),

    #[error("scan error at {path}: {source}")]
    Scan {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("repo not found: {0}")]
    RepoNotFound(String),

    #[error("operation blocked: {operation} requires difficulty '{required:?}', current is '{current:?}'")]
    PermissionDenied {
        operation: String,
        required: DifficultyLevel,
        current: DifficultyLevel,
    },

    #[error("path not in scan roots: {0}")]
    OutsideScanRoots(PathBuf),
}

pub type Result<T> = std::result::Result<T, KissaError>;

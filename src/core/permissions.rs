use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::types::KissaConfig;
use crate::error::KissaError;

/// Difficulty levels control what operations kissa will perform (ADR-500).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DifficultyLevel {
    Readonly,
    Fetch,
    Commit,
    Force,
    Unsafe,
}

impl DifficultyLevel {
    pub fn display_name(&self, cat_mode: bool) -> &'static str {
        if cat_mode {
            match self {
                Self::Readonly => "napping",
                Self::Fetch => "purring",
                Self::Commit => "hunting",
                Self::Force => "zoomies",
                Self::Unsafe => "knocking-things-off-the-counter",
            }
        } else {
            match self {
                Self::Readonly => "readonly",
                Self::Fetch => "fetch",
                Self::Commit => "commit",
                Self::Force => "force",
                Self::Unsafe => "unsafe",
            }
        }
    }
}

/// An operation category that maps to a minimum difficulty level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationClass {
    Read,
    Fetch,
    Write,
    Force,
    Destructive,
}

impl OperationClass {
    /// Minimum difficulty level required for this operation class.
    pub fn required_level(&self) -> DifficultyLevel {
        match self {
            Self::Read => DifficultyLevel::Readonly,
            Self::Fetch => DifficultyLevel::Fetch,
            Self::Write => DifficultyLevel::Commit,
            Self::Force => DifficultyLevel::Force,
            Self::Destructive => DifficultyLevel::Unsafe,
        }
    }
}

/// Resolve the effective difficulty level for a repo path.
/// Checks per-path overrides first, then interface default (CLI vs MCP).
pub fn effective_difficulty(
    repo_path: &Path,
    config: &KissaConfig,
    is_mcp: bool,
) -> DifficultyLevel {
    let path_str = repo_path.to_string_lossy();

    // Check per-path overrides (glob patterns)
    for (pattern, level) in &config.overrides {
        if let Ok(glob) = glob::Pattern::new(pattern) {
            if glob.matches(&path_str) {
                return *level;
            }
        }
    }

    // Fall back to interface default
    if is_mcp {
        config.defaults.mcp.difficulty
    } else {
        config.defaults.difficulty
    }
}

/// Check whether an operation is permitted for a given repo.
pub fn check_permission(
    operation: OperationClass,
    repo_path: &Path,
    config: &KissaConfig,
    is_mcp: bool,
) -> Result<(), KissaError> {
    let current = effective_difficulty(repo_path, config, is_mcp);
    let required = operation.required_level();

    if current >= required {
        Ok(())
    } else {
        Err(KissaError::PermissionDenied {
            operation: format!("{:?}", operation),
            required,
            current,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn default_config() -> KissaConfig {
        KissaConfig::default()
    }

    #[test]
    fn difficulty_ordering() {
        assert!(DifficultyLevel::Readonly < DifficultyLevel::Commit);
        assert!(DifficultyLevel::Commit < DifficultyLevel::Force);
        assert!(DifficultyLevel::Force < DifficultyLevel::Unsafe);
    }

    #[test]
    fn operation_class_levels() {
        assert_eq!(OperationClass::Read.required_level(), DifficultyLevel::Readonly);
        assert_eq!(OperationClass::Write.required_level(), DifficultyLevel::Commit);
        assert_eq!(OperationClass::Destructive.required_level(), DifficultyLevel::Unsafe);
    }

    #[test]
    fn cat_mode_names() {
        assert_eq!(DifficultyLevel::Readonly.display_name(true), "napping");
        assert_eq!(DifficultyLevel::Unsafe.display_name(true), "knocking-things-off-the-counter");
        assert_eq!(DifficultyLevel::Commit.display_name(false), "commit");
    }

    #[test]
    fn cli_default_is_commit() {
        let config = default_config();
        let level = effective_difficulty(Path::new("/some/repo"), &config, false);
        assert_eq!(level, DifficultyLevel::Commit);
    }

    #[test]
    fn mcp_default_is_readonly() {
        let config = default_config();
        let level = effective_difficulty(Path::new("/some/repo"), &config, true);
        assert_eq!(level, DifficultyLevel::Readonly);
    }

    #[test]
    fn per_path_override() {
        let mut config = default_config();
        config
            .overrides
            .insert("/home/user/experiments/*".into(), DifficultyLevel::Force);

        let level = effective_difficulty(
            Path::new("/home/user/experiments/scratch"),
            &config,
            false,
        );
        assert_eq!(level, DifficultyLevel::Force);

        // Non-matching path falls back to default
        let level = effective_difficulty(Path::new("/home/user/work/api"), &config, false);
        assert_eq!(level, DifficultyLevel::Commit);
    }

    #[test]
    fn permission_check_allows_read_at_readonly() {
        let config = default_config();
        let result = check_permission(
            OperationClass::Read,
            Path::new("/some/repo"),
            &config,
            true, // MCP = readonly
        );
        assert!(result.is_ok());
    }

    #[test]
    fn permission_check_blocks_write_at_readonly() {
        let config = default_config();
        let result = check_permission(
            OperationClass::Write,
            Path::new("/some/repo"),
            &config,
            true, // MCP = readonly
        );
        assert!(result.is_err());
        if let Err(KissaError::PermissionDenied {
            required, current, ..
        }) = result
        {
            assert_eq!(required, DifficultyLevel::Commit);
            assert_eq!(current, DifficultyLevel::Readonly);
        }
    }

    #[test]
    fn permission_check_allows_write_at_commit() {
        let config = default_config();
        let result = check_permission(
            OperationClass::Write,
            Path::new("/some/repo"),
            &config,
            false, // CLI = commit
        );
        assert!(result.is_ok());
    }

    #[test]
    fn permission_check_blocks_force_at_commit() {
        let config = default_config();
        let result = check_permission(
            OperationClass::Force,
            Path::new("/some/repo"),
            &config,
            false, // CLI = commit
        );
        assert!(result.is_err());
    }
}

pub mod types;

use std::path::{Path, PathBuf};

use crate::error::{KissaError, Result};
use types::KissaConfig;

/// Load config from XDG path, merging defaults.
/// If no config file exists, returns sensible defaults (first-run experience).
pub fn load_config() -> Result<KissaConfig> {
    load_config_from(config_dir().join("config.toml"))
}

/// Load config from a specific path. Testable entry point.
pub fn load_config_from(path: impl AsRef<Path>) -> Result<KissaConfig> {
    let path = path.as_ref();
    match std::fs::read_to_string(path) {
        Ok(contents) => {
            let config: KissaConfig =
                toml::from_str(&contents).map_err(|e| KissaError::Config(e.to_string()))?;
            Ok(config)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // First run — no config file yet, use defaults
            Ok(KissaConfig::default())
        }
        Err(e) => Err(KissaError::Config(format!(
            "failed to read {}: {}",
            path.display(),
            e
        ))),
    }
}

/// Return XDG config dir (~/.config/kissa/)
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("kissa")
}

/// Return XDG data dir (~/.local/share/kissa/)
pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("kissa")
}

/// Return the index database path
pub fn index_path() -> PathBuf {
    data_dir().join("index.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::permissions::DifficultyLevel;
    use std::io::Write;

    #[test]
    fn missing_file_returns_defaults() {
        let config = load_config_from("/nonexistent/path/config.toml").unwrap();
        assert_eq!(config.defaults.difficulty, DifficultyLevel::Commit);
        assert_eq!(config.defaults.mcp.difficulty, DifficultyLevel::Readonly);
        assert!(!config.scan.roots.is_empty());
    }

    #[test]
    fn empty_file_returns_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "").unwrap();

        let config = load_config_from(&path).unwrap();
        assert_eq!(config.defaults.difficulty, DifficultyLevel::Commit);
        assert!(!config.scan.roots.is_empty());
    }

    #[test]
    fn partial_config_merges_with_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[defaults]
difficulty = "force"

[scan]
max_depth = 5
"#,
        )
        .unwrap();

        let config = load_config_from(&path).unwrap();
        assert_eq!(config.defaults.difficulty, DifficultyLevel::Force);
        assert_eq!(config.defaults.mcp.difficulty, DifficultyLevel::Readonly); // kept default
        assert_eq!(config.scan.max_depth, 5);
        assert!(!config.scan.exclude.is_empty()); // kept default exclusions
    }

    #[test]
    fn full_config_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[scan]
roots = ["/home/user/code", "/opt/repos"]
exclude = ["node_modules", "vendor"]
max_depth = 8
auto_verify_seconds = 600

[scan.boundaries]
cross_mounts = true
stat_timeout_ms = 1000

[identity]
usernames = ["aaronsb"]
community_orgs = ["rust-lang"]

[[identity.work_orgs]]
name = "initech"
platform = "github"
label = "Initech Corp"

[defaults]
difficulty = "commit"

[defaults.mcp]
difficulty = "fetch"

[display]
color = "always"
nerd_fonts = true
cat_mode = true

[overrides]
"/home/user/experiments/*" = "unsafe"
"/opt/repos/*" = "readonly"

[safety]
protected_branches = ["main", "develop"]
always_confirm_destructive = false
max_plan_size = 100
"#,
        )
        .unwrap();

        let config = load_config_from(&path).unwrap();
        assert_eq!(config.scan.roots.len(), 2);
        assert_eq!(config.scan.max_depth, 8);
        assert!(config.scan.boundaries.cross_mounts);
        assert_eq!(config.defaults.mcp.difficulty, DifficultyLevel::Fetch);
        assert!(config.display.cat_mode);
        assert_eq!(config.overrides.len(), 2);
        assert_eq!(
            config.overrides.get("/home/user/experiments/*"),
            Some(&DifficultyLevel::Unsafe)
        );
        assert_eq!(config.safety.max_plan_size, 100);
    }

    #[test]
    fn invalid_toml_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(&path, "this is not valid toml [[[").unwrap();

        let result = load_config_from(&path);
        assert!(result.is_err());
        match result.unwrap_err() {
            KissaError::Config(msg) => assert!(msg.contains("expected")),
            other => panic!("expected Config error, got: {:?}", other),
        }
    }

    #[test]
    fn invalid_difficulty_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        std::fs::write(
            &path,
            r#"
[defaults]
difficulty = "yolo"
"#,
        )
        .unwrap();

        let result = load_config_from(&path);
        assert!(result.is_err());
    }

    #[test]
    fn xdg_paths_are_sensible() {
        let cfg = config_dir();
        assert!(cfg.ends_with("kissa"));

        let data = data_dir();
        assert!(data.ends_with("kissa"));

        let idx = index_path();
        assert!(idx.ends_with("index.db"));
    }
}

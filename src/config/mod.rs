pub mod types;

use std::path::PathBuf;

use crate::error::Result;
use types::KissaConfig;

/// Load config from XDG path, merging defaults.
pub fn load_config() -> Result<KissaConfig> {
    // TODO: implement XDG config loading
    Ok(KissaConfig::default())
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

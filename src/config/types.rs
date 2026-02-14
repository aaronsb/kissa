use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::core::permissions::DifficultyLevel;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KissaConfig {
    pub scan: ScanConfig,
    pub identity: IdentityConfig,
    pub defaults: DefaultsConfig,
    pub display: DisplayConfig,
    #[serde(default)]
    pub overrides: HashMap<String, DifficultyLevel>,
    pub safety: SafetyConfig,
    #[serde(default)]
    pub classify: Vec<ClassifyRule>,
}

impl Default for KissaConfig {
    fn default() -> Self {
        Self {
            scan: ScanConfig::default(),
            identity: IdentityConfig::default(),
            defaults: DefaultsConfig::default(),
            display: DisplayConfig::default(),
            overrides: HashMap::new(),
            safety: SafetyConfig::default(),
            classify: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScanConfig {
    pub roots: Vec<PathBuf>,
    pub exclude: Vec<String>,
    pub max_depth: usize,
    pub auto_verify_seconds: u64,
    pub boundaries: BoundaryConfig,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            roots: vec![dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))],
            exclude: vec![
                "node_modules".into(),
                ".cargo/registry".into(),
                ".rustup".into(),
                "target/".into(),
                ".cache".into(),
                ".local/share/Trash".into(),
                ".local/share/flatpak".into(),
                ".local/share/Steam".into(),
                "snap/".into(),
                ".npm".into(),
                ".nvm/versions".into(),
                "__pycache__".into(),
                ".venv".into(),
            ],
            max_depth: 10,
            auto_verify_seconds: 300,
            boundaries: BoundaryConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BoundaryConfig {
    pub cross_mounts: bool,
    pub allow_mounts: Vec<PathBuf>,
    pub block_mounts: Vec<PathBuf>,
    pub stat_timeout_ms: u64,
}

impl Default for BoundaryConfig {
    fn default() -> Self {
        Self {
            cross_mounts: false,
            allow_mounts: Vec::new(),
            block_mounts: Vec::new(),
            stat_timeout_ms: 500,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IdentityConfig {
    pub usernames: Vec<String>,
    pub work_orgs: Vec<WorkOrg>,
    pub community_orgs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkOrg {
    pub name: String,
    pub platform: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    pub difficulty: DifficultyLevel,
    pub mcp: McpDefaultsConfig,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            difficulty: DifficultyLevel::Commit,
            mcp: McpDefaultsConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpDefaultsConfig {
    pub difficulty: DifficultyLevel,
}

impl Default for McpDefaultsConfig {
    fn default() -> Self {
        Self {
            difficulty: DifficultyLevel::Readonly,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayConfig {
    pub color: String,
    pub nerd_fonts: bool,
    pub cat_mode: bool,
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            color: "auto".into(),
            nerd_fonts: false,
            cat_mode: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub protected_branches: Vec<String>,
    pub always_confirm_destructive: bool,
    pub max_plan_size: usize,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            protected_branches: vec![
                "main".into(),
                "master".into(),
                "production".into(),
            ],
            always_confirm_destructive: true,
            max_plan_size: 50,
        }
    }
}

/// A classification rule from config `[[classify]]` (ADR-106).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifyRule {
    #[serde(rename = "match")]
    pub match_criteria: ClassifyMatch,
    #[serde(default)]
    pub set: ClassifySet,
    #[serde(default)]
    pub managed_by: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Match criteria for a classification rule. All fields are AND-combined.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassifyMatch {
    pub path: Option<String>,
    pub org: Option<String>,
    pub name: Option<String>,
    pub has_remote: Option<bool>,
    pub is_bare: Option<bool>,
}

/// Fields to set when a classification rule matches.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClassifySet {
    pub category: Option<String>,
    pub ownership: Option<String>,
    pub intention: Option<String>,
    pub state: Option<String>,
}

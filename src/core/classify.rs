use crate::config::types::{ClassifyRule, KissaConfig};
use super::git_ops::parse_remote_org;
use super::repo::{Ownership, Intention, Repo, RepoState};

/// Built-in heuristic patterns for tool-managed repos.
/// Each entry: (glob pattern, managed_by name).
const BUILTIN_HEURISTICS: &[(&str, &str)] = &[
    ("*/.local/share/nvim/lazy/*", "lazy.nvim"),
    ("*/.local/share/nvim/site/pack/*/start/*", "nvim-pack"),
    ("*/.vim/plugged/*", "vim-plug"),
    ("*/.local/share/SuperCollider/downloaded-quarks/*", "SuperCollider"),
    ("*/.cargo/git/checkouts/*", "cargo"),
    ("*/.local/share/FreeCAD/Mod/*", "FreeCAD"),
    ("*/.local/share/86Box/*", "86Box"),
];

/// Apply classification rules and built-in heuristics to a repo.
///
/// Evaluation order:
/// 1. Config `[[classify]]` rules in order (first match per field wins)
/// 2. Built-in heuristics as lowest-priority fallback
///
/// Tags are always appended, never first-match gated.
pub fn classify_repo(repo: &mut Repo, config: &KissaConfig) {
    // Phase 1: config rules
    for rule in &config.classify {
        if rule_matches(rule, repo) {
            apply_rule(rule, repo);
        }
    }

    // Phase 2: built-in heuristics (only fill None fields)
    apply_heuristics(repo);
}

/// Check if all match criteria in a rule are satisfied (AND-combined).
fn rule_matches(rule: &ClassifyRule, repo: &Repo) -> bool {
    let m = &rule.match_criteria;

    if let Some(ref pattern) = m.path {
        let expanded = expand_tilde(pattern);
        let pat = glob::Pattern::new(&expanded);
        match pat {
            Ok(p) => {
                if !p.matches_path(&repo.path) {
                    return false;
                }
            }
            Err(_) => return false,
        }
    }

    if let Some(ref org_filter) = m.org {
        let matches_org = repo.remotes.iter().any(|remote| {
            parse_remote_org(&remote.url)
                .is_some_and(|info| info.org.eq_ignore_ascii_case(org_filter))
        });
        if !matches_org {
            return false;
        }
    }

    if let Some(ref name_pattern) = m.name {
        let pat = glob::Pattern::new(name_pattern);
        match pat {
            Ok(p) => {
                if !p.matches(&repo.name) {
                    return false;
                }
            }
            Err(_) => return false,
        }
    }

    if let Some(has_remote) = m.has_remote {
        if repo.remotes.is_empty() == has_remote {
            return false;
        }
    }

    true
}

/// Apply a matching rule's fields to a repo.
/// First-match-per-field: only sets fields that are currently None.
/// Tags are always appended.
fn apply_rule(rule: &ClassifyRule, repo: &mut Repo) {
    if repo.managed_by.is_none() {
        if let Some(ref mb) = rule.managed_by {
            repo.managed_by = Some(mb.clone());
        }
    }

    if let Some(ref ownership_str) = rule.set.ownership {
        if repo.ownership.is_none() {
            repo.ownership = parse_ownership(ownership_str);
        }
    }

    if let Some(ref intention_str) = rule.set.intention {
        if repo.intention.is_none() {
            repo.intention = serde_plain::from_str(intention_str).ok();
        }
    }

    if let Some(ref category_str) = rule.set.category {
        if repo.category.is_none() {
            repo.category = serde_plain::from_str(category_str).ok();
        }
    }

    if let Some(ref state_str) = rule.set.state {
        if let Ok(state) = serde_plain::from_str::<RepoState>(state_str) {
            repo.state = state;
        }
    }

    // Tags: always appended, deduplicated
    for tag in &rule.tags {
        if !repo.tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
            repo.tags.push(tag.clone());
        }
    }
}

/// Apply built-in heuristics as lowest-priority fallback.
fn apply_heuristics(repo: &mut Repo) {
    if repo.managed_by.is_some() {
        return;
    }

    let path_str = repo.path.to_string_lossy();
    for &(pattern, manager) in BUILTIN_HEURISTICS {
        let expanded = expand_tilde(pattern);
        if let Ok(p) = glob::Pattern::new(&expanded) {
            if p.matches(&path_str) {
                repo.managed_by = Some(manager.to_string());
                if repo.ownership.is_none() {
                    repo.ownership = Some(Ownership::ThirdParty);
                }
                if repo.intention.is_none() {
                    repo.intention = Some(Intention::Dependency);
                }
                return;
            }
        }
    }
}

/// Parse an ownership string like "personal", "work:acme", "third-party".
fn parse_ownership(s: &str) -> Option<Ownership> {
    if let Some(label) = s.strip_prefix("work:") {
        Some(Ownership::Work {
            label: label.to_string(),
        })
    } else {
        match s.to_lowercase().as_str() {
            "personal" => Some(Ownership::Personal),
            "community" => Some(Ownership::Community),
            "third-party" | "thirdparty" => Some(Ownership::ThirdParty),
            "local" => Some(Ownership::Local),
            _ => None,
        }
    }
}

/// Expand `~` prefix to home directory.
fn expand_tilde(pattern: &str) -> String {
    if let Some(rest) = pattern.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), rest);
        }
    }
    pattern.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;
    use crate::core::repo::*;
    use chrono::Utc;
    use std::path::PathBuf;

    fn make_repo(name: &str, path: &str) -> Repo {
        Repo {
            id: 0,
            name: name.to_string(),
            path: PathBuf::from(path),
            state: RepoState::Active,
            remotes: vec![Remote {
                name: "origin".into(),
                url: "git@github.com:someuser/somerepo.git".into(),
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
            category: None,
            ownership: None,
            intention: None,
            managed_by: None,
            tags: vec![],
            project: None,
            role: None,
        }
    }

    fn empty_config() -> KissaConfig {
        KissaConfig::default()
    }

    #[test]
    fn path_match_sets_managed_by() {
        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/home/user/.config/nvim/lazy/*".into()),
                ..Default::default()
            },
            set: ClassifySet::default(),
            managed_by: Some("lazy.nvim".into()),
            tags: vec![],
        });

        let mut repo = make_repo("plenary.nvim", "/home/user/.config/nvim/lazy/plenary.nvim");
        classify_repo(&mut repo, &config);
        assert_eq!(repo.managed_by, Some("lazy.nvim".into()));
    }

    #[test]
    fn org_match_sets_ownership() {
        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                org: Some("rust-lang".into()),
                ..Default::default()
            },
            set: ClassifySet {
                ownership: Some("community".into()),
                intention: Some("reference".into()),
                ..Default::default()
            },
            managed_by: None,
            tags: vec![],
        });

        let mut repo = make_repo("rust", "/home/user/code/rust");
        repo.remotes = vec![Remote {
            name: "origin".into(),
            url: "git@github.com:rust-lang/rust.git".into(),
            push_url: None,
        }];
        classify_repo(&mut repo, &config);
        assert_eq!(repo.ownership, Some(Ownership::Community));
        assert_eq!(repo.intention, Some(Intention::Reference));
    }

    #[test]
    fn first_match_per_field_wins() {
        let mut config = empty_config();
        // Rule 1: sets ownership
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/code/*".into()),
                ..Default::default()
            },
            set: ClassifySet {
                ownership: Some("personal".into()),
                ..Default::default()
            },
            managed_by: None,
            tags: vec![],
        });
        // Rule 2: also tries to set ownership, but should be ignored
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/code/*".into()),
                ..Default::default()
            },
            set: ClassifySet {
                ownership: Some("community".into()),
                intention: Some("developing".into()),
                ..Default::default()
            },
            managed_by: None,
            tags: vec![],
        });

        let mut repo = make_repo("myrepo", "/code/myrepo");
        classify_repo(&mut repo, &config);
        // First rule wins for ownership
        assert_eq!(repo.ownership, Some(Ownership::Personal));
        // Second rule fills intention (was None)
        assert_eq!(repo.intention, Some(Intention::Developing));
    }

    #[test]
    fn tags_always_appended() {
        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/code/*".into()),
                ..Default::default()
            },
            set: ClassifySet::default(),
            managed_by: None,
            tags: vec!["rust".into()],
        });
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/code/*".into()),
                ..Default::default()
            },
            set: ClassifySet::default(),
            managed_by: None,
            tags: vec!["backend".into()],
        });

        let mut repo = make_repo("myrepo", "/code/myrepo");
        classify_repo(&mut repo, &config);
        assert!(repo.tags.contains(&"rust".to_string()));
        assert!(repo.tags.contains(&"backend".to_string()));
    }

    #[test]
    fn tags_deduplicated() {
        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/code/*".into()),
                ..Default::default()
            },
            set: ClassifySet::default(),
            managed_by: None,
            tags: vec!["rust".into()],
        });

        let mut repo = make_repo("myrepo", "/code/myrepo");
        repo.tags = vec!["rust".into()];
        classify_repo(&mut repo, &config);
        assert_eq!(repo.tags.len(), 1);
    }

    #[test]
    fn heuristic_matches_lazy_nvim() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/testuser"));
        let path = home.join(".local/share/nvim/lazy/telescope.nvim");
        let mut repo = make_repo("telescope.nvim", path.to_str().unwrap());
        classify_repo(&mut repo, &empty_config());

        assert_eq!(repo.managed_by, Some("lazy.nvim".into()));
        assert_eq!(repo.ownership, Some(Ownership::ThirdParty));
        assert_eq!(repo.intention, Some(Intention::Dependency));
    }

    #[test]
    fn heuristic_matches_cargo_checkouts() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/testuser"));
        let path = home.join(".cargo/git/checkouts/serde-abc123");
        let mut repo = make_repo("serde-abc123", path.to_str().unwrap());
        classify_repo(&mut repo, &empty_config());

        assert_eq!(repo.managed_by, Some("cargo".into()));
    }

    #[test]
    fn config_rule_overrides_heuristic() {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/testuser"));
        let path = home.join(".local/share/nvim/lazy/my-plugin");

        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some(format!("{}/.local/share/nvim/lazy/*", home.display())),
                ..Default::default()
            },
            set: ClassifySet {
                ownership: Some("personal".into()),
                ..Default::default()
            },
            managed_by: Some("custom-manager".into()),
            tags: vec!["nvim".into()],
        });

        let mut repo = make_repo("my-plugin", path.to_str().unwrap());
        classify_repo(&mut repo, &config);

        // Config rule wins over heuristic
        assert_eq!(repo.managed_by, Some("custom-manager".into()));
        assert_eq!(repo.ownership, Some(Ownership::Personal));
        assert!(repo.tags.contains(&"nvim".to_string()));
    }

    #[test]
    fn no_match_leaves_fields_none() {
        let mut repo = make_repo("random-repo", "/tmp/random-repo");
        classify_repo(&mut repo, &empty_config());

        assert!(repo.managed_by.is_none());
        assert!(repo.ownership.is_none());
        assert!(repo.intention.is_none());
        assert!(repo.category.is_none());
    }

    #[test]
    fn and_combined_criteria() {
        let mut config = empty_config();
        config.classify.push(ClassifyRule {
            match_criteria: ClassifyMatch {
                path: Some("/work/*".into()),
                org: Some("acme-corp".into()),
                ..Default::default()
            },
            set: ClassifySet {
                ownership: Some("work:acme".into()),
                ..Default::default()
            },
            managed_by: None,
            tags: vec![],
        });

        // Path matches but org doesn't
        let mut repo = make_repo("myrepo", "/work/myrepo");
        classify_repo(&mut repo, &config);
        assert!(repo.ownership.is_none());

        // Both match
        let mut repo = make_repo("myrepo", "/work/myrepo");
        repo.remotes = vec![Remote {
            name: "origin".into(),
            url: "git@github.com:acme-corp/myrepo.git".into(),
            push_url: None,
        }];
        classify_repo(&mut repo, &config);
        assert_eq!(
            repo.ownership,
            Some(Ownership::Work {
                label: "acme".into()
            })
        );
    }

    #[test]
    fn work_ownership_parsing() {
        assert_eq!(
            parse_ownership("work:acme"),
            Some(Ownership::Work {
                label: "acme".into()
            })
        );
        assert_eq!(parse_ownership("personal"), Some(Ownership::Personal));
        assert_eq!(parse_ownership("third-party"), Some(Ownership::ThirdParty));
        assert_eq!(parse_ownership("thirdparty"), Some(Ownership::ThirdParty));
        assert_eq!(parse_ownership("community"), Some(Ownership::Community));
        assert_eq!(parse_ownership("local"), Some(Ownership::Local));
        assert_eq!(parse_ownership("nonsense"), None);
    }
}

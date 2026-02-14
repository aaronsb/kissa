use std::path::{Path, PathBuf};
use std::sync::Arc;

use rmcp::handler::server::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerCapabilities, ServerInfo};
use rmcp::{tool, tool_handler, tool_router, ErrorData as McpError};
use rmcp::schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use kissa::config;
use kissa::core::classify;
use kissa::core::filter::RepoFilter;
use kissa::core::git_ops;
use kissa::core::index::Index;
use kissa::core::repo::{Freshness, Repo, RepoState};
use kissa::core::scanner;

use super::format;

#[derive(Clone)]
pub struct KissaServer {
    index: Arc<Mutex<Index>>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
pub struct ListReposParams {
    /// Show only dirty repos
    #[serde(default)]
    pub dirty: Option<bool>,
    /// Show only repos with unpushed commits
    #[serde(default)]
    pub unpushed: Option<bool>,
    /// Show only orphan repos (no remote)
    #[serde(default)]
    pub orphan: Option<bool>,
    /// Filter by remote org/owner
    #[serde(default)]
    pub org: Option<String>,
    /// Filter by freshness tier (active, recent, stale, dormant, ancient)
    #[serde(default)]
    pub freshness: Option<String>,
    /// Filter by name (substring match)
    #[serde(default)]
    pub name: Option<String>,
    /// Filter by path prefix
    #[serde(default)]
    pub path_prefix: Option<String>,
    /// Filter by ownership
    #[serde(default)]
    pub ownership: Option<String>,
    /// Filter by intention
    #[serde(default)]
    pub intention: Option<String>,
    /// Filter by category
    #[serde(default)]
    pub category: Option<String>,
    /// Filter by tags (all must match)
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    /// Show only managed repos (true), only unmanaged (false), or all (omit)
    #[serde(default)]
    pub managed: Option<bool>,
    /// Filter by managing tool name (e.g., "lazy.nvim")
    #[serde(default)]
    pub managed_by: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
pub struct RepoStatusParams {
    /// Repo name or absolute path
    pub repo: String,
}

#[derive(Deserialize, JsonSchema)]
pub struct ScanParams {
    /// Override scan roots (paths)
    #[serde(default)]
    pub roots: Option<Vec<String>>,
}

#[derive(Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Search query (matches name, path, tags)
    pub query: String,
}

#[tool_router]
impl KissaServer {
    pub fn new(index: Arc<Mutex<Index>>) -> Self {
        Self {
            index,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        name = "list_repos",
        description = "List catalogued git repositories with optional filters. Returns terse text with state tags.",
        annotations(read_only_hint = true)
    )]
    async fn list_repos(
        &self,
        params: Parameters<ListReposParams>,
    ) -> Result<CallToolResult, McpError> {
        let p = params.0;
        let freshness = p
            .freshness
            .as_deref()
            .and_then(|s| serde_plain::from_str::<Freshness>(s).ok());

        let filter = RepoFilter {
            dirty: p.dirty,
            unpushed: p.unpushed,
            orphan: p.orphan,
            org: p.org,
            freshness,
            ownership: p.ownership,
            intention: p.intention,
            category: p.category,
            tags: p.tags,
            path_prefix: p.path_prefix,
            has_remote: None,
            name_contains: p.name,
            state: None,
            managed_by: p.managed_by,
            show_managed: p.managed,
        };

        let index = self.index.lock().await;
        let repos = index.list_repos(&filter).map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            format::format_repo_list(&repos),
        )]))
    }

    #[tool(
        name = "repo_status",
        description = "Show detailed status for a single repository by name or path.",
        annotations(read_only_hint = true)
    )]
    async fn repo_status(
        &self,
        params: Parameters<RepoStatusParams>,
    ) -> Result<CallToolResult, McpError> {
        let index = self.index.lock().await;
        let repo = if Path::new(&params.0.repo).is_absolute() {
            index
                .get_repo_by_path(Path::new(&params.0.repo))
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        } else {
            index
                .get_repo_by_name(&params.0.repo)
                .map_err(|e| McpError::internal_error(e.to_string(), None))?
        };

        let Some(repo) = repo else {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "[error] repo not found: {}\n→ next: list_repos | search",
                params.0.repo
            ))]));
        };

        Ok(CallToolResult::success(vec![Content::text(
            format::format_repo_status(&repo),
        )]))
    }

    #[tool(
        name = "freshness",
        description = "Show freshness tier overview of all catalogued repos.",
        annotations(read_only_hint = true)
    )]
    async fn freshness(&self) -> Result<CallToolResult, McpError> {
        let index = self.index.lock().await;
        let summary = index
            .freshness_summary()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            format::format_freshness(&summary),
        )]))
    }

    #[tool(
        name = "scan",
        description = "Scan filesystem for git repositories and update the index.",
        annotations(read_only_hint = false, destructive_hint = false)
    )]
    async fn scan(
        &self,
        params: Parameters<ScanParams>,
    ) -> Result<CallToolResult, McpError> {
        let cfg = config::load_config().map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        let roots: Vec<PathBuf> = if let Some(ref r) = params.0.roots {
            r.iter().map(PathBuf::from).collect()
        } else {
            cfg.scan.roots.clone()
        };

        let result = scanner::full_scan(&roots, &cfg.scan, None).map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        let mut upserted = 0;
        let index = self.index.lock().await;

        for discovered in &result.discovered {
            if let Ok(vitals) = git_ops::extract_vitals(&discovered.path) {
                let mut repo = Repo {
                    id: 0,
                    name: vitals.name,
                    path: discovered.path.clone(),
                    state: RepoState::Active,
                    remotes: vitals.remotes,
                    default_branch: vitals.default_branch,
                    current_branch: vitals.current_branch,
                    branch_count: vitals.branch_count,
                    stale_branch_count: vitals.stale_branch_count,
                    dirty: vitals.dirty,
                    staged: vitals.staged,
                    untracked: vitals.untracked,
                    ahead: vitals.ahead,
                    behind: vitals.behind,
                    last_commit: vitals.last_commit,
                    last_verified: Some(chrono::Utc::now()),
                    first_seen: chrono::Utc::now(),
                    freshness: Freshness::from_commit_time(vitals.last_commit),
                    category: None,
                    ownership: None,
                    intention: None,
                    managed_by: None,
                    tags: vec![],
                    project: None,
                    role: None,
                };
                classify::classify_repo(&mut repo, &cfg);
                if index.upsert_repo(&repo).is_ok() {
                    upserted += 1;
                }
            }
        }

        let _ = index.record_scan(&roots, upserted);

        Ok(CallToolResult::success(vec![Content::text(
            format::format_scan_complete(
                result.discovered.len(),
                upserted,
                result.duration.as_secs_f64(),
            ),
        )]))
    }

    #[tool(
        name = "search",
        description = "Search repos by name (fuzzy substring match).",
        annotations(read_only_hint = true)
    )]
    async fn search(
        &self,
        params: Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let index = self.index.lock().await;

        let filter = RepoFilter {
            name_contains: Some(params.0.query.clone()),
            ..Default::default()
        };

        let repos = index.list_repos(&filter).map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            format::format_repo_list(&repos),
        )]))
    }

    #[tool(
        name = "get_config",
        description = "Read the current kissa configuration.",
        annotations(read_only_hint = true)
    )]
    async fn get_config(&self) -> Result<CallToolResult, McpError> {
        let cfg = config::load_config().map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        let json = serde_json::to_string_pretty(&cfg).map_err(|e| {
            McpError::internal_error(e.to_string(), None)
        })?;

        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    #[tool(
        name = "summary",
        description = "Get high-level index statistics: repo count, dirty/unpushed/orphan counts, freshness breakdown.",
        annotations(read_only_hint = true)
    )]
    async fn summary(&self) -> Result<CallToolResult, McpError> {
        let index = self.index.lock().await;
        let summary = index
            .summary()
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        Ok(CallToolResult::success(vec![Content::text(
            format::format_summary(&summary),
        )]))
    }
}

#[tool_handler]
impl rmcp::ServerHandler for KissaServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "kissa: git repo catalogue and topology manager. \
                 Use scan to discover repos, list_repos to query, \
                 repo_status for details, freshness for overview."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

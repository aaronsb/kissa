pub mod format;
pub mod resources;
pub mod tools;

use std::sync::Arc;

use rmcp::ServiceExt;
use tokio::sync::Mutex;

use kissa::config;
use kissa::core::index::Index;
use tools::KissaServer;

/// Start the MCP server over stdio. Blocks until the connection closes.
pub fn serve_stdio() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let index = Index::open(&config::index_path())?;
        let index = Arc::new(Mutex::new(index));

        let server = KissaServer::new(index);
        let service = server.serve(rmcp::transport::stdio()).await?;
        service.waiting().await?;

        Ok(())
    })
}

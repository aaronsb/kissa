pub mod format;
pub mod resources;
pub mod tools;

/// Start the MCP server over stdio. Blocks until the connection closes.
pub fn serve_stdio() -> anyhow::Result<()> {
    todo!("Phase 5: implement MCP server with rmcp")
}

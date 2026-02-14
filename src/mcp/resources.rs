// MCP resource definitions
// Resources are served via the ServerHandler trait methods.
// Currently resources are implemented as tools (summary, get_config)
// since rmcp resources require URI-based access which is less
// ergonomic for LLM tool use. The summary and get_config tools
// provide the same data in a more accessible format.

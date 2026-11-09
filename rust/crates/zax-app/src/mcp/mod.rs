//! The MCP server of the Lua profile editor (EDITOR-1): Claude Code drives the running window —
//! a movie, its frames, screenshots, the RAM, the scripts' reload and their log.
//!
//! Off by default: started by `--mcp[=port]` only (default port [`DEFAULT_PORT`]), listening on
//! `127.0.0.1` only. When it is off nothing of it runs; when it is on, the main loop pays one
//! `Option` test and a channel poll per displayed frame. To add it to Claude Code:
//! `claude mcp add --transport http zaxnes http://127.0.0.1:<port>/mcp`.

pub mod server;
pub mod tools;

pub use tools::McpBridge;

/// The port listened on when `--mcp` names none.
pub const DEFAULT_PORT: u16 = 8765;

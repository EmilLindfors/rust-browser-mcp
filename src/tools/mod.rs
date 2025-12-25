mod automation;
mod driver_management;
mod performance;
mod recipes;

pub use automation::*;
pub use driver_management::*;
pub use performance::*;
pub use recipes::*;

use once_cell::sync::Lazy;
use rmcp::model::{Content, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Stdio,  // Client controls driver lifecycle
    Http,   // Server manages driver lifecycle automatically
}

/// Cached tool definitions for stdio mode (includes driver management tools)
static STDIO_TOOLS: Lazy<Vec<Tool>> = Lazy::new(|| {
    let mut tools = Vec::with_capacity(45);
    tools.extend(AutomationTools::get_tools());
    tools.extend(PerformanceTools::get_tools());
    tools.extend(RecipeTools::get_tools());
    tools.extend(DriverManagementTools::get_tools());
    tools
});

/// Cached tool definitions for http mode (excludes driver management tools)
static HTTP_TOOLS: Lazy<Vec<Tool>> = Lazy::new(|| {
    let mut tools = Vec::with_capacity(38);
    tools.extend(AutomationTools::get_tools());
    tools.extend(PerformanceTools::get_tools());
    tools.extend(RecipeTools::get_tools());
    tools
});

pub struct ToolDefinitions;

impl ToolDefinitions {
    /// Returns all tools for the default (stdio) mode
    pub fn list_all() -> Vec<Tool> {
        Self::list_for_mode(ServerMode::Stdio)
    }

    /// Returns a clone of the cached tool list for the given mode
    /// Tool definitions are computed once and cached for the lifetime of the program
    pub fn list_for_mode(mode: ServerMode) -> Vec<Tool> {
        match mode {
            ServerMode::Stdio => STDIO_TOOLS.clone(),
            ServerMode::Http => HTTP_TOOLS.clone(),
        }
    }
}

pub fn success_response(message: String) -> rmcp::model::CallToolResult {
    rmcp::model::CallToolResult {
        content: vec![Content::text(message)],
        is_error: Some(false),
    }
}

pub fn error_response(message: String) -> rmcp::model::CallToolResult {
    rmcp::model::CallToolResult {
        content: vec![Content::text(message)],
        is_error: Some(true),
    }
}
mod automation;
mod driver_management;
mod performance;
mod recipes;

pub use automation::*;
pub use driver_management::*;
pub use performance::*;
pub use recipes::*;

use rmcp::model::{Content, Tool};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerMode {
    Stdio,  // Client controls driver lifecycle
    Http,   // Server manages driver lifecycle automatically
}

pub struct ToolDefinitions;

impl ToolDefinitions {
    pub fn list_all() -> Vec<Tool> {
        // Default to stdio mode for backward compatibility
        Self::list_for_mode(ServerMode::Stdio)
    }

    pub fn list_for_mode(mode: ServerMode) -> Vec<Tool> {
        let mut tools = vec![];
        
        // Add core automation tools
        tools.extend(AutomationTools::get_tools());
        
        // Add performance tools
        tools.extend(PerformanceTools::get_tools());
        
        // Add recipe tools
        tools.extend(RecipeTools::get_tools());
        
        // Add driver lifecycle tools only in stdio mode
        if mode == ServerMode::Stdio {
            tools.extend(DriverManagementTools::get_tools());
        }

        tools
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
//! Handler modules for WebDriver MCP server
//!
//! Each module contains handlers for a specific category of tools:
//! - `drivers`: WebDriver lifecycle management (start, stop, health checks)
//! - `navigation`: Browser navigation (navigate, back, forward, refresh)
//! - `elements`: Element interaction (click, send_keys, find, hover, scroll)
//! - `page`: Page content operations (title, text, screenshot, source)
//! - `performance`: Performance monitoring (console logs, metrics, memory)
//! - `recipes`: Recipe management (create, execute, list, delete)

pub mod drivers;
pub mod navigation;
pub mod elements;
pub mod page;
pub mod performance;
pub mod recipes;

use serde_json::{Map, Value};

/// Common utility to extract session_id from arguments
pub fn extract_session_id(arguments: &Option<Map<String, Value>>) -> Option<String> {
    arguments
        .as_ref()
        .and_then(|args| args.get("session_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Common utility to extract optional wait_timeout from arguments
pub fn extract_wait_timeout(arguments: &Option<Map<String, Value>>) -> Option<f64> {
    arguments
        .as_ref()
        .and_then(|args| args.get("wait_timeout"))
        .and_then(|v| v.as_f64())
}


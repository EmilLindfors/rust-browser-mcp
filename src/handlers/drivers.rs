//! Driver lifecycle management handlers
//!
//! Handles WebDriver process lifecycle operations:
//! - Starting and stopping drivers
//! - Health checks and monitoring
//! - Orphaned process cleanup

use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    driver::DriverType,
    tools::{error_response, success_response},
};

/// Get currently healthy WebDriver endpoints
pub async fn handle_get_healthy_endpoints(
    client_manager: &ClientManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_manager = client_manager.get_driver_manager();
    let healthy_endpoints = driver_manager.get_healthy_endpoints().await;

    let mut result = serde_json::Map::new();
    for (driver_type, endpoint) in healthy_endpoints {
        result.insert(driver_type.browser_name().to_lowercase(), Value::String(endpoint));
    }

    Ok(success_response(format!(
        "Healthy endpoints:\n{}",
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
    )))
}

/// Refresh health status of all WebDriver endpoints
pub async fn handle_refresh_driver_health(
    client_manager: &ClientManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_manager = client_manager.get_driver_manager();

    match driver_manager.refresh_driver_health().await {
        Ok(_) => {
            let healthy_endpoints = driver_manager.get_healthy_endpoints().await;
            Ok(success_response(format!(
                "Health check completed. {} healthy endpoints found",
                healthy_endpoints.len()
            )))
        }
        Err(e) => Ok(error_response(format!("Health check failed: {e}"))),
    }
}

/// List all managed WebDriver processes
pub async fn handle_list_managed_drivers(
    client_manager: &ClientManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_manager = client_manager.get_driver_manager();
    let managed_processes = driver_manager.get_managed_processes_status().await;

    if managed_processes.is_empty() {
        Ok(success_response("No managed WebDriver processes running".to_string()))
    } else {
        use std::fmt::Write;
        let mut result = String::from("Managed WebDriver processes:\n");
        for (driver_type, pid, port) in managed_processes {
            let _ = writeln!(&mut result, "  {} - PID: {}, Port: {}", driver_type.browser_name(), pid, port);
        }
        Ok(success_response(result))
    }
}

/// Start a WebDriver process manually
pub async fn handle_start_driver(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_type_str = arguments
        .as_ref()
        .and_then(|args| args.get("driver_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("driver_type parameter required", None))?;

    let driver_type = DriverType::from_string(driver_type_str)
        .ok_or_else(|| McpError::invalid_params("Invalid driver_type. Use: chrome, firefox, or edge", None))?;

    let driver_manager = client_manager.get_driver_manager();

    match driver_manager.start_driver_manually(driver_type.clone()).await {
        Ok(endpoint) => {
            // Additional health refresh to ensure driver is available for recipe execution
            let _ = driver_manager.refresh_driver_health().await;
            Ok(success_response(format!(
                "Successfully started {} WebDriver at {}",
                driver_type.browser_name(),
                endpoint
            )))
        },
        Err(e) => Ok(error_response(format!(
            "Failed to start {} WebDriver: {}",
            driver_type.browser_name(),
            e
        ))),
    }
}

/// Stop a specific WebDriver process by type
pub async fn handle_stop_driver(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_type_str = arguments
        .as_ref()
        .and_then(|args| args.get("driver_type"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("driver_type parameter required", None))?;

    let driver_type = DriverType::from_string(driver_type_str)
        .ok_or_else(|| McpError::invalid_params("Invalid driver_type. Use: chrome, firefox, or edge", None))?;

    let driver_manager = client_manager.get_driver_manager();

    match driver_manager.stop_driver_by_type(&driver_type).await {
        Ok(_) => Ok(success_response(format!(
            "Successfully stopped {} WebDriver",
            driver_type.browser_name()
        ))),
        Err(e) => Ok(error_response(format!(
            "Failed to stop {} WebDriver: {}",
            driver_type.browser_name(),
            e
        ))),
    }
}

/// Stop all running WebDriver processes
pub async fn handle_stop_all_drivers(
    client_manager: &ClientManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let driver_manager = client_manager.get_driver_manager();

    match driver_manager.stop_all_drivers().await {
        Ok(_) => Ok(success_response("Successfully stopped all WebDriver processes".to_string())),
        Err(e) => Ok(error_response(format!("Failed to stop all drivers: {e}"))),
    }
}

/// Force cleanup orphaned browser and WebDriver processes
pub async fn handle_force_cleanup_orphaned_processes(
    client_manager: &ClientManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    match client_manager.force_cleanup_orphaned_processes_public().await {
        Ok(_) => Ok(success_response("Successfully force cleaned up all orphaned browser and WebDriver processes".to_string())),
        Err(e) => Ok(error_response(format!("Failed to force cleanup orphaned processes: {e}"))),
    }
}

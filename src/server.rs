use base64::{Engine as _, engine::general_purpose};
use fantoccini::Locator;
use rmcp::{ErrorData as McpError, ServerHandler, model::*};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    config::Config,
    recipes::{RecipeManager, RecipeTemplate, RecipeExecutor, ExecutionContext},
    tools::{ToolDefinitions, ServerMode, error_response, success_response},
};

#[derive(Clone)]
pub struct WebDriverServer {
    client_manager: ClientManager,
    recipe_manager: RecipeManager,
    mode: ServerMode,
}

impl WebDriverServer {
    pub fn new() -> crate::error::Result<Self> {
        let config = Config::from_env();
        Ok(Self {
            client_manager: ClientManager::new(config)?,
            recipe_manager: RecipeManager::new(None),
            mode: ServerMode::Stdio,
        })
    }

    pub fn with_config(config: Config) -> crate::error::Result<Self> {
        Ok(Self {
            client_manager: ClientManager::new(config)?,
            recipe_manager: RecipeManager::new(None),
            mode: ServerMode::Stdio,
        })
    }

    pub fn with_config_and_mode(config: Config, mode: ServerMode) -> crate::error::Result<Self> {
        Ok(Self {
            client_manager: ClientManager::new(config)?,
            recipe_manager: RecipeManager::new(None),
            mode,
        })
    }

    /// Get the client manager
    pub fn get_client_manager(&self) -> &ClientManager {
        &self.client_manager
    }

    /// Start drivers proactively (for HTTP mode)
    pub async fn ensure_drivers_started(&mut self) -> crate::error::Result<()> {
        let config = self.client_manager.get_config();
        
        if config.auto_start_driver && !config.concurrent_drivers.is_empty() {
            tracing::debug!("Starting concurrent webdrivers: {:?}", config.concurrent_drivers);
            
            let driver_manager = self.client_manager.get_driver_manager();
            let drivers = config.concurrent_drivers.clone();
            let timeout = std::time::Duration::from_millis(config.driver_startup_timeout_ms);
            
            match driver_manager.start_concurrent_drivers(&drivers, timeout).await {
                Ok(started_drivers) => {
                    let requested_count = drivers.len();
                    let started_count = started_drivers.len();
                    
                    if started_count == 0 {
                        return Err(crate::error::WebDriverError::Session(
                            format!("Failed to start any WebDriver processes. Requested: {drivers:?}")
                        ));
                    } else if started_count < requested_count {
                        // Some drivers failed - show warnings for what failed and info for what succeeded
                        let started_types: std::collections::HashSet<_> = started_drivers.iter().map(|(dt, _)| dt.browser_name()).collect();
                        for driver_name in &drivers {
                            if let Some(driver_type) = crate::driver::DriverType::from_string(driver_name) {
                                if !started_types.contains(driver_type.browser_name()) {
                                    tracing::warn!("Failed to start {} WebDriver - it may not be installed or accessible", driver_type.browser_name());
                                }
                            }
                        }
                        
                        tracing::debug!("Successfully started {}/{} WebDrivers:", started_count, requested_count);
                        for (driver_type, endpoint) in &started_drivers {
                            tracing::debug!("  {} → {}", driver_type.browser_name(), endpoint);
                        }
                    } else {
                        // All drivers started successfully
                        tracing::debug!("Successfully started all {} WebDrivers:", started_count);
                        for (driver_type, endpoint) in &started_drivers {
                            tracing::debug!("  {} → {}", driver_type.browser_name(), endpoint);
                        }
                    }
                    
                    // Start periodic health checks every 30 seconds
                    let health_check_interval = std::time::Duration::from_secs(30);
                    let _health_check_handle = driver_manager.start_periodic_health_checks(health_check_interval);
                    tracing::debug!("Started periodic health checks (every {:?})", health_check_interval);
                }
                Err(e) => {
                    tracing::warn!("Failed to start some WebDriver processes: {}. Server will continue with reactive startup.", e);
                }
            }
        }
        
        Ok(())
    }

    // Driver lifecycle tool handlers (stdio mode only)
    
    async fn handle_get_healthy_endpoints(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_manager = self.client_manager.get_driver_manager();
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

    async fn handle_refresh_driver_health(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_manager = self.client_manager.get_driver_manager();

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

    async fn handle_list_managed_drivers(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_manager = self.client_manager.get_driver_manager();
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

    async fn handle_start_driver(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_type_str = arguments
            .as_ref()
            .and_then(|args| args.get("driver_type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("driver_type parameter required", None))?;

        let driver_type = crate::driver::DriverType::from_string(driver_type_str)
            .ok_or_else(|| McpError::invalid_params("Invalid driver_type. Use: chrome, firefox, or edge", None))?;

        let driver_manager = self.client_manager.get_driver_manager();
        
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

    async fn handle_stop_driver(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_type_str = arguments
            .as_ref()
            .and_then(|args| args.get("driver_type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("driver_type parameter required", None))?;

        let driver_type = crate::driver::DriverType::from_string(driver_type_str)
            .ok_or_else(|| McpError::invalid_params("Invalid driver_type. Use: chrome, firefox, or edge", None))?;

        let driver_manager = self.client_manager.get_driver_manager();
        
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

    async fn handle_stop_all_drivers(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_manager = self.client_manager.get_driver_manager();
        
        match driver_manager.stop_all_drivers().await {
            Ok(_) => Ok(success_response("Successfully stopped all WebDriver processes".to_string())),
            Err(e) => Ok(error_response(format!("Failed to stop all drivers: {e}"))),
        }
    }

    async fn handle_force_cleanup_orphaned_processes(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        match self.client_manager.force_cleanup_orphaned_processes_public().await {
            Ok(_) => Ok(success_response("Successfully force cleaned up all orphaned browser and WebDriver processes".to_string())),
            Err(e) => Ok(error_response(format!("Failed to force cleanup orphaned processes: {e}"))),
        }
    }

    /// Cleanup method to stop any managed driver processes
    pub async fn cleanup(&self) -> crate::error::Result<()> {
        tracing::info!("WebDriver MCP Server shutting down...");
        
        // First close all active WebDriver sessions gracefully
        tracing::debug!("Closing active WebDriver sessions...");
        if let Err(e) = self.client_manager.close_all_sessions().await {
            tracing::warn!("Error closing WebDriver sessions: {}", e);
        } else {
            tracing::debug!("WebDriver sessions closed successfully");
        }
        
        // Add a small delay to allow session cleanup to complete
        tracing::debug!("Waiting for session cleanup to complete...");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        
        // Then stop all managed driver processes
        tracing::debug!("Stopping WebDriver processes...");
        match self.client_manager
            .get_driver_manager()
            .stop_all_drivers()
            .await 
        {
            Ok(()) => tracing::debug!("Successfully stopped all WebDriver processes"),
            Err(e) => {
                tracing::warn!("Error stopping WebDriver processes: {}", e);
                
                // Fallback: Force cleanup any remaining orphaned processes
                tracing::info!("Attempting force cleanup of orphaned processes...");
                if let Err(cleanup_err) = self.client_manager.force_cleanup_orphaned_processes_public().await {
                    tracing::error!("Force cleanup also failed: {}", cleanup_err);
                } else {
                    tracing::info!("Force cleanup completed successfully");
                }
            },
        }
        
        tracing::debug!("WebDriver MCP Server cleanup completed");
        Ok(())
    }

    fn extract_session_id(arguments: &Option<Map<String, Value>>) -> Option<String> {
        arguments
            .as_ref()
            .and_then(|args| args.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    async fn handle_navigate(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let url = arguments
            .as_ref()
            .and_then(|args| args.get("url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("url parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.goto(url).await {
                Ok(_) => {
                    // Set up console monitoring immediately after navigation
                    if let Err(e) = self.setup_console_monitoring(&client).await {
                        eprintln!("Warning: Failed to setup console monitoring: {}", e);
                    }
                    Ok(success_response(format!(
                        "Successfully navigated to {url} (session: {session})"
                    )))
                },
                Err(e) => Ok(error_response(format!("Failed to navigate: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_click(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(element) => match element.click().await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully clicked element {selector} (session: {session})"
                        ))),
                        Err(e) => Ok(error_response(format!("Failed to click element: {e}"))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element {selector}: {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_send_keys(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let text = arguments
            .as_ref()
            .and_then(|args| args.get("text"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("text parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(element) => match element.send_keys(text).await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully sent keys to {selector} (session: {session})"
                        ))),
                        Err(e) => Ok(error_response(format!("Failed to send keys: {e}"))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element {selector}: {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_title(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.title().await {
                Ok(title) => Ok(success_response(format!(
                    "Page title: {title} (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to get title: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_text(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.find(Locator::Css(selector)).await {
                Ok(element) => match element.text().await {
                    Ok(text) => Ok(success_response(format!(
                        "Element text: {text} (session: {session})"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to get element text: {e}"))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find element {selector}: {e}"
                ))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_execute_script(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let script = arguments
            .as_ref()
            .and_then(|args| args.get("script"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("script parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.execute(script, vec![]).await {
                Ok(result) => Ok(success_response(format!(
                    "Script result: {result:?} (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to execute script: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_screenshot(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);
        
        let save_path = arguments
            .as_ref()
            .and_then(|args| args.get("save_path"))
            .and_then(|v| v.as_str());

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((_session, client)) => match client.screenshot().await {
                Ok(png_data) => {
                    // Validate that we have valid PNG data
                    if png_data.is_empty() {
                        return Ok(error_response("Screenshot data is empty".to_string()));
                    }

                    // Check if data starts with PNG signature
                    if png_data.len() < 4 || &png_data[0..4] != b"\x89PNG" {
                        return Ok(error_response("Screenshot data is not valid PNG format".to_string()));
                    }

                    // Save to disk if path is provided
                    if let Some(path) = save_path {
                        match std::fs::write(path, &png_data) {
                            Ok(_) => {
                                // Also return the image data for display
                                let base64_data = general_purpose::STANDARD.encode(&png_data);
                                Ok(CallToolResult {
                                    content: vec![
                                        Content::text(format!("Screenshot saved to: {} ({} bytes)", path, png_data.len())),
                                        Content::image(
                                            base64_data,
                                            "image/png",
                                        )
                                    ],
                                    is_error: Some(false),
                                })
                            }
                            Err(e) => Ok(error_response(format!("Failed to save screenshot to {path}: {e}"))),
                        }
                    } else {
                        // Just return the image data
                        let base64_data = general_purpose::STANDARD.encode(&png_data);
                        Ok(CallToolResult {
                            content: vec![
                                Content::text(format!("Screenshot taken ({} bytes)", png_data.len())),
                                Content::image(
                                    base64_data,
                                    "image/png",
                                )
                            ],
                            is_error: Some(false),
                        })
                    }
                }
                Err(e) => Ok(error_response(format!("Failed to take screenshot: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_resize_window(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);
        
        let width = arguments
            .as_ref()
            .and_then(|args| args.get("width"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::invalid_params("width parameter required", None))?;
            
        let height = arguments
            .as_ref()
            .and_then(|args| args.get("height"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| McpError::invalid_params("height parameter required", None))?;

        // Validate dimensions
        if width <= 0.0 || height <= 0.0 {
            return Ok(error_response("Width and height must be positive numbers".to_string()));
        }
        
        if width > 10000.0 || height > 10000.0 {
            return Ok(error_response("Width and height must be less than 10000 pixels".to_string()));
        }

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.set_window_size(width as u32, height as u32).await {
                Ok(_) => {
                    // Verify the resize by getting the current size
                    match client.get_window_size().await {
                        Ok((actual_width, actual_height)) => Ok(success_response(format!(
                            "Window resized to {}x{} pixels (session: {})", 
                            actual_width, actual_height, session
                        ))),
                        Err(_) => Ok(success_response(format!(
                            "Window resize command sent ({}x{}) (session: {})",
                            width, height, session
                        ))),
                    }
                }
                Err(e) => Ok(error_response(format!("Failed to resize window: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_current_url(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.current_url().await {
                Ok(url) => Ok(success_response(format!(
                    "Current URL: {url} (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to get current URL: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_back(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.back().await {
                Ok(_) => Ok(success_response(format!(
                    "Successfully navigated back (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to navigate back: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_forward(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.forward().await {
                Ok(_) => Ok(success_response(format!(
                    "Successfully navigated forward (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to navigate forward: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_refresh(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.refresh().await {
                Ok(_) => {
                    // Set up console monitoring immediately after refresh
                    if let Err(e) = self.setup_console_monitoring(&client).await {
                        eprintln!("Warning: Failed to setup console monitoring: {}", e);
                    }
                    Ok(success_response(format!(
                        "Successfully refreshed page (session: {session})"
                    )))
                },
                Err(e) => Ok(error_response(format!("Failed to refresh page: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_page_load_status(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match client.execute("return document.readyState", vec![]).await {
                    Ok(result) => {
                        let ready_state = result.as_str().unwrap_or("unknown");
                        let status_msg = match ready_state {
                            "complete" => "Page fully loaded",
                            "interactive" => "Page loaded but resources may still be loading",
                            "loading" => "Page still loading",
                            _ => "Unknown page state",
                        };
                        Ok(success_response(format!(
                            "Page load status: {status_msg} ({ready_state}) (session: {session})"
                        )))
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to check page load status: {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_wait_for_element(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let timeout_seconds = arguments
            .as_ref()
            .and_then(|args| args.get("timeout_seconds"))
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, Some(timeout_seconds))
                    .await
                {
                    Ok(_element) => Ok(success_response(format!(
                        "Element '{selector}' found within {timeout_seconds:.1}s (session: {session})"
                    ))),
                    Err(e) => Ok(error_response(format!(
                        "Element '{selector}' not found within {timeout_seconds:.1}s: {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_wait_for_condition(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let condition = arguments
            .as_ref()
            .and_then(|args| args.get("condition"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("condition parameter required", None))?;

        let timeout_seconds = arguments
            .as_ref()
            .and_then(|args| args.get("timeout_seconds"))
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);

        let check_interval_ms = arguments
            .as_ref()
            .and_then(|args| args.get("check_interval_ms"))
            .and_then(|v| v.as_f64())
            .unwrap_or(100.0) as u64;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let start_time = std::time::Instant::now();
                let timeout_duration = std::time::Duration::from_secs_f64(timeout_seconds);
                let check_interval = std::time::Duration::from_millis(check_interval_ms);

                loop {
                    // Check if condition is true
                    match client.execute(condition, vec![]).await {
                        Ok(result) => {
                            // Check if result is truthy
                            let is_true = match result {
                                serde_json::Value::Bool(b) => b,
                                serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                                serde_json::Value::String(s) => !s.is_empty(),
                                serde_json::Value::Array(arr) => !arr.is_empty(),
                                serde_json::Value::Object(obj) => !obj.is_empty(),
                                serde_json::Value::Null => false,
                            };

                            if is_true {
                                let elapsed = start_time.elapsed();
                                return Ok(success_response(format!(
                                    "Condition '{}' became true after {:.1}s (session: {})",
                                    condition,
                                    elapsed.as_secs_f64(),
                                    session
                                )));
                            }
                        }
                        Err(e) => {
                            // JavaScript error - condition might be malformed
                            return Ok(error_response(format!(
                                "Error evaluating condition '{}': {}",
                                condition, e
                            )));
                        }
                    }

                    // Check timeout
                    if start_time.elapsed() >= timeout_duration {
                        return Ok(error_response(format!(
                            "Condition '{}' did not become true within {:.1}s (session: {})",
                            condition, timeout_seconds, session
                        )));
                    }

                    // Wait before next check
                    tokio::time::sleep(check_interval).await;
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_element_info(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let include_computed_styles = arguments
            .as_ref()
            .and_then(|args| args.get("include_computed_styles"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let _element = if wait_timeout > 0.0 {
                    match self
                        .client_manager
                        .find_element_with_wait(&client, selector, Some(wait_timeout))
                        .await
                    {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Element '{selector}' not found within {wait_timeout:.1}s: {e}"
                            )));
                        }
                    }
                } else {
                    match client.find(Locator::Css(selector)).await {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Element '{selector}' not found: {e}"
                            )));
                        }
                    }
                };

                // JavaScript to get comprehensive element information
                let info_script = format!(
                    r#"
                    try {{
                        const element = document.querySelector('{}');
                        if (!element) {{
                            return {{ error: 'Element not found' }};
                        }}
                        
                        const rect = element.getBoundingClientRect();
                        const style = window.getComputedStyle(element);
                        
                        const info = {{
                            tagName: element.tagName.toLowerCase(),
                            id: element.id || null,
                            className: element.className || null,
                            
                            // Visibility
                            isVisible: rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none',
                            isInViewport: rect.top >= 0 && rect.left >= 0 && rect.bottom <= window.innerHeight && rect.right <= window.innerWidth,
                            
                            // Size and position
                            boundingRect: {{
                                x: Math.round(rect.x),
                                y: Math.round(rect.y),
                                width: Math.round(rect.width),
                                height: Math.round(rect.height),
                                top: Math.round(rect.top),
                                right: Math.round(rect.right),
                                bottom: Math.round(rect.bottom),
                                left: Math.round(rect.left)
                            }},
                            
                            // Offset dimensions
                            offsetWidth: element.offsetWidth,
                            offsetHeight: element.offsetHeight,
                            offsetTop: element.offsetTop,
                            offsetLeft: element.offsetLeft,
                            
                            // Client dimensions
                            clientWidth: element.clientWidth,
                            clientHeight: element.clientHeight,
                            
                            // Scroll dimensions
                            scrollWidth: element.scrollWidth,
                            scrollHeight: element.scrollHeight,
                            scrollTop: element.scrollTop,
                            scrollLeft: element.scrollLeft,
                            
                            // Key computed styles
                            computedStyles: {{
                                display: style.display,
                                visibility: style.visibility,
                                opacity: style.opacity,
                                position: style.position,
                                zIndex: style.zIndex,
                                overflow: style.overflow,
                                overflowX: style.overflowX,
                                overflowY: style.overflowY
                            }}{}
                        }};
                        
                        return info;
                    }} catch (e) {{
                        return {{ error: e.message }};
                    }}
                    "#,
                    selector.replace('\'', "\\'"),
                    if include_computed_styles {
                        r#",
                            allComputedStyles: {
                                width: style.width,
                                height: style.height,
                                margin: style.margin,
                                padding: style.padding,
                                border: style.border,
                                backgroundColor: style.backgroundColor,
                                color: style.color,
                                fontSize: style.fontSize,
                                fontFamily: style.fontFamily,
                                lineHeight: style.lineHeight,
                                textAlign: style.textAlign,
                                transform: style.transform,
                                transition: style.transition,
                                animation: style.animation
                            }"#
                    } else {
                        ""
                    }
                );

                match client.execute(&info_script, vec![]).await {
                    Ok(result) => {
                        if let Ok(info) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(result.clone()) {
                            if let Some(error) = info.get("error") {
                                Ok(error_response(format!("JavaScript error: {}", error)))
                            } else {
                                let formatted_info = serde_json::to_string_pretty(&info)
                                    .unwrap_or_else(|_| format!("{:?}", info));
                                Ok(success_response(format!(
                                    "Element info for '{}' (session: {}):\n{}",
                                    selector, session, formatted_info
                                )))
                            }
                        } else {
                            Ok(error_response(format!("Failed to parse element info: {:?}", result)))
                        }
                    }
                    Err(e) => Ok(error_response(format!("Failed to get element info: {e}"))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_element_attribute(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let attribute = arguments
            .as_ref()
            .and_then(|args| args.get("attribute"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("attribute parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(element) => match element.attr(attribute).await {
                        Ok(attr_value) => {
                            let value_text = attr_value.unwrap_or_else(|| {
                                format!("[attribute '{attribute}' not found or empty]")
                            });
                            Ok(success_response(format!(
                                "Element '{selector}' attribute '{attribute}': {value_text} (session: {session})"
                            )))
                        }
                        Err(e) => Ok(error_response(format!(
                            "Failed to get attribute '{attribute}' from element '{selector}': {e}"
                        ))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element '{selector}': {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_page_source(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.source().await {
                Ok(html) => Ok(success_response(format!(
                    "Page HTML source (session: {session}):\n\n{html}"
                ))),
                Err(e) => Ok(error_response(format!("Failed to get page source: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_element_property(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let property = arguments
            .as_ref()
            .and_then(|args| args.get("property"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("property parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(element) => match element.prop(property).await {
                        Ok(prop_value) => {
                            let value_text = match prop_value {
                                Some(s) => s,
                                None => "[null/undefined]".to_string(),
                            };
                            Ok(success_response(format!(
                                "Element '{selector}' property '{property}': {value_text} (session: {session})"
                            )))
                        }
                        Err(e) => Ok(error_response(format!(
                            "Failed to get property '{property}' from element '{selector}': {e}"
                        ))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element '{selector}': {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_find_element(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let parent_selector = arguments
            .as_ref()
            .and_then(|args| args.get("parent_selector"))
            .and_then(|v| v.as_str());

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                // If parent_selector is provided, find within parent
                let search_result = if let Some(parent_sel) = parent_selector {
                    // First find the parent element
                    let parent_element = if wait_timeout > 0.0 {
                        match self
                            .client_manager
                            .find_element_with_wait(&client, parent_sel, Some(wait_timeout))
                            .await
                        {
                            Ok(element) => element,
                            Err(e) => {
                                return Ok(error_response(format!(
                                    "Parent element '{}' not found within {:.1}s: {}",
                                    parent_sel, wait_timeout, e
                                )));
                            }
                        }
                    } else {
                        match client.find(Locator::Css(parent_sel)).await {
                            Ok(element) => element,
                            Err(e) => {
                                return Ok(error_response(format!(
                                    "Parent element '{}' not found: {}",
                                    parent_sel, e
                                )));
                            }
                        }
                    };

                    // Then find child element within parent
                    parent_element.find(Locator::Css(selector)).await
                        .map_err(|e| format!("Child element '{}' not found within parent '{}': {}", selector, parent_sel, e))
                } else {
                    // Standard search without parent
                    if wait_timeout > 0.0 {
                        self.client_manager
                            .find_element_with_wait(&client, selector, Some(wait_timeout))
                            .await
                            .map_err(|e| format!("Element '{}' not found within {:.1}s: {}", selector, wait_timeout, e))
                    } else {
                        client.find(Locator::Css(selector)).await
                            .map_err(|e| format!("Element '{}' not found: {}", selector, e))
                    }
                };

                match search_result {
                    Ok(element) => {
                        let tag_name = element
                            .tag_name()
                            .await
                            .unwrap_or_else(|_| "unknown".to_string());
                        let text_content = element
                            .text()
                            .await
                            .unwrap_or_else(|_| "[no text]".to_string());
                        let text_preview = if text_content.len() > 100 {
                            format!("{}...", &text_content[..97])
                        } else {
                            text_content
                        };

                        let scope_msg = if let Some(parent_sel) = parent_selector {
                            format!(" within parent '{}'", parent_sel)
                        } else {
                            String::new()
                        };

                        Ok(success_response(format!(
                            "Found element '{}'{} (session: {}): <{}> - Text: \"{}\"",
                            selector, scope_msg, session, tag_name, text_preview
                        )))
                    }
                    Err(e) => Ok(error_response(e)),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_find_elements(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let parent_selector = arguments
            .as_ref()
            .and_then(|args| args.get("parent_selector"))
            .and_then(|v| v.as_str());

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                // If parent_selector is provided, find within parent
                let search_result = if let Some(parent_sel) = parent_selector {
                    // First find the parent element
                    let parent_element = if wait_timeout > 0.0 {
                        match self
                            .client_manager
                            .find_element_with_wait(&client, parent_sel, Some(wait_timeout))
                            .await
                        {
                            Ok(element) => element,
                            Err(e) => {
                                return Ok(error_response(format!(
                                    "Parent element '{}' not found within {:.1}s: {}",
                                    parent_sel, wait_timeout, e
                                )));
                            }
                        }
                    } else {
                        match client.find(Locator::Css(parent_sel)).await {
                            Ok(element) => element,
                            Err(e) => {
                                return Ok(error_response(format!(
                                    "Parent element '{}' not found: {}",
                                    parent_sel, e
                                )));
                            }
                        }
                    };

                    // Then find child elements within parent
                    parent_element.find_all(Locator::Css(selector)).await
                        .map_err(|e| format!("Child elements '{}' not found within parent '{}': {}", selector, parent_sel, e))
                } else {
                    // Standard search without parent
                    client.find_all(Locator::Css(selector)).await
                        .map_err(|e| format!("Elements '{}' not found: {}", selector, e))
                };

                match search_result {
                    Ok(elements) => {
                        let scope_msg = if let Some(parent_sel) = parent_selector {
                            format!(" within parent '{}'", parent_sel)
                        } else {
                            String::new()
                        };

                        let mut result_text = format!(
                            "Found {} element(s) matching '{}'{} (session: {}):\n\n",
                            elements.len(),
                            selector,
                            scope_msg,
                            session
                        );

                        for (i, element) in elements.iter().enumerate() {
                            let tag_name = element
                                .tag_name()
                                .await
                                .unwrap_or_else(|_| "unknown".to_string());
                            let text_content = element
                                .text()
                                .await
                                .unwrap_or_else(|_| "[no text]".to_string());
                            let text_preview = if text_content.len() > 100 {
                                format!("{}...", &text_content[..97])
                            } else {
                                text_content
                            };

                            result_text.push_str(&format!(
                                "{}. <{}> - Text: \"{}\"\n",
                                i + 1,
                                tag_name,
                                text_preview
                            ));
                        }

                        Ok(success_response(result_text))
                    }
                    Err(e) => Ok(error_response(e)),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_scroll_to_element(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                // First, try to find the element
                match client.find(Locator::Css(selector)).await {
                    Ok(_element) => {
                        // Scroll the element into view using JavaScript with CSS selector
                        let scroll_script = format!(
                            "var element = document.querySelector('{}'); if (element) {{ element.scrollIntoView({{behavior: 'smooth', block: 'center'}}); }}",
                            selector.replace("'", "\\'")
                        );

                        match client.execute(&scroll_script, vec![]).await {
                            Ok(_) => {
                                // Wait a moment for smooth scrolling to complete
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                                Ok(success_response(format!(
                                    "Successfully scrolled to element '{selector}' (session: {session})"
                                )))
                            }
                            Err(e) => {
                                Ok(error_response(format!("Failed to scroll to element: {e}")))
                            }
                        }
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element '{selector}': {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_hover(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(_element) => {
                        // Use JavaScript to trigger mouse hover events
                        let hover_script = format!(
                            r#"
                            var element = document.querySelector('{}');
                            if (element) {{
                                var events = ['mouseenter', 'mouseover'];
                                events.forEach(function(eventType) {{
                                    var event = new MouseEvent(eventType, {{
                                        'view': window,
                                        'bubbles': true,
                                        'cancelable': true
                                    }});
                                    element.dispatchEvent(event);
                                }});
                            }}
                            "#,
                            selector.replace("'", "\\'")
                        );

                        match client.execute(&hover_script, vec![]).await {
                            Ok(_) => Ok(success_response(format!(
                                "Successfully hovered over element '{selector}' (session: {session})"
                            ))),
                            Err(e) => {
                                Ok(error_response(format!("Failed to hover over element: {e}")))
                            }
                        }
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to find element '{selector}': {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_fill_and_submit_form(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let fields = arguments
            .as_ref()
            .and_then(|args| args.get("fields"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| McpError::invalid_params("fields parameter required", None))?;

        let submit_selector = arguments
            .as_ref()
            .and_then(|args| args.get("submit_selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("submit_selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let mut filled_fields = Vec::new();

                // Fill each field
                for (field_selector, value) in fields {
                    if let Some(text_value) = value.as_str() {
                        match client.find(Locator::Css(field_selector)).await {
                            Ok(element) => {
                                // Clear the field first
                                if let Err(e) = element.clear().await {
                                    return Ok(error_response(format!(
                                        "Failed to clear field '{field_selector}': {e}"
                                    )));
                                }

                                // Then send keys
                                if let Err(e) = element.send_keys(text_value).await {
                                    return Ok(error_response(format!(
                                        "Failed to fill field '{field_selector}': {e}"
                                    )));
                                }

                                filled_fields.push(field_selector.clone());
                            }
                            Err(e) => {
                                return Ok(error_response(format!(
                                    "Failed to find field '{field_selector}': {e}"
                                )));
                            }
                        }
                    }
                }

                // Submit the form
                match client.find(Locator::Css(submit_selector)).await {
                    Ok(submit_element) => match submit_element.click().await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully filled {} fields and submitted form (session: {}). Fields: {}",
                            filled_fields.len(),
                            session,
                            filled_fields.join(", ")
                        ))),
                        Err(e) => Ok(error_response(format!("Failed to submit form: {e}"))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find submit element '{submit_selector}': {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_login_form(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let username = arguments
            .as_ref()
            .and_then(|args| args.get("username"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("username parameter required", None))?;

        let password = arguments
            .as_ref()
            .and_then(|args| args.get("password"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("password parameter required", None))?;

        // Get optional custom selectors
        let username_selector = arguments
            .as_ref()
            .and_then(|args| args.get("username_selector"))
            .and_then(|v| v.as_str());

        let password_selector = arguments
            .as_ref()
            .and_then(|args| args.get("password_selector"))
            .and_then(|v| v.as_str());

        let submit_selector = arguments
            .as_ref()
            .and_then(|args| args.get("submit_selector"))
            .and_then(|v| v.as_str());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                // Define common login field selectors to try
                let default_username_selectors = vec![
                    "input[type='email']",
                    "input[type='text'][name*='user']",
                    "input[type='text'][name*='email']",
                    "input[name='username']",
                    "input[name='email']",
                    "input[id*='user']",
                    "input[id*='email']",
                    "#username",
                    "#email",
                    "[placeholder*='email' i]",
                    "[placeholder*='username' i]",
                ];

                let default_password_selectors = vec![
                    "input[type='password']",
                    "input[name='password']",
                    "#password",
                    "[placeholder*='password' i]",
                ];

                let default_submit_selectors = vec![
                    "button[type='submit']",
                    "input[type='submit']",
                    "button:contains('Sign in')",
                    "button:contains('Login')",
                    "button:contains('Log in')",
                    "[role='button']:contains('Sign in')",
                    "[role='button']:contains('Login')",
                    "button",
                ];

                // Try to find and fill username field
                let username_found = if let Some(selector) = username_selector {
                    // Use custom selector
                    match client.find(Locator::Css(selector)).await {
                        Ok(element) => {
                            if let Err(e) = element.clear().await {
                                return Ok(error_response(format!(
                                    "Failed to clear username field '{selector}': {e}"
                                )));
                            }
                            if let Err(e) = element.send_keys(username).await {
                                return Ok(error_response(format!(
                                    "Failed to fill username field '{selector}': {e}"
                                )));
                            }
                            true
                        }
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Failed to find username field with custom selector '{selector}': {e}"
                            )));
                        }
                    }
                } else {
                    // Try default selectors
                    let mut found = false;
                    for selector in &default_username_selectors {
                        if let Ok(element) = client.find(Locator::Css(selector)).await {
                            if element.clear().await.is_ok() && element.send_keys(username).await.is_ok() {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                };

                if !username_found {
                    return Ok(error_response(
                        "Could not find username/email field. Try providing a custom username_selector".to_string()
                    ));
                }

                // Try to find and fill password field
                let password_found = if let Some(selector) = password_selector {
                    // Use custom selector
                    match client.find(Locator::Css(selector)).await {
                        Ok(element) => {
                            if let Err(e) = element.clear().await {
                                return Ok(error_response(format!(
                                    "Failed to clear password field '{selector}': {e}"
                                )));
                            }
                            if let Err(e) = element.send_keys(password).await {
                                return Ok(error_response(format!(
                                    "Failed to fill password field '{selector}': {e}"
                                )));
                            }
                            true
                        }
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Failed to find password field with custom selector '{selector}': {e}"
                            )));
                        }
                    }
                } else {
                    // Try default selectors
                    let mut found = false;
                    for selector in &default_password_selectors {
                        if let Ok(element) = client.find(Locator::Css(selector)).await {
                            if element.clear().await.is_ok() && element.send_keys(password).await.is_ok() {
                                found = true;
                                break;
                            }
                        }
                    }
                    found
                };

                if !password_found {
                    return Ok(error_response(
                        "Could not find password field. Try providing a custom password_selector".to_string()
                    ));
                }

                // Try to find and click submit button
                if let Some(selector) = submit_selector {
                    // Use custom selector
                    match client.find(Locator::Css(selector)).await {
                        Ok(element) => match element.click().await {
                            Ok(_) => Ok(success_response(format!(
                                "Successfully filled login form and submitted (session: {session})"
                            ))),
                            Err(e) => Ok(error_response(format!(
                                "Login form filled but failed to click submit button. Error: {e}"
                            ))),
                        },
                        Err(e) => Ok(error_response(format!(
                            "Failed to find submit button with custom selector '{selector}': {e}"
                        ))),
                    }
                } else {
                    // Try default selectors
                    let mut submit_clicked = false;
                    for selector in &default_submit_selectors {
                        if let Ok(element) = client.find(Locator::Css(selector)).await {
                            if element.click().await.is_ok() {
                                submit_clicked = true;
                                break;
                            }
                        }
                    }
                    if submit_clicked {
                        Ok(success_response(format!(
                            "Successfully filled login form and submitted (session: {session})"
                        )))
                    } else {
                        Ok(error_response(
                            "Could not find submit button. Try providing a custom submit_selector".to_string()
                        ))
                    }
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_console_logs(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let level_filter = arguments
            .as_ref()
            .and_then(|args| args.get("level"))
            .and_then(|v| v.as_str())
            .unwrap_or("all");

        let since_timestamp = arguments
            .as_ref()
            .and_then(|args| args.get("since_timestamp"))
            .and_then(|v| v.as_f64());

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64())
            .unwrap_or(2.0); // Default to 2 seconds

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                // Wait for JavaScript execution to complete before capturing logs
                if wait_timeout > 0.0 {
                    tokio::time::sleep(std::time::Duration::from_secs_f64(wait_timeout)).await;
                }

                // Simple script to retrieve stored console logs
                let retrieve_script = r#"
                    try {
                        return window.__mcpConsoleLogs || [];
                    } catch (e) {
                        return [];
                    }
                "#;

                match client.execute(retrieve_script, vec![]).await {
                    Ok(result) => {
                        // Try to parse the result as JSON array of log entries
                        let formatted_logs = if let Ok(logs) = serde_json::from_value::<Vec<serde_json::Value>>(result.clone()) {
                            if logs.is_empty() {
                                "No console logs found.".to_string()
                            } else {
                                logs.into_iter()
                                    .filter(|log| {
                                        // Filter by level
                                        if level_filter != "all" {
                                            let log_level = log.get("level").and_then(|v| v.as_str()).unwrap_or("");
                                            if log_level != level_filter {
                                                return false;
                                            }
                                        }
                                        
                                        // Filter by timestamp
                                        if let Some(since) = since_timestamp {
                                            let log_timestamp = log.get("timestamp").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                            if log_timestamp < since {
                                                return false;
                                            }
                                        }
                                        
                                        true
                                    })
                                    .map(|log| {
                                        let level = log.get("level").and_then(|v| v.as_str()).unwrap_or("unknown");
                                        let message = log.get("message").and_then(|v| v.as_str()).unwrap_or("");
                                        let timestamp = log.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
                                        let _url = log.get("url").and_then(|v| v.as_str()).unwrap_or("");
                                        
                                        let time_str = if timestamp > 0 {
                                            format!("[{}ms] ", timestamp)
                                        } else {
                                            "".to_string()
                                        };
                                        
                                        format!("{time_str}{level}: {message}")
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }
                        } else {
                            // Fallback if parsing fails
                            format!("Raw result: {result:?}")
                        };

                        Ok(success_response(format!(
                            "Console logs (session: {session}):\n{formatted_logs}"
                        )))
                    }
                    Err(e) => Ok(error_response(format!("Failed to retrieve console logs: {e}"))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_performance_metrics(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let include_resources = arguments
            .as_ref()
            .and_then(|args| args.get("include_resources"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_navigation = arguments
            .as_ref()
            .and_then(|args| args.get("include_navigation"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_paint = arguments
            .as_ref()
            .and_then(|args| args.get("include_paint"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let performance_script = format!(r#"
                    const metrics = {{}};
                    
                    // Basic timing info
                    if (performance.timing) {{
                        metrics.timing = {{
                            navigationStart: performance.timing.navigationStart,
                            loadEventEnd: performance.timing.loadEventEnd,
                            domContentLoadedEventEnd: performance.timing.domContentLoadedEventEnd,
                            responseEnd: performance.timing.responseEnd,
                            domComplete: performance.timing.domComplete
                        }};
                        
                        metrics.calculated = {{
                            pageLoadTime: performance.timing.loadEventEnd - performance.timing.navigationStart,
                            domContentLoadedTime: performance.timing.domContentLoadedEventEnd - performance.timing.navigationStart,
                            responseTime: performance.timing.responseEnd - performance.timing.navigationStart
                        }};
                    }}
                    
                    // Navigation timing (newer API)
                    if ({include_navigation} && performance.getEntriesByType) {{
                        const nav = performance.getEntriesByType('navigation')[0];
                        if (nav) {{
                            metrics.navigation = {{
                                type: nav.type,
                                redirectCount: nav.redirectCount,
                                transferSize: nav.transferSize,
                                encodedBodySize: nav.encodedBodySize,
                                decodedBodySize: nav.decodedBodySize,
                                duration: nav.duration,
                                domContentLoadedEventStart: nav.domContentLoadedEventStart,
                                domContentLoadedEventEnd: nav.domContentLoadedEventEnd,
                                loadEventStart: nav.loadEventStart,
                                loadEventEnd: nav.loadEventEnd
                            }};
                        }}
                    }}
                    
                    // Resource timing
                    if ({include_resources} && performance.getEntriesByType) {{
                        const resources = performance.getEntriesByType('resource');
                        metrics.resources = resources.map(r => ({{
                            name: r.name,
                            duration: r.duration,
                            transferSize: r.transferSize,
                            encodedBodySize: r.encodedBodySize,
                            decodedBodySize: r.decodedBodySize,
                            initiatorType: r.initiatorType
                        }})).slice(0, 50); // Limit to first 50 resources
                    }}
                    
                    // Paint timing
                    if ({include_paint} && performance.getEntriesByType) {{
                        const paintEntries = performance.getEntriesByType('paint');
                        metrics.paint = {{}};
                        paintEntries.forEach(entry => {{
                            metrics.paint[entry.name] = entry.startTime;
                        }});
                    }}
                    
                    // Memory info if available
                    if (performance.memory) {{
                        metrics.memory = {{
                            usedJSHeapSize: performance.memory.usedJSHeapSize,
                            totalJSHeapSize: performance.memory.totalJSHeapSize,
                            jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                        }};
                    }}
                    
                    return metrics;
                "#);

                match client.execute(&performance_script, vec![]).await {
                    Ok(result) => Ok(success_response(format!(
                        "Performance metrics collected (session: {session}):\n{result:#?}"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to collect performance metrics: {e}"))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
        }
    }

    async fn handle_monitor_memory_usage(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let duration_seconds = arguments
            .as_ref()
            .and_then(|args| args.get("duration_seconds"))
            .and_then(|v| v.as_f64())
            .unwrap_or(10.0);
        let interval_ms = arguments
            .as_ref()
            .and_then(|args| args.get("interval_ms"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1000.0);
        let include_gc_info = arguments
            .as_ref()
            .and_then(|args| args.get("include_gc_info"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let memory_script = format!(r#"
                    return new Promise((resolve) => {{
                        const samples = [];
                        const startTime = Date.now();
                        const duration = {duration_seconds} * 1000;
                        const interval = {interval_ms};
                        
                        function collectSample() {{
                            const sample = {{
                                timestamp: Date.now() - startTime,
                                url: window.location.href
                            }};
                            
                            if (performance.memory) {{
                                sample.memory = {{
                                    usedJSHeapSize: performance.memory.usedJSHeapSize,
                                    totalJSHeapSize: performance.memory.totalJSHeapSize,
                                    jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                                }};
                            }}
                            
                            // Try to get GC info if available
                            if ({include_gc_info} && performance.measureUserAgentSpecificMemory) {{
                                performance.measureUserAgentSpecificMemory().then(result => {{
                                    sample.detailedMemory = result;
                                }}).catch(() => {{
                                    // GC info not available
                                }});
                            }}
                            
                            samples.push(sample);
                            
                            if (Date.now() - startTime < duration) {{
                                setTimeout(collectSample, interval);
                            }} else {{
                                // Calculate memory leak indicators
                                const analysis = {{}};
                                if (samples.length > 1) {{
                                    const first = samples[0];
                                    const last = samples[samples.length - 1];
                                    
                                    if (first.memory && last.memory) {{
                                        analysis.memoryGrowth = {{
                                            usedHeapGrowth: last.memory.usedJSHeapSize - first.memory.usedJSHeapSize,
                                            totalHeapGrowth: last.memory.totalJSHeapSize - first.memory.totalJSHeapSize,
                                            growthRate: (last.memory.usedJSHeapSize - first.memory.usedJSHeapSize) / (duration / 1000)
                                        }};
                                        
                                        analysis.leakIndicators = {{
                                            steadyGrowth: analysis.memoryGrowth.usedHeapGrowth > 1024 * 1024, // 1MB growth
                                            highGrowthRate: analysis.memoryGrowth.growthRate > 512 * 1024 // 512KB/sec
                                        }};
                                    }}
                                }}
                                
                                resolve({{
                                    samples: samples,
                                    analysis: analysis,
                                    summary: {{
                                        duration: duration,
                                        sampleCount: samples.length,
                                        interval: interval
                                    }}
                                }});
                            }}
                        }}
                        
                        collectSample();
                    }});
                "#);

                match client.execute(&memory_script, vec![]).await {
                    Ok(result) => Ok(success_response(format!(
                        "Memory monitoring completed (session: {session}):\n{result:#?}"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to monitor memory usage: {e}"))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
        }
    }

    async fn handle_run_performance_test(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let test_actions = arguments
            .as_ref()
            .and_then(|args| args.get("test_actions"))
            .and_then(|v| v.as_array())
            .ok_or_else(|| McpError::invalid_params("test_actions array is required", None))?;
        let iterations = arguments
            .as_ref()
            .and_then(|args| args.get("iterations"))
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0) as usize;
        let collect_screenshots = arguments
            .as_ref()
            .and_then(|args| args.get("collect_screenshots"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let mut results = Vec::new();
                
                for iteration in 0..iterations {
                    let mut iteration_results = Vec::new();
                    
                    // Start performance monitoring
                    let start_script = r#"
                        window.__perfTestStart = performance.now();
                        window.__perfTestMarks = [];
                        return "Performance test started";
                    "#;
                    client.execute(start_script, vec![]).await.ok();
                    
                    // Execute test actions
                    for (action_idx, action) in test_actions.iter().enumerate() {
                        let action_obj = action.as_object().ok_or_else(|| {
                            McpError::invalid_params("Each test action must be an object", None)
                        })?;
                        
                        let action_type = action_obj.get("type")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| McpError::invalid_params("Action type is required", None))?;
                        
                        let mark_script = format!(r#"
                            window.__perfTestMarks.push({{
                                action: "{action_type}",
                                index: {action_idx},
                                timestamp: performance.now() - window.__perfTestStart
                            }});
                        "#);
                        client.execute(&mark_script, vec![]).await.ok();
                        
                        match action_type {
                            "click" => {
                                if let Some(selector) = action_obj.get("selector").and_then(|v| v.as_str()) {
                                    if let Ok(element) = client.find(Locator::Css(selector)).await {
                                        element.click().await.ok();
                                    }
                                }
                            }
                            "scroll" => {
                                if let Some(selector) = action_obj.get("selector").and_then(|v| v.as_str()) {
                                    let scroll_script = format!("document.querySelector('{selector}')?.scrollIntoView();");
                                    client.execute(&scroll_script, vec![]).await.ok();
                                }
                            }
                            "wait" => {
                                if let Some(duration_ms) = action_obj.get("duration_ms").and_then(|v| v.as_f64()) {
                                    tokio::time::sleep(std::time::Duration::from_millis(duration_ms as u64)).await;
                                }
                            }
                            "navigate" => {
                                if let Some(url) = action_obj.get("url").and_then(|v| v.as_str()) {
                                    client.goto(url).await.ok();
                                }
                            }
                            _ => {
                                // Unknown action type, skip
                            }
                        }
                        
                        // Small delay between actions
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                    
                    // Collect final metrics
                    let end_script = r#"
                        const endTime = performance.now();
                        const testDuration = endTime - window.__perfTestStart;
                        
                        const result = {
                            testDuration: testDuration,
                            marks: window.__perfTestMarks,
                            finalMetrics: {}
                        };
                        
                        // Collect performance metrics
                        if (performance.memory) {
                            result.finalMetrics.memory = {
                                usedJSHeapSize: performance.memory.usedJSHeapSize,
                                totalJSHeapSize: performance.memory.totalJSHeapSize,
                                jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                            };
                        }
                        
                        // Collect paint metrics
                        const paintEntries = performance.getEntriesByType('paint');
                        result.finalMetrics.paint = {};
                        paintEntries.forEach(entry => {
                            result.finalMetrics.paint[entry.name] = entry.startTime;
                        });
                        
                        return result;
                    "#;
                    
                    match client.execute(end_script, vec![]).await {
                        Ok(iteration_result) => {
                            iteration_results.push(iteration_result);
                            
                            if collect_screenshots {
                                if let Ok(screenshot) = client.screenshot().await {
                                    // Convert screenshot to base64
                                    let screenshot_b64 = general_purpose::STANDARD.encode(&screenshot);
                                    iteration_results.push(serde_json::json!({
                                        "screenshot": format!("data:image/png;base64,{}", screenshot_b64)
                                    }));
                                }
                            }
                        }
                        Err(e) => {
                            iteration_results.push(serde_json::json!({
                                "error": format!("Failed to collect metrics: {}", e)
                            }));
                        }
                    }
                    
                    results.push(serde_json::json!({
                        "iteration": iteration,
                        "results": iteration_results
                    }));
                }
                
                Ok(success_response(format!(
                    "Performance test completed (session: {session}):\n{results:#?}"
                )))
            }
            Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
        }
    }

    async fn handle_monitor_resource_usage(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let duration_seconds = arguments
            .as_ref()
            .and_then(|args| args.get("duration_seconds"))
            .and_then(|v| v.as_f64())
            .unwrap_or(30.0);
        let include_network = arguments
            .as_ref()
            .and_then(|args| args.get("include_network"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_cpu = arguments
            .as_ref()
            .and_then(|args| args.get("include_cpu"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let include_fps = arguments
            .as_ref()
            .and_then(|args| args.get("include_fps"))
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let network_filter = arguments
            .as_ref()
            .and_then(|args| args.get("network_filter"))
            .and_then(|v| v.as_str())
            .unwrap_or(".*");
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                let resource_script = format!(r#"
                    return new Promise((resolve) => {{
                        const results = {{
                            network: [],
                            fps: [],
                            cpu: [],
                            summary: {{}}
                        }};
                        
                        const startTime = performance.now();
                        const duration = {duration_seconds} * 1000;
                        const networkFilter = new RegExp('{network_filter}');
                        
                        // Network monitoring
                        if ({include_network}) {{
                            const observer = new PerformanceObserver((list) => {{
                                for (const entry of list.getEntries()) {{
                                    if (entry.entryType === 'resource' && networkFilter.test(entry.name)) {{
                                        results.network.push({{
                                            name: entry.name,
                                            type: entry.initiatorType,
                                            duration: entry.duration,
                                            transferSize: entry.transferSize,
                                            encodedBodySize: entry.encodedBodySize,
                                            startTime: entry.startTime,
                                            responseEnd: entry.responseEnd
                                        }});
                                    }}
                                }}
                            }});
                            observer.observe({{entryTypes: ['resource']}});
                        }}
                        
                        // FPS monitoring
                        if ({include_fps}) {{
                            let frameCount = 0;
                            let lastTime = performance.now();
                            
                            function countFrame() {{
                                frameCount++;
                                const currentTime = performance.now();
                                
                                if (currentTime - lastTime >= 1000) {{
                                    results.fps.push({{
                                        timestamp: currentTime - startTime,
                                        fps: frameCount
                                    }});
                                    frameCount = 0;
                                    lastTime = currentTime;
                                }}
                                
                                if (currentTime - startTime < duration) {{
                                    requestAnimationFrame(countFrame);
                                }}
                            }}
                            requestAnimationFrame(countFrame);
                        }}
                        
                        // CPU monitoring (approximation using timing)
                        if ({include_cpu}) {{
                            let cpuSamples = [];
                            
                            function sampleCPU() {{
                                const start = performance.now();
                                
                                // Perform a small CPU-intensive task to measure responsiveness
                                let sum = 0;
                                for (let i = 0; i < 10000; i++) {{
                                    sum += Math.random();
                                }}
                                
                                const end = performance.now();
                                const cpuTime = end - start;
                                
                                cpuSamples.push({{
                                    timestamp: start - startTime,
                                    taskTime: cpuTime,
                                    responsiveness: cpuTime < 5 ? 'good' : cpuTime < 15 ? 'fair' : 'poor'
                                }});
                                
                                if (end - startTime < duration) {{
                                    setTimeout(sampleCPU, 1000);
                                }}
                            }}
                            setTimeout(sampleCPU, 100);
                        }}
                        
                        // Final collection
                        setTimeout(() => {{
                            results.summary = {{
                                duration: duration,
                                networkRequests: results.network.length,
                                averageFPS: results.fps.length > 0 ? 
                                    results.fps.reduce((a, b) => a + b.fps, 0) / results.fps.length : 0,
                                totalTransferSize: results.network.reduce((a, b) => a + (b.transferSize || 0), 0),
                                slowRequests: results.network.filter(r => r.duration > 1000).length
                            }};
                            
                            resolve(results);
                        }}, duration + 100);
                    }});
                "#);

                match client.execute(&resource_script, vec![]).await {
                    Ok(result) => Ok(success_response(format!(
                        "Resource usage monitoring completed (session: {session}):\n{result:#?}"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to monitor resource usage: {e}"))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
        }
    }

    // Helper method to inject console monitoring script
    async fn setup_console_monitoring(&self, client: &fantoccini::Client) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let console_script = r#"
            try {
                if (!window.__mcpConsoleLogs) {
                    window.__mcpConsoleLogs = [];
                    
                    const originalConsole = {
                        log: console.log,
                        error: console.error,
                        warn: console.warn,
                        info: console.info,
                        debug: console.debug
                    };
                    
                    ['log', 'error', 'warn', 'info', 'debug'].forEach(level => {
                        console[level] = function(...args) {
                            originalConsole[level].apply(console, args);
                            window.__mcpConsoleLogs.push({
                                level: level,
                                message: args.map(arg => {
                                    if (typeof arg === 'object') {
                                        try {
                                            return JSON.stringify(arg, null, 2);
                                        } catch (e) {
                                            return String(arg);
                                        }
                                    }
                                    return String(arg);
                                }).join(' '),
                                timestamp: Date.now(),
                                url: window.location.href
                            });
                        };
                    });
                    
                    window.onerror = function(message, source, lineno, colno, error) {
                        window.__mcpConsoleLogs.push({
                            level: 'error',
                            message: message + ' at ' + source + ':' + lineno + ':' + colno,
                            timestamp: Date.now(),
                            url: window.location.href,
                            stack: error ? error.stack : null
                        });
                        return false;
                    };
                    
                    window.addEventListener('unhandledrejection', function(event) {
                        window.__mcpConsoleLogs.push({
                            level: 'error',
                            message: 'Unhandled Promise Rejection: ' + event.reason,
                            timestamp: Date.now(),
                            url: window.location.href
                        });
                    });
                }
                return true;
            } catch (e) {
                return false;
            }
        "#;

        client.execute(console_script, vec![]).await?;
        Ok(())
    }

}

impl ServerHandler for WebDriverServer {
    fn get_info(&self) -> InitializeResult {
        InitializeResult {
            protocol_version: ProtocolVersion::V_2024_11_05,
            server_info: Implementation {
                name: "rust-browser-mcp".to_string(),
                version: "0.1.0".to_string(),
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability::default()),
                ..Default::default()
            },
            instructions: Some("WebDriver MCP Server - Browser automation for Claude".to_string()),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: ToolDefinitions::list_for_mode(self.mode),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "navigate" => self.handle_navigate(&request.arguments).await,
            "click" => self.handle_click(&request.arguments).await,
            "send_keys" => self.handle_send_keys(&request.arguments).await,
            "get_title" => self.handle_get_title(&request.arguments).await,
            "get_text" => self.handle_get_text(&request.arguments).await,
            "execute_script" => self.handle_execute_script(&request.arguments).await,
            "screenshot" => self.handle_screenshot(&request.arguments).await,
            "resize_window" => self.handle_resize_window(&request.arguments).await,
            "get_current_url" => self.handle_get_current_url(&request.arguments).await,
            "back" => self.handle_back(&request.arguments).await,
            "forward" => self.handle_forward(&request.arguments).await,
            "refresh" => self.handle_refresh(&request.arguments).await,
            "get_page_load_status" => self.handle_get_page_load_status(&request.arguments).await,
            "wait_for_element" => self.handle_wait_for_element(&request.arguments).await,
            "wait_for_condition" => self.handle_wait_for_condition(&request.arguments).await,
            "get_element_info" => self.handle_get_element_info(&request.arguments).await,
            "get_attribute" => self.handle_get_element_attribute(&request.arguments).await,
            "get_page_source" => self.handle_get_page_source(&request.arguments).await,
            "get_property" => self.handle_get_element_property(&request.arguments).await,
            "find_elements" => self.handle_find_elements(&request.arguments).await,
            "find_element" => self.handle_find_element(&request.arguments).await,
            "scroll_to_element" => self.handle_scroll_to_element(&request.arguments).await,
            "hover" => self.handle_hover(&request.arguments).await,
            "fill_and_submit_form" => self.handle_fill_and_submit_form(&request.arguments).await,
            "login_form" => self.handle_login_form(&request.arguments).await,
            "get_console_logs" => self.handle_get_console_logs(&request.arguments).await,
            "get_performance_metrics" => self.handle_get_performance_metrics(&request.arguments).await,
            "monitor_memory_usage" => self.handle_monitor_memory_usage(&request.arguments).await,
            "run_performance_test" => self.handle_run_performance_test(&request.arguments).await,
            "monitor_resource_usage" => self.handle_monitor_resource_usage(&request.arguments).await,
            // Driver lifecycle tools - only available in stdio mode
            "get_healthy_endpoints" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_get_healthy_endpoints(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "refresh_driver_health" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_refresh_driver_health(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "list_managed_drivers" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_list_managed_drivers(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "start_driver" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_start_driver(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "stop_driver" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_stop_driver(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "stop_all_drivers" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_stop_all_drivers(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "force_cleanup_orphaned_processes" => {
                if self.mode == ServerMode::Stdio {
                    self.handle_force_cleanup_orphaned_processes(&request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            // Recipe management tools (available in both modes)
            "create_recipe" => self.handle_create_recipe(&request.arguments).await,
            "list_recipes" => self.handle_list_recipes(&request.arguments).await,
            "get_recipe" => self.handle_get_recipe(&request.arguments).await,
            "execute_recipe" => self.handle_execute_recipe(&request.arguments).await,
            "delete_recipe" => self.handle_delete_recipe(&request.arguments).await,
            "create_recipe_template" => self.handle_create_recipe_template(&request.arguments).await,
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

impl WebDriverServer {
    // Recipe management tool handlers

    async fn handle_create_recipe(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let recipe_json = arguments
            .as_ref()
            .and_then(|args| args.get("recipe_json"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("recipe_json parameter required", None))?;

        match crate::Recipe::from_json(recipe_json) {
            Ok(recipe) => {
                match recipe.validate() {
                    Ok(_) => {
                        match self.recipe_manager.save_recipe(&recipe).await {
                            Ok(file_path) => Ok(success_response(format!(
                                "Recipe '{}' created successfully at {}",
                                recipe.name,
                                file_path.display()
                            ))),
                            Err(e) => Ok(error_response(format!("Failed to save recipe: {}", e))),
                        }
                    }
                    Err(e) => Ok(error_response(format!("Recipe validation failed: {}", e))),
                }
            }
            Err(e) => Ok(error_response(format!("Invalid recipe JSON: {}", e))),
        }
    }

    async fn handle_list_recipes(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        match self.recipe_manager.list_recipes().await {
            Ok(recipes) => {
                if recipes.is_empty() {
                    Ok(success_response("No recipes found".to_string()))
                } else {
                    let mut result = String::from("Available recipes:\n");
                    for recipe in recipes {
                        result.push_str(&format!("  {} (v{})", recipe.name, recipe.version));
                        if let Some(desc) = &recipe.description {
                            result.push_str(&format!(" - {}", desc));
                        }
                        result.push_str(&format!(" - {} steps\n", recipe.step_count));
                    }
                    Ok(success_response(result))
                }
            }
            Err(e) => Ok(error_response(format!("Failed to list recipes: {}", e))),
        }
    }

    async fn handle_get_recipe(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let name = arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

        match self.recipe_manager.load_recipe(name).await {
            Ok(recipe) => {
                match recipe.to_json() {
                    Ok(json) => Ok(success_response(json)),
                    Err(e) => Ok(error_response(format!("Failed to serialize recipe: {}", e))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to load recipe '{}': {}", name, e))),
        }
    }

    async fn handle_execute_recipe(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let name = arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

        let parameters: Option<std::collections::HashMap<String, String>> = arguments
            .as_ref()
            .and_then(|args| args.get("parameters"))
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            });

        let session_id = arguments
            .as_ref()
            .and_then(|args| args.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let continue_on_error = arguments
            .as_ref()
            .and_then(|args| args.get("continue_on_error"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Load the recipe
        let recipe = match self.recipe_manager.load_recipe(name).await {
            Ok(recipe) => recipe,
            Err(e) => return Ok(error_response(format!("Failed to load recipe '{}': {}", name, e))),
        };

        // Create execution context
        let context = ExecutionContext {
            session_id,
            variables: std::collections::HashMap::new(),
            continue_on_error,
        };

        // Execute the recipe
        let executor = RecipeExecutor::new(self);
        match executor.execute_recipe(&recipe, parameters, context).await {
            Ok(result) => {
                if result.success {
                    Ok(success_response(result.to_summary_string()))
                } else {
                    Ok(error_response(result.to_detailed_string()))
                }
            }
            Err(e) => Ok(error_response(format!("Recipe execution failed: {}", e))),
        }
    }

    async fn handle_delete_recipe(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let name = arguments
            .as_ref()
            .and_then(|args| args.get("name"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

        match self.recipe_manager.delete_recipe(name).await {
            Ok(_) => Ok(success_response(format!("Recipe '{}' deleted successfully", name))),
            Err(e) => Ok(error_response(format!("Failed to delete recipe '{}': {}", name, e))),
        }
    }

    async fn handle_create_recipe_template(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let template_type = arguments
            .as_ref()
            .and_then(|args| args.get("template"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("template parameter required", None))?;

        // Helper function to parse browsers array
        let parse_browsers = |args: &Option<Map<String, Value>>| -> Vec<String> {
            args.as_ref()
                .and_then(|args| args.get("browsers"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .filter(|browsers: &Vec<String>| !browsers.is_empty())
                .unwrap_or_else(|| vec!["auto".to_string()])
        };

        let template = match template_type {
            "login_and_screenshot" => {
                let base_url = arguments
                    .as_ref()
                    .and_then(|args| args.get("base_url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("http://localhost:3000")
                    .to_string();

                let username = arguments
                    .as_ref()
                    .and_then(|args| args.get("username"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("user")
                    .to_string();

                let password = arguments
                    .as_ref()
                    .and_then(|args| args.get("password"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("password")
                    .to_string();

                let browsers = Some(parse_browsers(arguments));
                
                RecipeTemplate::LoginAndScreenshot {
                    base_url,
                    username,
                    password,
                    browsers,
                }
            }
            "multi_browser_screenshot" => {
                let url = arguments
                    .as_ref()
                    .and_then(|args| args.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://example.com")
                    .to_string();
                let browsers = parse_browsers(arguments);
                RecipeTemplate::MultiBrowserScreenshot { url, browsers }
            }
            "responsive_test" => {
                let url = arguments
                    .as_ref()
                    .and_then(|args| args.get("url"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://example.com")
                    .to_string();
                let browsers = parse_browsers(arguments);
                let resolutions = arguments
                    .as_ref()
                    .and_then(|args| args.get("resolutions"))
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| {
                                v.as_object().and_then(|obj| {
                                    let width = obj.get("width")?.as_f64()? as u32;
                                    let height = obj.get("height")?.as_f64()? as u32;
                                    Some((width, height))
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| vec![(1920, 1080), (768, 1024), (375, 667)]);
                RecipeTemplate::ResponsiveTest { url, browsers, resolutions }
            }
            _ => return Ok(error_response(format!("Unknown template type: {}", template_type))),
        };

        match self.recipe_manager.create_recipe_from_template(template).await {
            Ok(recipe) => {
                match self.recipe_manager.save_recipe(&recipe).await {
                    Ok(file_path) => Ok(success_response(format!(
                        "Recipe '{}' created from template at {}",
                        recipe.name,
                        file_path.display()
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to save recipe: {}", e))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to create recipe from template: {}", e))),
        }
    }
}

impl Default for WebDriverServer {
    fn default() -> Self {
        Self::new().expect("Failed to create WebDriverServer with default config")
    }
}

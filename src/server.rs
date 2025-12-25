//! WebDriver MCP Server implementation
//!
//! This module contains the main server struct and the ServerHandler implementation
//! that dispatches MCP tool calls to the appropriate handler modules.

use rmcp::{ErrorData as McpError, ServerHandler, model::*};

use crate::{
    ClientManager,
    config::Config,
    handlers::{drivers, navigation, elements, page, performance, recipes},
    recipes::RecipeManager,
    tools::{ToolDefinitions, ServerMode},
};

/// The main WebDriver MCP server
#[derive(Clone)]
pub struct WebDriverServer {
    client_manager: ClientManager,
    recipe_manager: RecipeManager,
    mode: ServerMode,
}

impl WebDriverServer {
    /// Create a new server with default configuration
    pub fn new() -> crate::error::Result<Self> {
        let config = Config::from_env();
        Ok(Self {
            client_manager: ClientManager::new(config)?,
            recipe_manager: RecipeManager::new(None),
            mode: ServerMode::Stdio,
        })
    }

    /// Create a new server with custom configuration
    pub fn with_config(config: Config) -> crate::error::Result<Self> {
        Ok(Self {
            client_manager: ClientManager::new(config)?,
            recipe_manager: RecipeManager::new(None),
            mode: ServerMode::Stdio,
        })
    }

    /// Create a new server with custom configuration and mode
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
                        tracing::debug!("Successfully started all {} WebDrivers:", started_count);
                        for (driver_type, endpoint) in &started_drivers {
                            tracing::debug!("  {} → {}", driver_type.browser_name(), endpoint);
                        }
                    }

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

    /// Cleanup method to stop any managed driver processes
    pub async fn cleanup(&self) -> crate::error::Result<()> {
        tracing::info!("WebDriver MCP Server shutting down...");

        tracing::debug!("Closing active WebDriver sessions...");
        if let Err(e) = self.client_manager.close_all_sessions().await {
            tracing::warn!("Error closing WebDriver sessions: {}", e);
        } else {
            tracing::debug!("WebDriver sessions closed successfully");
        }

        tracing::debug!("Waiting for session cleanup to complete...");
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        tracing::debug!("Stopping WebDriver processes...");
        match self.client_manager.get_driver_manager().stop_all_drivers().await {
            Ok(()) => tracing::debug!("Successfully stopped all WebDriver processes"),
            Err(e) => {
                tracing::warn!("Error stopping WebDriver processes: {}", e);

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
}

impl Default for WebDriverServer {
    fn default() -> Self {
        Self::new().expect("Failed to create WebDriverServer with default config")
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
            // Navigation tools
            "navigate" => navigation::handle_navigate(&self.client_manager, &request.arguments).await,
            "get_current_url" => navigation::handle_get_current_url(&self.client_manager, &request.arguments).await,
            "back" => navigation::handle_back(&self.client_manager, &request.arguments).await,
            "forward" => navigation::handle_forward(&self.client_manager, &request.arguments).await,
            "refresh" => navigation::handle_refresh(&self.client_manager, &request.arguments).await,
            "get_page_load_status" => navigation::handle_get_page_load_status(&self.client_manager, &request.arguments).await,

            // Element tools
            "click" => elements::handle_click(&self.client_manager, &request.arguments).await,
            "send_keys" => elements::handle_send_keys(&self.client_manager, &request.arguments).await,
            "wait_for_element" => elements::handle_wait_for_element(&self.client_manager, &request.arguments).await,
            "wait_for_condition" => elements::handle_wait_for_condition(&self.client_manager, &request.arguments).await,
            "get_element_info" => elements::handle_get_element_info(&self.client_manager, &request.arguments).await,
            "get_attribute" => elements::handle_get_element_attribute(&self.client_manager, &request.arguments).await,
            "get_property" => elements::handle_get_element_property(&self.client_manager, &request.arguments).await,
            "find_element" => elements::handle_find_element(&self.client_manager, &request.arguments).await,
            "find_elements" => elements::handle_find_elements(&self.client_manager, &request.arguments).await,
            "scroll_to_element" => elements::handle_scroll_to_element(&self.client_manager, &request.arguments).await,
            "hover" => elements::handle_hover(&self.client_manager, &request.arguments).await,
            "fill_and_submit_form" => elements::handle_fill_and_submit_form(&self.client_manager, &request.arguments).await,
            "login_form" => elements::handle_login_form(&self.client_manager, &request.arguments).await,

            // Page tools
            "get_title" => page::handle_get_title(&self.client_manager, &request.arguments).await,
            "get_text" => page::handle_get_text(&self.client_manager, &request.arguments).await,
            "execute_script" => page::handle_execute_script(&self.client_manager, &request.arguments).await,
            "screenshot" => page::handle_screenshot(&self.client_manager, &request.arguments).await,
            "resize_window" => page::handle_resize_window(&self.client_manager, &request.arguments).await,
            "get_page_source" => page::handle_get_page_source(&self.client_manager, &request.arguments).await,

            // Performance tools
            "get_console_logs" => performance::handle_get_console_logs(&self.client_manager, &request.arguments).await,
            "get_performance_metrics" => performance::handle_get_performance_metrics(&self.client_manager, &request.arguments).await,
            "monitor_memory_usage" => performance::handle_monitor_memory_usage(&self.client_manager, &request.arguments).await,
            "run_performance_test" => performance::handle_run_performance_test(&self.client_manager, &request.arguments).await,
            "monitor_resource_usage" => performance::handle_monitor_resource_usage(&self.client_manager, &request.arguments).await,

            // Driver lifecycle tools (stdio mode only)
            "get_healthy_endpoints" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_get_healthy_endpoints(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "refresh_driver_health" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_refresh_driver_health(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "list_managed_drivers" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_list_managed_drivers(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "start_driver" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_start_driver(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "stop_driver" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_stop_driver(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "stop_all_drivers" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_stop_all_drivers(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }
            "force_cleanup_orphaned_processes" => {
                if self.mode == ServerMode::Stdio {
                    drivers::handle_force_cleanup_orphaned_processes(&self.client_manager, &request.arguments).await
                } else {
                    Err(McpError::method_not_found::<CallToolRequestMethod>())
                }
            }

            // Recipe management tools (available in both modes)
            "create_recipe" => recipes::handle_create_recipe(&self.recipe_manager, &request.arguments).await,
            "list_recipes" => recipes::handle_list_recipes(&self.recipe_manager, &request.arguments).await,
            "get_recipe" => recipes::handle_get_recipe(&self.recipe_manager, &request.arguments).await,
            "execute_recipe" => recipes::handle_execute_recipe(self, &self.recipe_manager, &request.arguments).await,
            "delete_recipe" => recipes::handle_delete_recipe(&self.recipe_manager, &request.arguments).await,
            "create_recipe_template" => recipes::handle_create_recipe_template(&self.recipe_manager, &request.arguments).await,

            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

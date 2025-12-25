use std::{collections::HashMap, sync::Arc, time::Duration};

use fantoccini::{Client, ClientBuilder, Locator, elements::Element};
use futures::lock::Mutex;

use crate::{config::Config, driver::DriverManager, error::Result};

#[derive(Clone)]
pub struct ClientManager {
    clients: Arc<Mutex<HashMap<String, Client>>>,
    config: Config,
    driver_manager: DriverManager,
}

impl ClientManager {
    pub fn new(config: Config) -> Result<Self> {
        config
            .validate()
            .map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;

        Ok(Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            config,
            driver_manager: DriverManager::new(),
        })
    }

    pub async fn get_or_create_client(
        &self,
        session_id: Option<String>,
    ) -> Result<(String, Client)> {
        // For stdio mode, use simplified single-session approach
        if self.is_stdio_mode() {
            return self.get_or_create_client_stdio(session_id).await;
        }
        
        // Full multi-tenant logic for HTTP mode
        self.get_or_create_client_http(session_id).await
    }

    /// Simplified client creation for stdio mode - single session per browser type
    async fn get_or_create_client_stdio(&self, session_id: Option<String>) -> Result<(String, Client)> {
        let session = session_id.unwrap_or_else(|| "stdio_default".to_string());
        
        let mut clients = self.clients.lock().await;
        if let Some(client) = clients.get(&session) {
            // Quick check if client is still alive
            match client.current_url().await {
                Ok(_) => return Ok((session, client.clone())),
                Err(_) => {
                    tracing::debug!("Existing stdio client is dead, removing");
                    clients.remove(&session);
                }
            }
        }
        drop(clients);

        // Use browser-aware endpoint resolution for stdio to support multi-browser recipes
        let endpoint = self.resolve_webdriver_endpoint_for_session(&session).await?;
        
        let client = self.create_configured_client(&endpoint, &session).await?;
        
        let mut clients = self.clients.lock().await; 
        clients.insert(session.clone(), client.clone());
        
        Ok((session, client))
    }

    /// Full multi-tenant client creation for HTTP mode
    async fn get_or_create_client_http(&self, session_id: Option<String>) -> Result<(String, Client)> {
        let mut clients = self.clients.lock().await;
        let session = session_id.unwrap_or_else(|| "default".to_string());

        if let Some(client) = clients.get(&session) {
            match client.current_url().await {
                Ok(_) => return Ok((session, client.clone())),
                Err(_) => {
                    clients.remove(&session);
                }
            }
        }

        // Determine the actual endpoint to use based on session preferences
        let endpoint = self.resolve_webdriver_endpoint_for_session(&session).await?;

        // Create client with proper browser configuration
        let client = self
            .create_configured_client(&endpoint, &session)
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to connect to WebDriver at '{}': {}\n\n{}",
                    endpoint,
                    e,
                    crate::config::Config::setup_guidance()
                )
            })?;

        clients.insert(session.clone(), client.clone());
        Ok((session, client))
    }

    async fn create_configured_client(&self, endpoint: &str, session_id: &str) -> Result<Client> {
        use serde_json::json;

        // Determine which browser we're connecting to based on session ID first, then fallback to port/preference
        let session_browser_preference = self.extract_browser_preference_from_session(session_id);
        
        let is_firefox = if let Some(driver_type) = session_browser_preference {
            matches!(driver_type, crate::driver::DriverType::Firefox)
        } else {
            endpoint.contains(":4444")
                || self.config.preferred_driver.as_ref().is_some_and(|p| {
                    p.to_lowercase().contains("firefox") || p.to_lowercase().contains("gecko")
                })
        };

        let driver_type = if is_firefox {
            crate::driver::DriverType::Firefox
        } else {
            crate::driver::DriverType::Chrome
        };

        // Create W3C compliant capabilities structure
        let mut caps = serde_json::Map::new();

        if is_firefox {
            // Firefox capabilities
            caps.insert("browserName".to_string(), json!("firefox"));

            if self.config.headless {
                let mut firefox_options = serde_json::Map::new();
                firefox_options.insert("args".to_string(), json!(["--headless"]));
                caps.insert("moz:firefoxOptions".to_string(), json!(firefox_options));
            }
        } else {
            // Chrome capabilities (default)
            caps.insert("browserName".to_string(), json!("chrome"));
            
            let mut chrome_options = serde_json::Map::new();
            let mut chrome_args = Vec::new();
            
            // Chrome args to fix DevToolsActivePort issues based on proven configurations
            chrome_args.extend([
                "--no-sandbox", 
                "--disable-dev-shm-usage",
                "--disable-gpu",
                "--remote-debugging-port=0"
            ].iter().map(|s| s.to_string()));
            
            if self.config.headless {
                chrome_args.push("--headless".to_string());
            }
            
            chrome_options.insert("args".to_string(), json!(chrome_args));
            caps.insert("goog:chromeOptions".to_string(), json!(chrome_options));
        }

        // Try to connect, if it fails due to session conflict, clean up and retry
        let client = ClientBuilder::native()
            .capabilities(caps.clone())
            .connect(endpoint)
            .await;

        match client {
            Ok(client) => Ok(client),
            Err(e) => {
                let error_msg = e.to_string().to_lowercase();
                if error_msg.contains("session is already started")
                    || error_msg.contains("session already exists")
                {
                    tracing::info!("Detected session conflict, attempting automatic cleanup...");

                    // Kill external drivers that might be causing conflicts
                    if let Err(cleanup_err) = self
                        .driver_manager
                        .kill_external_drivers(&driver_type)
                        .await
                    {
                        tracing::warn!("Failed to cleanup external drivers: {}", cleanup_err);
                    }

                    // Try connecting again
                    tracing::info!("Retrying connection after cleanup...");
                    ClientBuilder::native()
                        .capabilities(caps)
                        .connect(endpoint)
                        .await
                        .map_err(Into::into)
                } else {
                    Err(e.into())
                }
            }
        }
    }

    async fn resolve_webdriver_endpoint_for_session(&self, session_id: &str) -> Result<String> {
        // If endpoint is "auto", try to use pre-started drivers first
        if self.config.webdriver_endpoint == "auto" {
            // Check for healthy pre-started drivers
            let healthy_endpoints = self.driver_manager.get_healthy_endpoints().await;
            tracing::debug!("Available healthy endpoints: {:?}", healthy_endpoints);
            
            if !healthy_endpoints.is_empty() {
                // Check if session ID specifies a browser preference (e.g., "firefox_session1", "chrome_default")
                let preferred_driver = self.extract_browser_preference_from_session(session_id);
                
                // Try preferred driver from session ID first
                if let Some(driver_type) = preferred_driver {
                    if let Some(endpoint) = healthy_endpoints.get(&driver_type) {
                        return Ok(endpoint.clone());
                    } else {
                        tracing::warn!("Preferred {} driver for session '{}' not found in healthy endpoints: {:?}", 
                                      driver_type.browser_name(), session_id, healthy_endpoints.keys().collect::<Vec<_>>());
                        if self.config.auto_start_driver {
                            return self.start_driver_by_type(&driver_type).await.map_err(|e| crate::error::WebDriverError::Generic(e));
                        }
                    }
                }
                
                // Try preferred driver from config second
                if let Some(preferred) = &self.config.preferred_driver {
                    if let Some(driver_type) = crate::driver::DriverType::from_string(preferred) {
                        if let Some(endpoint) = healthy_endpoints.get(&driver_type) {
                            return Ok(endpoint.clone());
                        }
                    }
                }
                
                // Use any available healthy driver with deterministic ordering
                // Priority order: Chrome, Firefox, Edge
                let priority_order = [
                    crate::driver::DriverType::Chrome,
                    crate::driver::DriverType::Firefox, 
                    crate::driver::DriverType::Edge,
                ];
                
                for driver_type in &priority_order {
                    if let Some(endpoint) = healthy_endpoints.get(driver_type) {
                        return Ok(endpoint.clone());
                    }
                }
                
                // If none of the priority drivers are available, use any available (shouldn't happen)
                let (driver_type, endpoint) = healthy_endpoints.iter().next().unwrap();
                tracing::warn!("Using fallback {} driver at {} for session '{}' - this shouldn't happen", driver_type.browser_name(), endpoint, session_id);
                return Ok(endpoint.clone());
            }
            
            // Fall back to reactive startup if no pre-started drivers available
            if self.config.auto_start_driver {
                tracing::debug!("No pre-started drivers available, falling back to reactive startup for session '{}'", session_id);

                // Try to auto-start the preferred driver or the first available one
                let endpoint = if let Some(preferred) = &self.config.preferred_driver {
                    self.start_preferred_driver(preferred).await?
                } else {
                    self.start_any_available_driver().await?
                };

                tracing::debug!("Successfully started WebDriver at {} for session '{}'", endpoint, session_id);
                Ok(endpoint)
            } else {
                Err(anyhow::anyhow!(
                    "No pre-started WebDriver services available and auto_start_driver is disabled. \
                     Enable auto_start_driver or manually start a WebDriver service."
                ).into())
            }
        } else {
            // Use configured endpoint as-is
            Ok(self.config.webdriver_endpoint.clone())
        }
    }

    /// Extract browser preference from session ID (e.g., "firefox_session1" -> Some(DriverType::Firefox))
    fn extract_browser_preference_from_session(&self, session_id: &str) -> Option<crate::driver::DriverType> {
        let session_lower = session_id.to_lowercase();
        
        if session_lower.starts_with("firefox") || session_lower.starts_with("gecko") {
            Some(crate::driver::DriverType::Firefox)
        } else if session_lower.starts_with("chrome") || session_lower.starts_with("chromium") {
            Some(crate::driver::DriverType::Chrome)
        } else if session_lower.starts_with("edge") {
            Some(crate::driver::DriverType::Edge)
        } else {
            None
        }
    }


    /// Check if we're running in stdio mode (simple heuristic)
    fn is_stdio_mode(&self) -> bool {
        // Simple detection: if we're using auto endpoint and auto_start_driver is true, likely stdio mode
        // In practice, you might want to pass mode explicitly to ClientManager
        self.config.webdriver_endpoint == "auto" && self.config.auto_start_driver
    }

    /// Simple endpoint resolution for stdio mode - no complex session logic
    /// 

    async fn start_driver_by_type(&self, driver_type: &crate::driver::DriverType) -> anyhow::Result<String> {
        
        // Attempt to start the driver
        match self.driver_manager.start_driver_manually(driver_type.clone()).await {
            Ok(actual_endpoint) => Ok(actual_endpoint),
            Err(e) => Err(anyhow::anyhow!("Failed to start {} driver: {}", driver_type.browser_name(), e))
        }
    }

    async fn start_preferred_driver(&self, preferred: &str) -> Result<String> {
        let endpoint = match preferred.to_lowercase().as_str() {
            "chrome" | "chromium" => "http://localhost:9515",
            "firefox" | "gecko" => "http://localhost:4444",
            "edge" => "http://localhost:9515",
            _ => return Err(anyhow::anyhow!("Unknown preferred driver: {}", preferred).into()),
        };

        self.driver_manager.auto_start_for_endpoint(endpoint).await
    }

    async fn start_any_available_driver(&self) -> Result<String> {
        let available_drivers = self.driver_manager.detect_available_drivers();

        if available_drivers.is_empty() {
            return Err(anyhow::anyhow!(
                "No WebDriver executables found. Please install ChromeDriver, GeckoDriver, or EdgeDriver.\n\n{}",
                crate::config::Config::setup_guidance()
            ).into());
        }

        // Try to start the first available driver
        let (driver_type, _) = &available_drivers[0];
        let endpoint = format!("http://localhost:{}", driver_type.default_port());

        self.driver_manager.auto_start_for_endpoint(&endpoint).await
    }

    /// Get access to the driver manager for lifecycle operations
    pub fn get_driver_manager(&self) -> &DriverManager {
        &self.driver_manager
    }

    /// Get access to the configuration
    pub fn get_config(&self) -> &Config {
        &self.config
    }

    /// Close all active WebDriver sessions
    pub async fn close_all_sessions(&self) -> Result<()> {
        tracing::info!("Closing all active WebDriver sessions...");
        let mut clients = self.clients.lock().await;
        
        for (session_id, client) in clients.drain() {
            tracing::debug!("Closing session: {}", session_id);
            
            // Add timeout to individual session close operations
            let close_timeout = Duration::from_secs(2);
            match tokio::time::timeout(close_timeout, client.close()).await {
                Ok(Ok(())) => tracing::debug!("Successfully closed session: {}", session_id),
                Ok(Err(e)) => tracing::warn!("Error closing session {}: {}", session_id, e),
                Err(_) => {
                    tracing::warn!("Timeout closing session {} after {:?}, forcing cleanup", session_id, close_timeout);
                    // Session close timed out, but continue with other sessions
                }
            }
        }
        
        // CRITICAL FIX: Force cleanup of orphaned browser processes
        self.force_cleanup_orphaned_processes().await?;
        
        // Also cleanup all managed processes through driver manager
        self.driver_manager.force_cleanup_all_processes().await?;
        
        tracing::info!("All WebDriver sessions closed and orphaned processes cleaned");
        Ok(())
    }

    /// Force cleanup of orphaned browser processes that may be consuming resources
    async fn force_cleanup_orphaned_processes(&self) -> Result<()> {
        tracing::info!("🧹 Force cleaning orphaned browser and WebDriver processes...");
        
        // First kill all driver processes to prevent new browser spawns
        let driver_cleanup_commands = [
            ("chromedriver processes", "pkill -f chromedriver"),
            ("geckodriver processes", "pkill -f geckodriver"), 
            ("msedgedriver processes", "pkill -f msedgedriver"),
        ];
        
        for (description, command) in driver_cleanup_commands {
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        tracing::debug!("✅ Cleaned up {}", description);
                    } else {
                        tracing::debug!("ℹ️  No {} to clean up", description);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup {}: {}", description, e);
                }
            }
        }
        
        // Wait for drivers to shutdown gracefully
        tokio::time::sleep(Duration::from_millis(1000)).await;
        
        // Now kill all browser processes (more comprehensive patterns)
        let browser_cleanup_commands = [
            ("firefox headless processes", "pkill -f 'firefox.*headless'"),
            ("chrome headless processes", "pkill -f 'chrome.*headless'"), 
            ("chromium headless processes", "pkill -f 'chromium.*headless'"),
            ("firefox marionette processes", "pkill -f 'firefox.*marionette'"),
            ("chrome webdriver processes", "pkill -f 'chrome.*webdriver'"),
            ("chromium webdriver processes", "pkill -f 'chromium.*webdriver'"),
            ("chrome crashpad handlers", "pkill -f chrome_crashpad_handler"),
            ("chrome zygote processes", "pkill -f 'chrome.*zygote'"),
            ("chrome utility processes", "pkill -f 'chrome.*utility'"),
            ("chrome gpu processes", "pkill -f 'chrome.*gpu-process'"),
            ("chrome renderer processes", "pkill -f 'chrome.*renderer'"),
        ];
        
        for (description, command) in browser_cleanup_commands {
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        tracing::debug!("✅ Cleaned up {}", description);
                    } else {
                        tracing::debug!("ℹ️  No {} to clean up", description);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup {}: {}", description, e);
                }
            }
        }
        
        // Final cleanup: use more aggressive patterns to catch any remaining processes
        let aggressive_cleanup_commands = [
            ("remaining browser processes", "ps aux | grep -E '(chrome|firefox|chromium)' | grep -E '(headless|webdriver|marionette)' | grep -v grep | awk '{print $2}' | xargs -r kill -9"),
        ];
        
        for (description, command) in aggressive_cleanup_commands {
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
            {
                Ok(output) => {
                    if output.status.success() {
                        tracing::debug!("✅ Aggressive cleanup of {}", description);
                    } else {
                        tracing::debug!("ℹ️  No {} for aggressive cleanup", description);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed aggressive cleanup of {}: {}", description, e);
                }
            }
        }
        
        // Clean up temporary browser data directories
        self.cleanup_temp_browser_data().await?;
        
        Ok(())
    }
    
    /// Clean up temporary browser data directories created by WebDriver sessions
    async fn cleanup_temp_browser_data(&self) -> Result<()> {
        tracing::debug!("🗂️  Cleaning up temporary browser data directories...");
        
        let temp_cleanup_commands = [
            ("chromium temp directories", "find /tmp -name '.org.chromium.Chromium.*' -type d -exec rm -rf {} + 2>/dev/null || true"),
            ("firefox temp directories", "find /tmp -name 'rust_mozprofile*' -type d -exec rm -rf {} + 2>/dev/null || true"),
            ("webdriver temp files", "find /tmp -name 'webdriver-*' -exec rm -rf {} + 2>/dev/null || true"),
        ];
        
        for (description, command) in temp_cleanup_commands {
            match tokio::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
            {
                Ok(_) => {
                    tracing::debug!("✅ Cleaned up {}", description);
                }
                Err(e) => {
                    tracing::warn!("Failed to cleanup {}: {}", description, e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Public method to force cleanup of all orphaned browser and WebDriver processes
    /// This is useful for emergency cleanup when sessions are misbehaving
    pub async fn force_cleanup_orphaned_processes_public(&self) -> Result<()> {
        tracing::info!("🚨 Manual force cleanup of all orphaned processes requested");
        
        // Use both cleanup methods for comprehensive cleanup
        self.force_cleanup_orphaned_processes().await?;
        self.driver_manager.force_cleanup_all_processes().await?;
        
        tracing::info!("✅ Manual force cleanup completed");
        Ok(())
    }

    pub async fn find_element_with_wait(
        &self,
        client: &Client,
        selector: &str,
        wait_timeout: Option<f64>,
    ) -> Result<Element> {
        let locator = Locator::Css(selector);

        if let Some(timeout_seconds) = wait_timeout {
            if timeout_seconds > 0.0 {
                let timeout = Duration::from_secs_f64(timeout_seconds);
                return client
                    .wait()
                    .at_most(timeout)
                    .for_element(locator)
                    .await
                    .map_err(Into::into);
            }
        }

        client.find(locator).await.map_err(Into::into)
    }
}

// Note: Default is intentionally not implemented for ClientManager
// because ClientManager::new() can fail if configuration validation fails.
// Use ClientManager::new(Config::from_env()) with proper error handling instead.

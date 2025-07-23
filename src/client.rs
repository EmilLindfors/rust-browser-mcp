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

        // Determine the actual endpoint to use
        let endpoint = self.resolve_webdriver_endpoint().await?;

        // Create client with proper browser configuration
        let client = self
            .create_configured_client(&endpoint)
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

    async fn create_configured_client(&self, endpoint: &str) -> Result<Client> {
        use serde_json::json;

        // Determine which browser we're connecting to based on port/preference
        let is_firefox = endpoint.contains(":4444")
            || self.config.preferred_driver.as_ref().is_some_and(|p| {
                p.to_lowercase().contains("firefox") || p.to_lowercase().contains("gecko")
            });

        let driver_type = if is_firefox {
            crate::driver::DriverType::Firefox
        } else {
            crate::driver::DriverType::Chrome
        };

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

            if self.config.headless {
                let mut chrome_options = serde_json::Map::new();
                chrome_options.insert(
                    "args".to_string(),
                    json!([
                        "--headless",
                        "--no-sandbox",
                        "--disable-dev-shm-usage",
                        "--disable-gpu"
                    ]),
                );
                caps.insert("goog:chromeOptions".to_string(), json!(chrome_options));
            }
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

    async fn resolve_webdriver_endpoint(&self) -> Result<String> {
        // If endpoint is "auto", try to auto-start a driver
        if self.config.webdriver_endpoint == "auto" && self.config.auto_start_driver {
            tracing::info!("Auto-detecting and starting WebDriver service...");

            // Try to auto-start the preferred driver or the first available one
            let endpoint = if let Some(preferred) = &self.config.preferred_driver {
                self.start_preferred_driver(preferred).await?
            } else {
                self.start_any_available_driver().await?
            };

            tracing::info!("Successfully started WebDriver at: {}", endpoint);
            Ok(endpoint)
        } else {
            // Use configured endpoint as-is
            Ok(self.config.webdriver_endpoint.clone())
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

impl Default for ClientManager {
    fn default() -> Self {
        Self::new(Config::from_env()).expect("Failed to create ClientManager with default config")
    }
}

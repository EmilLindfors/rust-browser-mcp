use std::env;

#[derive(Clone)]
pub struct Config {
    pub webdriver_endpoint: String,
    pub default_session_timeout_ms: u64,
    pub auto_start_driver: bool,
    pub preferred_driver: Option<String>,
    pub headless: bool,
    /// List of drivers to start concurrently at server startup
    pub concurrent_drivers: Vec<String>,
    /// Timeout for driver startup in milliseconds
    pub driver_startup_timeout_ms: u64,
    /// Enable Chrome performance memory APIs
    pub enable_performance_memory: bool,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            webdriver_endpoint: env::var("WEBDRIVER_ENDPOINT")
                .unwrap_or_else(|_| "auto".to_string()), // Default to "auto" for auto-detection
            default_session_timeout_ms: env::var("WEBDRIVER_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2000),
            auto_start_driver: env::var("WEBDRIVER_AUTO_START")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(true), // Default to true for auto-start
            preferred_driver: env::var("WEBDRIVER_PREFERRED_DRIVER").ok(),
            headless: env::var("WEBDRIVER_HEADLESS")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(true), // Default to true for headless mode
            concurrent_drivers: env::var("WEBDRIVER_CONCURRENT_DRIVERS")
                .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_else(|_| vec!["firefox".to_string(), "chrome".to_string()]), // Default to both
            driver_startup_timeout_ms: env::var("WEBDRIVER_STARTUP_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10000), // Default to 10 seconds
            enable_performance_memory: env::var("WEBDRIVER_ENABLE_PERFORMANCE_MEMORY")
                .map(|v| v.to_lowercase() == "true" || v == "1")
                .unwrap_or(false), // Default to false for compatibility
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Basic validation of the WebDriver endpoint URL
        if self.webdriver_endpoint != "auto"
            && !self.webdriver_endpoint.starts_with("http://")
            && !self.webdriver_endpoint.starts_with("https://")
        {
            return Err(format!(
                "Invalid WebDriver endpoint '{}'. Must be 'auto' or start with http:// or https://",
                self.webdriver_endpoint
            ));
        }

        if self.default_session_timeout_ms == 0 {
            return Err("WebDriver timeout must be greater than 0".to_string());
        }

        if self.driver_startup_timeout_ms == 0 {
            return Err("Driver startup timeout must be greater than 0".to_string());
        }

        // Validate concurrent drivers list
        let valid_drivers = ["firefox", "chrome", "edge"];
        for driver in &self.concurrent_drivers {
            if !valid_drivers.contains(&driver.to_lowercase().as_str()) {
                return Err(format!(
                    "Invalid concurrent driver '{}'. Must be one of: {}",
                    driver,
                    valid_drivers.join(", ")
                ));
            }
        }

        Ok(())
    }

    pub fn setup_guidance() -> &'static str {
        r#"
WebDriver MCP Server Setup:

🚀 AUTOMATIC SETUP (Recommended):
   The server will automatically detect and start WebDriver services!
   Just install one of the drivers below and run the server.

1. Install a WebDriver (choose one):
   
   ChromeDriver:
   - macOS: brew install chromedriver
   - Ubuntu: sudo apt-get install chromium-chromedriver
   - Windows: choco install chromedriver
   - Manual: https://chromedriver.chromium.org/

   GeckoDriver (Firefox):
   - macOS: brew install geckodriver
   - Ubuntu: sudo apt-get install firefox-geckodriver
   - Windows: choco install geckodriver
   - Manual: https://github.com/mozilla/geckodriver/releases

   EdgeDriver:
   - Download from: https://developer.microsoft.com/microsoft-edge/tools/webdriver/

2. Environment Variables (all optional):
   - WEBDRIVER_ENDPOINT: 'auto' (default) or specific URL
   - WEBDRIVER_AUTO_START: true (default) or false
   - WEBDRIVER_PREFERRED_DRIVER: chrome, firefox, or edge
   - WEBDRIVER_TIMEOUT_MS: Connection timeout in ms (default: 2000)
   - WEBDRIVER_HEADLESS: true (default) or false for GUI mode
   - WEBDRIVER_CONCURRENT_DRIVERS: comma-separated list (default: firefox,chrome)
   - WEBDRIVER_STARTUP_TIMEOUT_MS: Driver startup timeout (default: 10000)
   - WEBDRIVER_ENABLE_PERFORMANCE_MEMORY: true or false (default: false) - enables Chrome memory APIs

3. Manual Setup (if auto-start disabled):
   - Chrome: chromedriver --port=9515
   - Firefox: geckodriver --port=4444
   - Set WEBDRIVER_ENDPOINT to the appropriate URL
"#
    }
}

use std::env;

#[derive(Clone)]
pub struct Config {
    pub webdriver_endpoint: String,
    pub default_session_timeout_ms: u64,
    pub auto_start_driver: bool,
    pub preferred_driver: Option<String>,
    pub headless: bool,
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

3. Manual Setup (if auto-start disabled):
   - Chrome: chromedriver --port=9515
   - Firefox: geckodriver --port=4444
   - Set WEBDRIVER_ENDPOINT to the appropriate URL
"#
    }
}

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use tokio::{process::Child as TokioChild, time::sleep};
use tracing::{debug, info, warn};

use crate::error::{Result, WebDriverError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DriverType {
    Chrome,
    Firefox,
    Edge,
}

impl DriverType {
    pub fn from_string(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "chrome" | "chromium" => Some(DriverType::Chrome),
            "firefox" | "gecko" => Some(DriverType::Firefox),
            "edge" => Some(DriverType::Edge),
            _ => None,
        }
    }

    pub fn executable_name(&self) -> &'static str {
        match self {
            DriverType::Chrome => {
                if cfg!(windows) {
                    "chromedriver.exe"
                } else {
                    "chromedriver"
                }
            }
            DriverType::Firefox => {
                if cfg!(windows) {
                    "geckodriver.exe"
                } else {
                    "geckodriver"
                }
            }
            DriverType::Edge => {
                if cfg!(windows) {
                    "msedgedriver.exe"
                } else {
                    "msedgedriver"
                }
            }
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            DriverType::Chrome => 9515,
            DriverType::Firefox => 4444,
            DriverType::Edge => 9515,
        }
    }

    pub fn browser_name(&self) -> &'static str {
        match self {
            DriverType::Chrome => "Chrome",
            DriverType::Firefox => "Firefox",
            DriverType::Edge => "Edge",
        }
    }
}

#[derive(Clone)]
pub struct DriverManager {
    running_processes: Arc<Mutex<Vec<ManagedProcess>>>,
    /// Map of driver type to endpoint URL for healthy running drivers
    healthy_endpoints: Arc<Mutex<HashMap<DriverType, String>>>,
}

struct ManagedProcess {
    driver_type: DriverType,
    process: TokioChild,
    port: u16,
    pid: u32,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            running_processes: Arc::new(Mutex::new(Vec::new())),
            healthy_endpoints: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start multiple WebDriver processes concurrently 
    pub async fn start_concurrent_drivers(
        &self,
        driver_names: &[String],
        timeout: Duration,
    ) -> Result<Vec<(DriverType, String)>> {
        info!("Starting concurrent WebDriver processes: {:?}", driver_names);
        
        let mut handles = Vec::new();

        // Start all requested drivers concurrently
        for driver_name in driver_names {
            if let Some(driver_type) = DriverType::from_string(driver_name) {
                let driver_manager = self.clone();
                let driver_type_clone = driver_type.clone();
                
                let handle = tokio::spawn(async move {
                    let result = driver_manager.start_single_driver(driver_type_clone).await;
                    (driver_type, result)
                });
                handles.push(handle);
            } else {
                warn!("Unknown driver type '{}', skipping", driver_name);
            }
        }

        // Wait for all drivers to start with timeout
        let timeout_result = tokio::time::timeout(timeout, async {
            let mut results = Vec::new();
            for handle in handles {
                let (driver_type, result) = handle.await.unwrap_or_else(|e| {
                    warn!("Driver startup task failed: {}", e);
                    (DriverType::Chrome, Err(WebDriverError::Session("Task failed".to_string())))
                });

                match result {
                    Ok(endpoint) => {
                        info!(
                            "Successfully started {} at {}",
                            driver_type.browser_name(),
                            endpoint
                        );
                        
                        // Mark as healthy
                        {
                            let mut healthy = self.healthy_endpoints.lock().unwrap();
                            healthy.insert(driver_type.clone(), endpoint.clone());
                        }
                        
                        results.push((driver_type, endpoint));
                    }
                    Err(e) => {
                        warn!(
                            "Failed to start {}: {}",
                            driver_type.browser_name(),
                            e
                        );
                    }
                }
            }
            results
        }).await;

        match timeout_result {
            Ok(results) => {
                info!("Concurrent driver startup completed. {} drivers running", results.len());
                Ok(results)
            }
            Err(_) => {
                warn!("Driver startup timed out after {:?}", timeout);
                // Return empty vec since we can't access partial results after timeout
                Ok(Vec::new())
            }
        }
    }

    /// Start a single WebDriver process
    async fn start_single_driver(&self, driver_type: DriverType) -> Result<String> {
        let driver_path = self.find_driver_executable(&driver_type).ok_or_else(|| {
            WebDriverError::Session(format!(
                "{} executable not found in PATH",
                driver_type.executable_name()
            ))
        })?;

        let port = driver_type.default_port();
        let endpoint = format!("http://localhost:{port}");

        // Check if already running on this port
        if self.is_service_running(port).await {
            info!("{} already running on port {}", driver_type.browser_name(), port);
            return Ok(endpoint);
        }

        info!(
            "Starting {} on port {}",
            driver_type.browser_name(),
            port
        );
        
        self.start_driver(&driver_type, &driver_path, port).await?;

        // Wait for service to be ready
        self.wait_for_service_ready(&endpoint, Duration::from_secs(10))
            .await?;

        Ok(endpoint)
    }

    /// Get all healthy endpoints
    pub fn get_healthy_endpoints(&self) -> HashMap<DriverType, String> {
        let healthy = self.healthy_endpoints.lock().unwrap();
        healthy.clone()
    }

    /// Check if a specific driver type is healthy
    pub fn is_driver_healthy(&self, driver_type: &DriverType) -> bool {
        let healthy = self.healthy_endpoints.lock().unwrap();
        healthy.contains_key(driver_type)
    }

    /// Perform health checks on all running drivers and update healthy_endpoints
    pub async fn refresh_driver_health(&self) -> Result<()> {
        let mut healthy_endpoints_updated = HashMap::new();
        
        // Get current running processes to check their health
        let processes = {
            let processes = self.running_processes.lock().unwrap();
            processes.iter().map(|p| (p.driver_type.clone(), p.port)).collect::<Vec<_>>()
        };

        for (driver_type, port) in processes {
            if self.is_service_running(port).await {
                let endpoint = format!("http://localhost:{}", port);
                healthy_endpoints_updated.insert(driver_type.clone(), endpoint);
                debug!("Health check passed for {} on port {}", driver_type.browser_name(), port);
            } else {
                warn!("Health check failed for {} on port {}", driver_type.browser_name(), port);
            }
        }

        // Update healthy endpoints atomically
        {
            let mut healthy = self.healthy_endpoints.lock().unwrap();
            *healthy = healthy_endpoints_updated;
        }

        Ok(())
    }

    /// Start a periodic health check task (returns a handle to cancel it)
    pub fn start_periodic_health_checks(&self, interval: Duration) -> tokio::task::JoinHandle<()> {
        let driver_manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = driver_manager.refresh_driver_health().await {
                    warn!("Health check failed: {}", e);
                }
            }
        })
    }

    /// Auto-detect available WebDriver executables on the system
    pub fn detect_available_drivers(&self) -> Vec<(DriverType, PathBuf)> {
        let driver_types = [DriverType::Chrome, DriverType::Firefox, DriverType::Edge];
        let mut available = Vec::new();

        for driver_type in &driver_types {
            if let Some(path) = self.find_driver_executable(driver_type) {
                debug!("Found {} at: {:?}", driver_type.browser_name(), path);
                available.push((driver_type.clone(), path));
            }
        }

        available
    }

    /// Find the executable path for a specific driver type
    fn find_driver_executable(&self, driver_type: &DriverType) -> Option<PathBuf> {
        let exe_name = driver_type.executable_name();

        // First, check if it's in PATH
        let which_cmd = if cfg!(windows) { "where" } else { "which" };
        if let Ok(output) = Command::new(which_cmd).arg(exe_name).output() {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let path_str = output_str.trim();
                if !path_str.is_empty() {
                    let first_path = path_str.lines().next().unwrap_or("");
                    if !first_path.is_empty() {
                        return Some(PathBuf::from(first_path));
                    }
                }
            }
        }

        // Check common installation paths
        let common_paths = self.get_common_driver_paths(driver_type);
        common_paths.into_iter().find(|path| path.exists())
    }

    /// Get common installation paths for different driver types
    fn get_common_driver_paths(&self, driver_type: &DriverType) -> Vec<PathBuf> {
        let exe_name = driver_type.executable_name();
        let mut paths = Vec::new();

        if cfg!(target_os = "macos") {
            paths.extend([
                PathBuf::from("/usr/local/bin").join(exe_name),
                PathBuf::from("/opt/homebrew/bin").join(exe_name),
                PathBuf::from(format!(
                    "/Applications/{}.app/Contents/MacOS/{}",
                    driver_type.browser_name(),
                    exe_name
                )),
            ]);
        } else if cfg!(windows) {
            paths.extend([
                PathBuf::from(format!(
                    "C:\\Program Files\\{}\\{}",
                    driver_type.browser_name(),
                    exe_name
                )),
                PathBuf::from(format!(
                    "C:\\Program Files (x86)\\{}\\{}",
                    driver_type.browser_name(),
                    exe_name
                )),
                PathBuf::from("C:\\WebDrivers").join(exe_name),
            ]);
        } else {
            // Linux
            paths.extend([
                PathBuf::from("/usr/bin").join(exe_name),
                PathBuf::from("/usr/local/bin").join(exe_name),
                PathBuf::from("/snap/bin").join(exe_name),
            ]);
        }

        paths
    }

    /// Auto-start a WebDriver service based on endpoint URL
    pub async fn auto_start_for_endpoint(&self, endpoint: &str) -> Result<String> {
        info!(
            "Attempting to auto-start WebDriver for endpoint: {}",
            endpoint
        );

        // Extract port from endpoint
        let port = self.extract_port_from_endpoint(endpoint)?;

        // Determine which driver to use based on port or configuration
        let driver_type = self.determine_driver_type(port);

        // Check if already running
        if self.is_service_running(port).await {
            info!("WebDriver service already running on port {}", port);
            return Ok(format!("http://localhost:{port}"));
        }

        // Find and start the appropriate driver
        if let Some(driver_path) = self.find_driver_executable(&driver_type) {
            info!(
                "Starting {} from: {:?}",
                driver_type.browser_name(),
                driver_path
            );

            let actual_endpoint = self.start_driver(&driver_type, &driver_path, port).await?;

            // Wait for service to be ready
            self.wait_for_service_ready(&actual_endpoint, Duration::from_secs(10))
                .await?;

            Ok(actual_endpoint)
        } else {
            Err(WebDriverError::Session(format!(
                "Could not find {} executable. Please install {} or set a custom WEBDRIVER_ENDPOINT.\n\n{}",
                driver_type.executable_name(),
                driver_type.browser_name(),
                self.installation_guidance(&driver_type)
            )))
        }
    }

    fn extract_port_from_endpoint(&self, endpoint: &str) -> Result<u16> {
        if let Some(port_start) = endpoint.rfind(':') {
            let port_str = &endpoint[port_start + 1..];
            // Remove any path components
            let port_str = port_str.split('/').next().unwrap_or(port_str);

            port_str.parse().map_err(|_| {
                WebDriverError::Session(format!("Invalid port in endpoint: {endpoint}"))
            })
        } else {
            // Default ports based on protocol
            if endpoint.contains("https") {
                Ok(443)
            } else {
                Ok(80)
            }
        }
    }

    fn determine_driver_type(&self, port: u16) -> DriverType {
        match port {
            4444 => DriverType::Firefox,
            9515 => DriverType::Chrome,
            _ => {
                // Default to Chrome if we can't determine
                warn!("Unknown port {}, defaulting to Chrome", port);
                DriverType::Chrome
            }
        }
    }

    async fn is_service_running(&self, port: u16) -> bool {
        // Try to connect to the service
        let endpoint = format!("http://localhost:{port}/status");

        match reqwest::Client::new()
            .get(&endpoint)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }

    async fn start_driver(
        &self,
        driver_type: &DriverType,
        driver_path: &Path,
        port: u16,
    ) -> Result<String> {
        let mut command = tokio::process::Command::new(driver_path);

        match driver_type {
            DriverType::Chrome => {
                command
                    .arg(format!("--port={port}"))
                    .arg("--whitelisted-ips=127.0.0.1")
                    .arg("--log-level=INFO");
            }
            DriverType::Firefox => {
                command
                    .arg("--port")
                    .arg(port.to_string())
                    .arg("--host")
                    .arg("127.0.0.1");
            }
            DriverType::Edge => {
                command
                    .arg(format!("--port={port}"))
                    .arg("--whitelisted-ips=127.0.0.1");
            }
        }

        // Redirect stdout/stderr to avoid blocking
        let process = command
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                WebDriverError::Session(format!(
                    "Failed to start {}: {}",
                    driver_type.browser_name(),
                    e
                ))
            })?;

        let pid = process.id().ok_or_else(|| {
            WebDriverError::Session(format!(
                "Failed to get PID for {}",
                driver_type.browser_name()
            ))
        })?;

        // Store the process for cleanup
        {
            let mut processes = self.running_processes.lock().unwrap();
            processes.push(ManagedProcess {
                driver_type: driver_type.clone(),
                process,
                pid,
                port,
            });
        }

        Ok(format!("http://localhost:{port}"))
    }

    async fn wait_for_service_ready(&self, endpoint: &str, timeout: Duration) -> Result<()> {
        let status_endpoint = format!("{endpoint}/status");
        let client = reqwest::Client::new();
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match client
                .get(&status_endpoint)
                .timeout(Duration::from_secs(1))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    info!("WebDriver service ready at: {}", endpoint);
                    return Ok(());
                }
                _ => {
                    debug!("Waiting for WebDriver service...");
                    sleep(Duration::from_millis(250)).await;
                }
            }
        }

        Err(WebDriverError::Session(format!(
            "WebDriver service did not become ready within {timeout:?}"
        )))
    }

    fn installation_guidance(&self, driver_type: &DriverType) -> String {
        match driver_type {
            DriverType::Chrome => {
                r#"ChromeDriver Installation:
1. Download from: https://chromedriver.chromium.org/
2. Package managers:
   - macOS: brew install chromedriver
   - Ubuntu: sudo apt-get install chromium-chromedriver
   - Windows: choco install chromedriver
3. Place in PATH or set WEBDRIVER_ENDPOINT to custom location"#
            }
            DriverType::Firefox => {
                r#"GeckoDriver Installation:
1. Download from: https://github.com/mozilla/geckodriver/releases
2. Package managers:
   - macOS: brew install geckodriver
   - Ubuntu: sudo apt-get install firefox-geckodriver
   - Windows: choco install geckodriver
3. Place in PATH or set WEBDRIVER_ENDPOINT=http://localhost:4444"#
            }
            DriverType::Edge => {
                r#"EdgeDriver Installation:
1. Download from: https://developer.microsoft.com/en-us/microsoft-edge/tools/webdriver/
2. Install matching your Edge version
3. Place in PATH or set WEBDRIVER_ENDPOINT to custom location"#
            }
        }
        .to_string()
    }

    /// Stop all managed driver processes
    pub async fn stop_all_drivers(&self) -> Result<()> {
        let mut processes = self.running_processes.lock().unwrap();

        for managed_process in processes.iter_mut() {
            info!(
                "Stopping {} driver (PID: {}) on port {}",
                managed_process.driver_type.browser_name(),
                managed_process.pid,
                managed_process.port
            );

            // Use start_kill for immediate termination to avoid async issues
            if let Err(e) = managed_process.process.start_kill() {
                warn!(
                    "Failed to kill {} process: {}",
                    managed_process.driver_type.browser_name(),
                    e
                );
            }
        }

        processes.clear();
        
        // Clear healthy endpoints
        {
            let mut healthy = self.healthy_endpoints.lock().unwrap();
            healthy.clear();
        }
        
        Ok(())
    }

    /// Start a specific WebDriver process manually
    pub async fn start_driver_manually(&self, driver_type: DriverType) -> Result<String> {
        let driver_path = self.find_driver_executable(&driver_type).ok_or_else(|| {
            WebDriverError::Session(format!(
                "{} executable not found in PATH",
                driver_type.executable_name()
            ))
        })?;

        let port = driver_type.default_port();
        let endpoint = format!("http://localhost:{port}");

        // Check if already running on this port
        if self.is_service_running(port).await {
            return Ok(endpoint);
        }

        info!(
            "Starting {} manually on port {}",
            driver_type.browser_name(),
            port
        );
        self.start_driver(&driver_type, &driver_path, port).await?;

        // Wait for service to be ready
        self.wait_for_service_ready(&endpoint, Duration::from_secs(10))
            .await?;

        Ok(endpoint)
    }

    /// Stop a specific type of WebDriver process
    pub async fn stop_driver_by_type(&self, driver_type: &DriverType) -> Result<()> {
        let mut indices_to_remove = Vec::new();

        {
            let mut processes = self.running_processes.lock().unwrap();
            for (i, managed_process) in processes.iter_mut().enumerate() {
                if &managed_process.driver_type == driver_type {
                    info!(
                        "Stopping {} driver (PID: {})",
                        driver_type.browser_name(),
                        managed_process.pid
                    );

                    // Use start_kill for immediate termination
                    if let Err(e) = managed_process.process.start_kill() {
                        warn!(
                            "Failed to kill {} process: {}",
                            driver_type.browser_name(),
                            e
                        );
                    }
                    indices_to_remove.push(i);
                }
            }

            // Remove stopped processes from the list
            for &i in indices_to_remove.iter().rev() {
                processes.remove(i);
            }
        }

        // Remove from healthy endpoints
        {
            let mut healthy = self.healthy_endpoints.lock().unwrap();
            healthy.remove(driver_type);
        }

        Ok(())
    }

    /// Get status of all managed processes
    pub fn get_managed_processes_status(&self) -> Vec<(DriverType, u32, u16)> {
        let processes = self.running_processes.lock().unwrap();
        processes
            .iter()
            .map(|p| (p.driver_type.clone(), p.pid, p.port))
            .collect()
    }

    /// Check if a specific driver type is currently managed
    pub fn is_driver_managed(&self, driver_type: &DriverType) -> bool {
        let processes = self.running_processes.lock().unwrap();
        processes.iter().any(|p| &p.driver_type == driver_type)
    }

    /// Kill external WebDriver processes that might be conflicting
    pub async fn kill_external_drivers(&self, driver_type: &DriverType) -> Result<()> {
        let executable_name = driver_type.executable_name();

        info!(
            "Searching for external {} processes to kill...",
            executable_name
        );

        #[cfg(windows)]
        {
            self.kill_external_drivers_windows(executable_name).await
        }

        #[cfg(unix)]
        {
            self.kill_external_drivers_unix(executable_name).await
        }
    }

    #[cfg(windows)]
    async fn kill_external_drivers_windows(&self, executable_name: &str) -> Result<()> {
        // Use tasklist to find processes by image name
        let output = Command::new("tasklist")
            .arg("/FI")
            .arg(&format!("IMAGENAME eq {}", executable_name))
            .arg("/FO")
            .arg("CSV")
            .arg("/NH")
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let processes = String::from_utf8_lossy(&output.stdout);
                for line in processes.lines() {
                    if line.trim().is_empty() {
                        continue;
                    }

                    // Parse CSV format: "Image Name","PID","Session Name","Session#","Mem Usage"
                    let fields: Vec<&str> = line.split(',').collect();
                    if fields.len() >= 2 {
                        // Remove quotes from PID field
                        let pid_str = fields[1].trim_matches('"');
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            info!(
                                "Killing external {} process with PID {}",
                                executable_name, pid
                            );

                            // Kill the process using taskkill
                            let kill_result = Command::new("taskkill")
                                .arg("/F")
                                .arg("/PID")
                                .arg(pid.to_string())
                                .status();

                            match kill_result {
                                Ok(status) if status.success() => {
                                    info!(
                                        "Successfully killed {} process (PID: {})",
                                        executable_name, pid
                                    );
                                }
                                Ok(_) => {
                                    warn!("Failed to kill {} process (PID: {})", executable_name, pid);
                                }
                                Err(e) => {
                                    warn!(
                                        "Error killing {} process (PID: {}): {}",
                                        executable_name, pid, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                info!("No external {} processes found", executable_name);
            }
            Err(e) => {
                warn!("Failed to search for {} processes: {}", executable_name, e);
            }
        }

        // Wait a moment for processes to die
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }

    #[cfg(unix)]
    async fn kill_external_drivers_unix(&self, executable_name: &str) -> Result<()> {
        // Use pgrep to find processes by name
        let output = Command::new("pgrep")
            .arg("-f")
            .arg(executable_name)
            .output();

        match output {
            Ok(output) if output.status.success() => {
                let pids = String::from_utf8_lossy(&output.stdout);
                for pid_str in pids.lines() {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        info!(
                            "Killing external {} process with PID {}",
                            executable_name, pid
                        );

                        // Kill the process
                        let kill_result =
                            Command::new("kill").arg("-9").arg(pid.to_string()).status();

                        match kill_result {
                            Ok(status) if status.success() => {
                                info!(
                                    "Successfully killed {} process (PID: {})",
                                    executable_name, pid
                                );
                            }
                            Ok(_) => {
                                warn!("Failed to kill {} process (PID: {})", executable_name, pid);
                            }
                            Err(e) => {
                                warn!(
                                    "Error killing {} process (PID: {}): {}",
                                    executable_name, pid, e
                                );
                            }
                        }
                    }
                }
            }
            Ok(_) => {
                info!("No external {} processes found", executable_name);
            }
            Err(e) => {
                warn!("Failed to search for {} processes: {}", executable_name, e);
            }
        }

        // Wait a moment for processes to die
        sleep(Duration::from_millis(500)).await;
        Ok(())
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for DriverManager {
    fn drop(&mut self) {
        // For cleanup in Drop, we need to kill processes synchronously
        let mut processes = self.running_processes.lock().unwrap();

        for managed_process in processes.iter_mut() {
            info!(
                "Cleanup: Stopping {} driver (PID: {})",
                managed_process.driver_type.browser_name(),
                managed_process.pid
            );

            // Use start_kill for immediate termination in Drop
            if let Err(e) = managed_process.process.start_kill() {
                warn!(
                    "Failed to kill {} process during cleanup: {}",
                    managed_process.driver_type.browser_name(),
                    e
                );
            }
        }

        processes.clear();
    }
}

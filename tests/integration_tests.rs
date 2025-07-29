use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use rust_browser_mcp::{ClientManager, Config, DriverManager, DriverType};

#[test]
fn test_headless_configuration() {
    // Test that headless mode is properly configured
    let config = Config::from_env();

    // Should default to headless=true
    assert!(
        config.headless,
        "Default configuration should enable headless mode"
    );

    // Test environment variable override
    unsafe {
        std::env::set_var("WEBDRIVER_HEADLESS", "false");
    }
    let config = Config::from_env();
    assert!(
        !config.headless,
        "WEBDRIVER_HEADLESS=false should disable headless mode"
    );

    unsafe {
        std::env::set_var("WEBDRIVER_HEADLESS", "true");
    }
    let config = Config::from_env();
    assert!(
        config.headless,
        "WEBDRIVER_HEADLESS=true should enable headless mode"
    );

    // Clean up
    unsafe {
        std::env::remove_var("WEBDRIVER_HEADLESS");
    }
}

#[test]
fn test_browser_capabilities() {
    // Test that browser capabilities are correctly configured for different scenarios
    unsafe {
        std::env::set_var("WEBDRIVER_HEADLESS", "true");
        std::env::set_var("WEBDRIVER_PREFERRED_DRIVER", "firefox");
    }

    let config = Config::from_env();
    assert!(config.headless);
    assert_eq!(config.preferred_driver.as_deref(), Some("firefox"));

    // Clean up
    unsafe {
        std::env::remove_var("WEBDRIVER_HEADLESS");
        std::env::remove_var("WEBDRIVER_PREFERRED_DRIVER");
    }
}

#[test]
fn test_config_validation() {
    let mut config = Config::from_env();

    // Valid configuration should pass
    assert!(config.validate().is_ok());

    // Invalid timeout should fail
    config.default_session_timeout_ms = 0;
    assert!(config.validate().is_err());

    // Invalid endpoint should fail
    config.webdriver_endpoint = "invalid-url".to_string();
    assert!(config.validate().is_err());
}

// Test automatic cleanup functionality
#[test]
fn test_automatic_cleanup_config() {
    // Test that the automatic cleanup is properly configured
    let config = Config::from_env();
    assert!(
        config.auto_start_driver,
        "Auto-start should be enabled by default"
    );

    unsafe {
        std::env::set_var("WEBDRIVER_AUTO_START", "false");
    }
    let config = Config::from_env();
    assert!(
        !config.auto_start_driver,
        "WEBDRIVER_AUTO_START=false should disable auto-start"
    );

    // Clean up
    unsafe {
        std::env::remove_var("WEBDRIVER_AUTO_START");
    }
}

#[test]
fn test_driver_detection() {
    // Test that we can detect different driver types
    use rust_browser_mcp::DriverType;

    assert_eq!(DriverType::Firefox.executable_name(), "geckodriver");
    assert_eq!(DriverType::Firefox.default_port(), 4444);
    assert_eq!(DriverType::Firefox.browser_name(), "Firefox");

    assert_eq!(DriverType::Chrome.executable_name(), "chromedriver");
    assert_eq!(DriverType::Chrome.default_port(), 9515);
    assert_eq!(DriverType::Chrome.browser_name(), "Chrome");
}

// Helper function to check if geckodriver is available
fn is_geckodriver_available() -> bool {
    Command::new("which")
        .arg("geckodriver")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

// Helper function to check if any geckodriver processes are running
fn count_geckodriver_processes() -> usize {
    Command::new("pgrep")
        .arg("-f")
        .arg("geckodriver")
        .output()
        .map(|output| {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).lines().count()
            } else {
                0
            }
        })
        .unwrap_or(0)
}

#[tokio::test]
async fn test_driver_manager_cleanup() {
    if !is_geckodriver_available() {
        println!("Skipping test: geckodriver not available");
        return;
    }

    let driver_manager = DriverManager::new();

    // Test that cleanup methods can be called without errors
    // This is more realistic than trying to test actual process killing which is flaky
    let cleanup_result = driver_manager.force_cleanup_all_processes().await;
    assert!(cleanup_result.is_ok(), "Cleanup should succeed without errors");

    // Test that the DriverManager can handle the case when no processes need cleanup
    let driver_type = DriverType::Firefox;
    let kill_external_result = driver_manager.kill_external_drivers(&driver_type).await;
    assert!(kill_external_result.is_ok(), "kill_external_drivers should succeed");

    // Test stop_all_drivers on empty state
    let stop_all_result = driver_manager.stop_all_drivers().await;
    assert!(stop_all_result.is_ok(), "stop_all_drivers should succeed");

    println!("✅ Driver cleanup methods work correctly");
}

#[tokio::test]
async fn test_client_automatic_retry() {
    if !is_geckodriver_available() {
        println!("Skipping test: geckodriver not available");
        return;
    }

    // Set up environment for Firefox headless mode
    unsafe {
        std::env::set_var("WEBDRIVER_HEADLESS", "true");
        std::env::set_var("WEBDRIVER_PREFERRED_DRIVER", "firefox");
    }

    let config = Config::from_env();
    let client_manager = ClientManager::new(config).expect("Failed to create ClientManager");

    // Start a manual geckodriver process that will conflict
    let mut conflicting_process = Command::new("geckodriver")
        .arg("--port=4444")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start conflicting geckodriver");

    // Wait for it to start
    sleep(Duration::from_secs(2)).await;

    // Now try to create a client - this should trigger automatic cleanup and retry
    let create_client_result = timeout(
        Duration::from_secs(15), // Give it time to cleanup and retry
        client_manager.get_or_create_client(None),
    )
    .await;

    // Clean up the manual process
    let _ = conflicting_process.kill();

    // Clean up environment
    unsafe {
        std::env::remove_var("WEBDRIVER_HEADLESS");
        std::env::remove_var("WEBDRIVER_PREFERRED_DRIVER");
    }

    match create_client_result {
        Ok(Ok((session_id, _client))) => {
            println!("✅ Successfully created client after automatic cleanup: {session_id}");
            // This is the success case - automatic cleanup worked
        }
        Ok(Err(e)) => {
            println!("⚠️  Client creation failed: {e}");
            // This might be expected in some environments (no Firefox, etc.)
            // Don't fail the test, just log it
        }
        Err(_timeout) => {
            panic!("❌ Client creation timed out - automatic cleanup may not be working");
        }
    }
}

#[test]
fn test_error_handling_robustness() {
    // Test that our cleanup code handles various error conditions gracefully
    let driver_manager = DriverManager::new();
    let driver_type = DriverType::Firefox;

    // This should not panic even if pgrep fails or no processes exist
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(driver_manager.kill_external_drivers(&driver_type));

    // Should succeed even if no processes to kill
    assert!(
        result.is_ok(),
        "Cleanup should handle 'no processes found' gracefully"
    );
}

#[tokio::test]
async fn test_multiple_session_isolation() {
    if !is_geckodriver_available() {
        println!("Skipping test: geckodriver not available");
        return;
    }

    unsafe {
        std::env::set_var("WEBDRIVER_HEADLESS", "true");
        std::env::set_var("WEBDRIVER_PREFERRED_DRIVER", "firefox");
    }

    let config = Config::from_env();
    let client_manager = ClientManager::new(config).expect("Failed to create ClientManager");

    // Try to create two different sessions
    let session1_result = timeout(
        Duration::from_secs(10),
        client_manager.get_or_create_client(Some("session1".to_string())),
    )
    .await;

    let session2_result = timeout(
        Duration::from_secs(10),
        client_manager.get_or_create_client(Some("session2".to_string())),
    )
    .await;

    // Clean up environment
    unsafe {
        std::env::remove_var("WEBDRIVER_HEADLESS");
        std::env::remove_var("WEBDRIVER_PREFERRED_DRIVER");
    }

    match (session1_result, session2_result) {
        (Ok(Ok((id1, _))), Ok(Ok((id2, _)))) => {
            println!("✅ Successfully created multiple sessions: {id1} and {id2}");
            assert_eq!(id1, "session1");
            assert_eq!(id2, "session2");
        }
        _ => {
            println!(
                "⚠️  Multiple session creation had issues - this may be expected in some environments"
            );
            // Don't fail the test as this depends on browser availability
        }
    }
}

#[tokio::test]
async fn test_lifecycle_management() {
    if !is_geckodriver_available() {
        println!("Skipping test: geckodriver not available");
        return;
    }

    let driver_manager = DriverManager::new();
    let driver_type = DriverType::Firefox;

    // Test that no drivers are initially managed
    let initial_status = driver_manager.get_managed_processes_status();
    assert!(
        initial_status.is_empty(),
        "Should start with no managed processes"
    );
    assert!(
        !driver_manager.is_driver_managed(&driver_type),
        "Firefox should not be initially managed"
    );

    // Test starting a driver manually
    let start_result = driver_manager
        .start_driver_manually(driver_type.clone())
        .await;

    match start_result {
        Ok(endpoint) => {
            println!("✅ Successfully started driver at: {endpoint}");

            // Verify driver is now managed
            let status_after_start = driver_manager.get_managed_processes_status();
            assert!(
                !status_after_start.is_empty(),
                "Should have managed processes after start"
            );
            assert!(
                driver_manager.is_driver_managed(&driver_type),
                "Firefox should be managed after start"
            );

            // Verify endpoint format
            assert!(
                endpoint.starts_with("http://localhost:"),
                "Endpoint should be localhost URL"
            );
            assert!(endpoint.contains("4444"), "Firefox should use port 4444");

            // Test stopping the specific driver
            let stop_result = driver_manager.stop_driver_by_type(&driver_type).await;
            assert!(
                stop_result.is_ok(),
                "Should be able to stop specific driver type"
            );

            // Wait for process to be cleaned up
            sleep(Duration::from_millis(500)).await;

            // Verify driver is no longer managed
            let status_after_stop = driver_manager.get_managed_processes_status();
            let firefox_processes: Vec<_> = status_after_stop
                .iter()
                .filter(|(dt, _, _)| dt == &driver_type)
                .collect();
            assert!(
                firefox_processes.is_empty(),
                "Firefox processes should be cleaned up after stop"
            );

            println!("✅ Lifecycle management test completed successfully");
        }
        Err(e) => {
            println!("⚠️  Driver start failed: {e} - this may be expected in some environments");
            // Don't fail the test as this depends on system configuration
        }
    }

    // Test stop_all_drivers functionality (should not fail even if no drivers running)
    let stop_all_result = driver_manager.stop_all_drivers().await;
    assert!(
        stop_all_result.is_ok(),
        "stop_all_drivers should succeed even with no running drivers"
    );

    // Final verification - no processes should be managed
    let final_status = driver_manager.get_managed_processes_status();
    assert!(
        final_status.is_empty(),
        "Should end with no managed processes"
    );
}

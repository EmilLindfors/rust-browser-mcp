use anyhow::Result;
use rust_browser_mcp::{Config, ClientManager, DriverManager, WebDriverServer};
use std::time::Duration;

fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("info")
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .try_init();
}

#[tokio::test]
async fn test_concurrent_driver_startup() -> Result<()> {
    init_test_tracing();

    let driver_manager = DriverManager::new();
    let drivers = vec!["firefox".to_string()];
    let timeout = Duration::from_secs(15);
    
    let start_time = std::time::Instant::now();
    let result = driver_manager.start_concurrent_drivers(&drivers, timeout).await;
    let duration = start_time.elapsed();
    
    assert!(result.is_ok(), "Driver startup should succeed");
    let started_drivers = result.unwrap();
    assert!(!started_drivers.is_empty(), "At least one driver should start");
    
    // Verify performance: startup should be under 5 seconds
    assert!(duration < Duration::from_secs(5), "Driver startup should be fast");
    
    // Cleanup
    driver_manager.stop_all_drivers().await?;
    Ok(())
}

#[tokio::test]  
async fn test_health_check_functionality() -> Result<()> {
    init_test_tracing();

    let driver_manager = DriverManager::new();
    let drivers = vec!["firefox".to_string()];
    let timeout = Duration::from_secs(15);
    
    // Start drivers
    driver_manager.start_concurrent_drivers(&drivers, timeout).await?;
    
    // Test health check
    driver_manager.refresh_driver_health().await?;
    let healthy_endpoints = driver_manager.get_healthy_endpoints();
    
    // Should have at least one healthy driver
    assert!(!healthy_endpoints.is_empty(), "Should have healthy drivers after startup");
    
    // Cleanup
    driver_manager.stop_all_drivers().await?;
    Ok(())
}

#[tokio::test]
async fn test_client_manager_session_creation() -> Result<()> {
    init_test_tracing();

    // Setup: Start drivers first
    let driver_manager = DriverManager::new();
    let drivers = vec!["firefox".to_string()];
    let timeout = Duration::from_secs(15);
    driver_manager.start_concurrent_drivers(&drivers, timeout).await?;

    // Create ClientManager with pre-started drivers
    let mut config = Config::from_env();
    config.concurrent_drivers = vec!["firefox".to_string()];
    let client_manager = ClientManager::new(config)?;

    // Test session creation performance
    let start_time = std::time::Instant::now();
    let result = client_manager.get_or_create_client(Some("test_session".to_string())).await;
    let duration = start_time.elapsed();

    assert!(result.is_ok(), "Session creation should succeed");
    let (session_id, _client) = result.unwrap();
    assert_eq!(session_id, "test_session");
    
    // Session creation should be fast with pre-started drivers
    println!("Session creation took: {:?}", duration);
    
    // Cleanup
    driver_manager.stop_all_drivers().await?;
    Ok(())
}

#[tokio::test]
async fn test_session_browser_preference() -> Result<()> {
    init_test_tracing();

    // Setup: Start drivers
    let driver_manager = DriverManager::new();
    let drivers = vec!["firefox".to_string()];
    let timeout = Duration::from_secs(15);
    driver_manager.start_concurrent_drivers(&drivers, timeout).await?;

    let mut config = Config::from_env();
    config.concurrent_drivers = vec!["firefox".to_string()];
    let client_manager = ClientManager::new(config)?;

    // Test different session ID patterns
    let test_cases = vec![
        ("firefox_session1", "Should prefer Firefox"),
        ("chrome_session1", "Should try Chrome but fallback"),
        ("default_session", "Should use any available"),
    ];

    for (session_id, _description) in test_cases {
        let result = client_manager.get_or_create_client(Some(session_id.to_string())).await;
        
        // All should succeed (even chrome_session1 should fallback to firefox)
        assert!(result.is_ok(), "Session creation should succeed for {}", session_id);
        
        if let Ok((actual_session, _client)) = result {
            assert_eq!(actual_session, session_id);
        }
    }

    // Cleanup
    driver_manager.stop_all_drivers().await?;
    Ok(())
}

#[tokio::test]
async fn test_driver_status_reporting() -> Result<()> {
    init_test_tracing();

    let driver_manager = DriverManager::new();
    
    // Initially no processes
    let initial_status = driver_manager.get_managed_processes_status();
    assert!(initial_status.is_empty(), "Should start with no managed processes");

    // Start drivers
    let drivers = vec!["firefox".to_string()];
    let timeout = Duration::from_secs(15);
    driver_manager.start_concurrent_drivers(&drivers, timeout).await?;

    // Should have managed processes now
    let status_after_start = driver_manager.get_managed_processes_status();
    assert!(!status_after_start.is_empty(), "Should have managed processes after startup");

    // Cleanup
    driver_manager.stop_all_drivers().await?;

    // Should be empty again after cleanup
    let final_status = driver_manager.get_managed_processes_status();
    assert!(final_status.is_empty(), "Should have no managed processes after cleanup");
    
    Ok(())
}

#[tokio::test]
async fn test_new_mcp_tools_exist() -> Result<()> {
    init_test_tracing();

    // Create server to test tool definitions
    let mut config = Config::from_env();
    config.auto_start_driver = false; // Disable auto-start for this test
    let _server = WebDriverServer::with_config(config)?;

    // Test that ToolDefinitions includes our new tools in stdio mode
    let stdio_tools = rust_browser_mcp::tools::ToolDefinitions::list_for_mode(rust_browser_mcp::tools::ServerMode::Stdio);
    let stdio_tool_names: Vec<&str> = stdio_tools.iter().map(|t| t.name.as_ref()).collect();
    
    assert!(stdio_tool_names.contains(&"get_healthy_endpoints"), "get_healthy_endpoints tool should be exposed in stdio mode");
    assert!(stdio_tool_names.contains(&"refresh_driver_health"), "refresh_driver_health tool should be exposed in stdio mode");
    assert!(stdio_tool_names.contains(&"list_managed_drivers"), "list_managed_drivers tool should be exposed in stdio mode");
    assert!(stdio_tool_names.contains(&"start_driver"), "start_driver tool should be exposed in stdio mode");
    assert!(stdio_tool_names.contains(&"stop_driver"), "stop_driver tool should be exposed in stdio mode");
    assert!(stdio_tool_names.contains(&"stop_all_drivers"), "stop_all_drivers tool should be exposed in stdio mode");

    // Test that HTTP mode does NOT include driver lifecycle tools
    let http_tools = rust_browser_mcp::tools::ToolDefinitions::list_for_mode(rust_browser_mcp::tools::ServerMode::Http);
    let http_tool_names: Vec<&str> = http_tools.iter().map(|t| t.name.as_ref()).collect();
    
    assert!(!http_tool_names.contains(&"get_healthy_endpoints"), "get_healthy_endpoints tool should NOT be exposed in http mode");
    assert!(!http_tool_names.contains(&"refresh_driver_health"), "refresh_driver_health tool should NOT be exposed in http mode");
    assert!(!http_tool_names.contains(&"list_managed_drivers"), "list_managed_drivers tool should NOT be exposed in http mode");
    assert!(!http_tool_names.contains(&"start_driver"), "start_driver tool should NOT be exposed in http mode");
    assert!(!http_tool_names.contains(&"stop_driver"), "stop_driver tool should NOT be exposed in http mode");
    assert!(!http_tool_names.contains(&"stop_all_drivers"), "stop_all_drivers tool should NOT be exposed in http mode");

    // Verify we have the expected number of tools
    assert!(stdio_tools.len() >= 25, "Should have at least 25 tools with new additions in stdio mode");
    assert_eq!(http_tools.len(), stdio_tools.len() - 6, "HTTP mode should have 6 fewer tools than stdio mode");

    println!("✅ Found {} MCP tools including new health monitoring tools", stdio_tools.len());
    println!("📋 STDIO mode tools: {:?}", stdio_tool_names);
    println!("📋 HTTP mode tools: {:?}", http_tool_names);

    Ok(())
}
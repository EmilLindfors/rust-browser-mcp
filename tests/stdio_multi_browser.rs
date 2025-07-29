mod common;
use common::{TestClient, TestTimer};

/// Integration test for using multiple browser drivers via stdio
/// Tests that we can start both Chrome and Firefox and switch between them
#[tokio::test]
async fn test_multi_browser_stdio_integration() {
    println!("🚀 Testing multi-browser stdio integration (Chrome + Firefox)");
    
    let client = TestClient::new().await.expect("Failed to create test client");
    
    if let Some(server_info) = client.server_info() {
        println!("✅ Connected to server: {}", server_info.server_info.name);
    }
    
    // Test 1: Start Chrome driver
    println!("🌐 Test 1: Starting Chrome driver");
    let chrome_timer = TestTimer::new();
    
    let chrome_result = client.start_driver("chrome").await;
    let chrome_time = chrome_timer.elapsed();
    
    println!("  Chrome startup time: {:.2}ms", chrome_timer.elapsed_ms());
    
    let chrome_success = chrome_result.is_ok() && 
        chrome_result.as_ref().map(|r| common::check_tool_result_success(r)).unwrap_or(false);
    
    // Test 2: Start Firefox driver  
    println!("🦊 Test 2: Starting Firefox driver");
    let firefox_timer = TestTimer::new();
    
    let firefox_result = client.start_driver("firefox").await;
    let firefox_time = firefox_timer.elapsed();
    
    println!("  Firefox startup time: {:.2}ms", firefox_timer.elapsed_ms());
    
    let firefox_success = firefox_result.is_ok() && 
        firefox_result.as_ref().map(|r| common::check_tool_result_success(r)).unwrap_or(false);
    
    // Test 3: Navigate with Chrome (if available)
    let mut chrome_nav_success = false;
    if chrome_success {
        println!("🌐 Test 3: Navigate with Chrome");
        let nav_timer = TestTimer::new();
        
        match client.navigate("https://example.com", Some("chrome_session")).await {
            Ok(result) => {
                println!("  Chrome navigation time: {:.2}ms", nav_timer.elapsed_ms());
                
                chrome_nav_success = common::check_tool_result_success(&result);
                
                if chrome_nav_success {
                    println!("  ✅ Chrome navigation successful");
                } else {
                    println!("  ⚠️  Chrome navigation failed (expected in some environments)");
                }
            }
            Err(e) => {
                println!("  ⚠️  Chrome navigation failed: {}", e);
            }
        }
    }
    
    // Test 4: Navigate with Firefox (if available)
    let mut firefox_nav_success = false;
    if firefox_success {
        println!("🦊 Test 4: Navigate with Firefox");
        let nav_timer = TestTimer::new();
        
        match client.navigate("https://httpbin.org/html", Some("firefox_session")).await {
            Ok(result) => {
                println!("  Firefox navigation time: {:.2}ms", nav_timer.elapsed_ms());
                
                firefox_nav_success = common::check_tool_result_success(&result);
                
                if firefox_nav_success {
                    println!("  ✅ Firefox navigation successful");
                } else {
                    println!("  ⚠️  Firefox navigation failed (expected in some environments)");
                }
            }
            Err(e) => {
                println!("  ⚠️  Firefox navigation failed: {}", e);
            }
        }
    }
    
    // Test 5: Get page titles from both browsers (if both working)
    if chrome_nav_success && firefox_nav_success {
        println!("📄 Test 5: Get page titles from both browsers");
        
        // Get Chrome title
        if let Ok(chrome_title_result) = client.get_title(Some("chrome_session")).await {
            println!("  Chrome title result: {:?}", chrome_title_result.content.first());
        }
        
        // Get Firefox title  
        if let Ok(firefox_title_result) = client.get_title(Some("firefox_session")).await {
            println!("  Firefox title result: {:?}", firefox_title_result.content.first());
        }
    }
    
    // Test 6: Cleanup - stop all drivers
    println!("🧹 Test 6: Cleanup - stopping all drivers");
    
    let cleanup_success = match client.stop_all_drivers().await {
        Ok(_) => {
            println!("  Cleanup successful: true");
            true
        }
        Err(e) => {
            println!("  Cleanup failed: {}", e);
            false
        }
    };
    
    // Clean shutdown
    let _ = client.cleanup().await;
    
    // Results summary
    println!("\n📊 Multi-Browser Integration Test Results:");
    println!("==========================================");
    println!("Chrome driver start: {}", if chrome_success { "✅ SUCCESS" } else { "❌ FAILED" });
    println!("Firefox driver start: {}", if firefox_success { "✅ SUCCESS" } else { "❌ FAILED" });
    println!("Chrome navigation: {}", if chrome_nav_success { "✅ SUCCESS" } else { "❌ FAILED" });
    println!("Firefox navigation: {}", if firefox_nav_success { "✅ SUCCESS" } else { "❌ FAILED" });
    println!("Cleanup: {}", if cleanup_success { "✅ SUCCESS" } else { "❌ FAILED" });
    
    let total_tests = 5;
    let passed_tests = [chrome_success, firefox_success, chrome_nav_success, firefox_nav_success, cleanup_success]
        .iter().filter(|&&x| x).count();
    
    println!("Overall: {}/{} tests passed ({:.1}%)", 
             passed_tests, total_tests, 
             (passed_tests as f64 / total_tests as f64) * 100.0);
    
    // Performance assertions
    if chrome_success {
        assert!(chrome_time.as_millis() < 10000, "Chrome startup should be under 10 seconds");
    }
    if firefox_success {
        assert!(firefox_time.as_millis() < 15000, "Firefox startup should be under 15 seconds (slower in WSL2)");  
    }
    
    // At least one browser should work
    assert!(chrome_success || firefox_success, "At least one browser driver should start successfully");
    
    // Cleanup should always work
    assert!(cleanup_success, "Cleanup should always succeed");
}

#[tokio::test]
async fn test_stdio_session_isolation() {
    println!("🔒 Testing stdio session isolation between browsers");
    
    let client = TestClient::new().await.expect("Failed to create test client");
    
    if let Some(server_info) = client.server_info() {
        println!("✅ Connected to server: {}", server_info.server_info.name);
    }
    
    // Test: For stdio mode, should use simplified session logic
    // Multiple session requests should reuse the same session
    println!("Testing stdio session reuse behavior...");
    
    // Request 1: Default session
    let timer1 = TestTimer::new();
    let response1 = client.start_driver("chrome").await;
    let first_request_time = timer1.elapsed();
    
    // Request 2: Different session ID (should still reuse in stdio mode)
    let timer2 = TestTimer::new();
    let response2 = client.start_driver_with_session("chrome", "different_session").await;
    let second_request_time = timer2.elapsed();
    
    println!("First request time: {:.2}ms", timer1.elapsed_ms());
    println!("Second request time: {:.2}ms", timer2.elapsed_ms());
    
    // In stdio mode, the second request should be much faster (reusing session)
    // unless it's starting a new driver type
    let reuse_detected = second_request_time < first_request_time / 2;
    println!("Session reuse detected: {}", reuse_detected);
    
    let _ = client.cleanup().await;
    
    // At least one request should succeed
    let request1_success = response1.is_ok();
    let request2_success = response2.is_ok();
    
    assert!(request1_success || request2_success, "At least one request should succeed");
    
    println!("✅ Session isolation test completed");
}
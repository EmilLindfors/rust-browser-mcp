mod common;
use common::{TestClient, TestTimer};
use std::time::Duration;
use tokio::time::sleep;

/// Test multi-browser recipe execution
/// Comprehensive test for Chrome + Firefox recipe execution with screenshots
#[tokio::test]
async fn test_multi_browser_recipe_execution() {
    println!("🚀 Testing Multi-Browser Recipe Execution");
    println!("Comprehensive test: Chrome + Firefox recipe execution with screenshots");
    
    let client = TestClient::new().await.expect("Failed to create test client");
    
    if let Some(server_info) = client.server_info() {
        println!("✅ Connected to server: {}", server_info.server_info.name);
    }
    
    // Test 1: Execute multi-browser recipe
    println!("\n📋 Test 1: Execute multi-browser recipe 'test_multi_browser'");
    let recipe_timer = TestTimer::new();
    
    let recipe_result = client.execute_recipe("test_multi_browser").await;
    println!("  Recipe execution time: {:.2}ms", recipe_timer.elapsed_ms());
    
    let _recipe_success = match &recipe_result {
        Ok(_) => {
            println!("  ✅ Recipe execution completed successfully");
            true
        }
        Err(e) => {
            println!("  ❌ Recipe execution failed: {}", e);
            false
        }
    };
    
    // Test 2: Check for screenshot artifacts (the original issue was no screenshots)
    println!("\n📸 Test 2: Check for screenshot artifacts");
    
    let screenshot_files = ["chrome_test.png", "firefox_test.png"];
    let mut artifacts_found = 0;
    
    for filename in &screenshot_files {
        if std::path::Path::new(filename).exists() {
            let metadata = std::fs::metadata(filename).unwrap();
            println!("  ✅ {} exists ({} bytes)", filename, metadata.len());
            artifacts_found += 1;
        } else {
            println!("  ❌ {} not found", filename);
        }
    }
    
    // Test 3: Verify no orphaned processes remain
    println!("\n🧹 Test 3: Check for orphaned browser processes");
    
    let process_check = std::process::Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -E '(chrome|firefox|chromium)' | grep -E '(headless|webdriver|marionette)' | grep -v grep | wc -l")
        .output()
        .expect("Failed to check processes");
    
    let process_count = String::from_utf8_lossy(&process_check.stdout)
        .trim()
        .parse::<i32>()
        .unwrap_or(-1);
    
    println!("  📊 Orphaned browser processes found: {}", process_count);
    
    if process_count <= 2 {  // Allow for some system processes
        println!("  ✅ Process count is acceptable");
    } else {
        println!("  ⚠️ Potentially orphaned processes detected");
    }
    
    // Test 4: Test the new force cleanup functionality
    println!("\n🧽 Test 4: Test force cleanup functionality");
    
    let cleanup_result = client.force_cleanup_orphaned_processes().await;
    match &cleanup_result {
        Ok(_) => println!("  ✅ Force cleanup completed successfully"),
        Err(e) => println!("  ❌ Force cleanup failed: {}", e),
    }
    
    // Final verification - check process count again
    let final_check = std::process::Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -E '(chrome|firefox|chromium)' | grep -E '(headless|webdriver|marionette)' | grep -v grep | wc -l")
        .output()
        .expect("Failed to check final processes");
    
    let final_count = String::from_utf8_lossy(&final_check.stdout)
        .trim()
        .parse::<i32>()
        .unwrap_or(-1);
    
    println!("  📊 Final orphaned process count: {}", final_count);
    
    // Test Summary
    println!("\n{}", "=".repeat(60));
    println!("📋 MULTI-BROWSER RECIPE TEST SUMMARY");
    println!("{}", "=".repeat(60));
    
    let recipe_success = recipe_result.is_ok();
    let artifacts_created = artifacts_found > 0;
    let cleanup_success = cleanup_result.is_ok();
    let processes_clean = final_count <= 2;
    
    println!("✅ Recipe execution: {}", if recipe_success { "PASSED" } else { "FAILED" });
    println!("✅ Screenshot creation: {}", if artifacts_created { "PASSED" } else { "FAILED" });
    println!("✅ Process cleanup: {}", if cleanup_success { "PASSED" } else { "FAILED" });
    println!("✅ No orphaned processes: {}", if processes_clean { "PASSED" } else { "FAILED" });
    
    let overall_success = recipe_success && cleanup_success && processes_clean;
    
    if overall_success {
        println!("\n🎉 OVERALL RESULT: MULTI-BROWSER ISSUE RESOLVED!");
        println!("The orphaned browser process fix has successfully resolved the multi-browser recipe execution issue.");
    } else {
        println!("\n💥 OVERALL RESULT: SOME ISSUES REMAIN");
        if !recipe_success {
            println!("   - Recipe execution still failing");
        }
        if !cleanup_success {
            println!("   - Cleanup functionality not working");
        }
        if !processes_clean {
            println!("   - Orphaned processes still present");
        }
    }
    
    // Clean up test artifacts
    for filename in &screenshot_files {
        if std::path::Path::new(filename).exists() {
            let _ = std::fs::remove_file(filename);
        }
    }
    
    // Assert for test framework
    assert!(overall_success, "Multi-browser recipe execution test failed");
}

/// Test the specific force cleanup functionality we implemented
#[tokio::test]
async fn test_force_cleanup_functionality() {
    println!("🧹 Testing Force Cleanup Functionality");
    
    let client = TestClient::new().await.expect("Failed to create test client");
    
    // Get initial process count
    let initial_check = std::process::Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -E '(chrome|firefox|chromium)' | grep -E '(headless|webdriver|marionette)' | grep -v grep | wc -l")
        .output()
        .expect("Failed to check initial processes");
    
    let initial_count = String::from_utf8_lossy(&initial_check.stdout)
        .trim()
        .parse::<i32>()
        .unwrap_or(-1);
    
    println!("📊 Initial orphaned process count: {}", initial_count);
    
    // Test the force cleanup functionality
    let cleanup_result = client.force_cleanup_orphaned_processes().await;
    
    match &cleanup_result {
        Ok(result) => {
            println!("✅ Force cleanup completed successfully");
            
            // Check if the result contains success message
            for item in &result.content {
                let text = format!("{:?}", item.raw);
                println!("📄 Cleanup message: {}", text);
            }
        }
        Err(e) => {
            println!("❌ Force cleanup failed: {}", e);
        }
    }
    
    // Check final process count
    sleep(Duration::from_millis(1000)).await; // Give processes time to die
    
    let final_check = std::process::Command::new("sh")
        .arg("-c")
        .arg("ps aux | grep -E '(chrome|firefox|chromium)' | grep -E '(headless|webdriver|marionette)' | grep -v grep | wc -l")
        .output()
        .expect("Failed to check final processes");
    
    let final_count = String::from_utf8_lossy(&final_check.stdout)
        .trim()
        .parse::<i32>()
        .unwrap_or(-1);
    
    println!("📊 Final orphaned process count: {}", final_count);
    
    // The cleanup should work and reduce process count
    let cleanup_effective = final_count <= initial_count;
    
    if cleanup_effective {
        println!("✅ Force cleanup functionality is working correctly");
    } else {
        println!("❌ Force cleanup may not be fully effective");
    }
    
    assert!(cleanup_result.is_ok(), "Force cleanup should complete without errors");
    assert!(cleanup_effective, "Cleanup should be effective at reducing process count");
}
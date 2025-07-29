use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tokio;
use rust_browser_mcp::{WebDriverServer, Recipe, RecipeStep, RecipeExecutor, ExecutionContext};

#[tokio::test]
async fn test_recipe_navigation_and_screenshot() {
    // Create WebDriver server
    let server = match WebDriverServer::new() {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to create WebDriver server: {}", e);
            panic!("Cannot run test without WebDriver server");
        }
    };

    // Create a simple test recipe
    let recipe = Recipe {
        name: "integration_test_recipe".to_string(),
        description: Some("Integration test recipe for navigation and screenshot".to_string()),
        version: "1.0.0".to_string(),
        author: None,
        created_at: None,
        parameters: None,
        browsers: vec!["chrome".to_string()],
        steps: vec![
            RecipeStep {
                name: Some("Navigate to example.com".to_string()),
                description: None,
                action: "navigate".to_string(),
                arguments: {
                    let mut args = serde_json::Map::new();
                    args.insert("url".to_string(), serde_json::Value::String("https://example.com".to_string()));
                    args
                },
                continue_on_error: None,
                retry_count: None,
                retry_delay_ms: None,
                condition: None,
                session_id: Some("test_session".to_string()),
                browser: Some("chrome".to_string()),
            },
            RecipeStep {
                name: Some("Take screenshot".to_string()),
                description: None,
                action: "screenshot".to_string(),
                arguments: {
                    let mut args = serde_json::Map::new();
                    args.insert("save_path".to_string(), serde_json::Value::String("integration_test_screenshot.png".to_string()));
                    args
                },
                continue_on_error: None,
                retry_count: None,
                retry_delay_ms: None,
                condition: None,
                session_id: Some("test_session".to_string()),
                browser: Some("chrome".to_string()),
            },
        ],
    };

    // Create execution context
    let context = ExecutionContext {
        session_id: Some("test_session".to_string()),
        variables: HashMap::new(),
        continue_on_error: false,
    };

    // Execute the recipe
    let executor = RecipeExecutor::new(&server);
    let start_time = Instant::now();
    
    println!("🚀 Starting recipe execution...");
    let result = executor.execute_recipe(&recipe, None, context).await;
    let execution_time = start_time.elapsed();
    
    println!("⏱️  Recipe execution took: {}ms", execution_time.as_millis());

    // Analyze the results
    match result {
        Ok(execution_result) => {
            println!("✅ Recipe execution result: {}", execution_result.to_detailed_string());
            
            // Critical checks
            println!("\n🔍 CRITICAL ANALYSIS:");
            println!("- Reported execution time: {}ms", execution_result.execution_time_ms);
            println!("- Actual measured time: {}ms", execution_time.as_millis());
            println!("- Success: {}", execution_result.success);
            println!("- Total steps: {}", execution_result.total_steps);
            println!("- Executed steps: {}", execution_result.executed_steps);
            println!("- Failed steps: {}", execution_result.failed_steps);

            // Check if screenshot file was created
            let screenshot_path = PathBuf::from("integration_test_screenshot.png");
            let file_exists = screenshot_path.exists();
            println!("- Screenshot file created: {}", file_exists);
            
            if file_exists {
                if let Ok(metadata) = std::fs::metadata(&screenshot_path) {
                    println!("- Screenshot file size: {} bytes", metadata.len());
                }
            }

            // Step-by-step analysis
            println!("\n📋 STEP-BY-STEP ANALYSIS:");
            for (i, step_result) in execution_result.step_results.iter().enumerate() {
                println!("Step {}: {} - {} in {}ms", 
                    i + 1, 
                    step_result.step_name.as_deref().unwrap_or("unnamed"),
                    if step_result.success { "SUCCESS" } else { "FAILED" },
                    step_result.execution_time_ms
                );
                if let Some(error) = &step_result.error_message {
                    println!("  Error: {}", error);
                }
                if let Some(result_msg) = &step_result.result {
                    println!("  Result: {}", result_msg);
                }
            }

            // The smoking gun test
            if execution_result.execution_time_ms < 100 {
                println!("\n🚨 PROBLEM DETECTED: Recipe completed in {}ms - too fast for real browser operations!", execution_result.execution_time_ms);
            }
            
            if !file_exists && execution_result.success {
                println!("🚨 PROBLEM DETECTED: Recipe claims success but screenshot file was not created!");
            }

            // Assert conditions that would indicate proper execution
            if execution_result.success {
                assert!(execution_result.execution_time_ms > 100, 
                    "Recipe execution time too fast: {}ms (should be >100ms for real browser operations)", 
                    execution_result.execution_time_ms);
                
                assert!(file_exists, 
                    "Screenshot file should exist if recipe succeeded");
            }
        }
        Err(e) => {
            println!("❌ Recipe execution failed with error: {}", e);
            panic!("Recipe execution should not fail in this test");
        }
    }
}

#[tokio::test] 
async fn test_direct_vs_recipe_comparison() {
    println!("\n🔬 DIRECT VS RECIPE COMPARISON TEST");
    
    let server = match WebDriverServer::new() {
        Ok(server) => server,
        Err(e) => {
            eprintln!("Failed to create WebDriver server: {}", e);
            return; // Skip test if server can't be created
        }
    };

    // Test 1: Direct WebDriver calls
    println!("\n1️⃣  Testing DIRECT WebDriver calls...");
    let start_time = Instant::now();
    
    let client_manager = server.get_client_manager();
    let (_session, client) = client_manager.get_or_create_client(Some("direct_test".to_string())).await
        .expect("Should be able to create client");
    
    println!("🌐 Navigating directly to example.com...");
    client.goto("https://example.com").await
        .expect("Direct navigation should work");
    
    println!("📸 Taking direct screenshot...");
    let screenshot_data = client.screenshot().await
        .expect("Direct screenshot should work");
    
    // Save direct screenshot (screenshot_data is already PNG binary data)
    std::fs::write("direct_comparison_screenshot.png", &screenshot_data)
        .expect("Should save direct screenshot");
    
    let direct_time = start_time.elapsed();
    println!("✅ Direct operations completed in {}ms", direct_time.as_millis());

    // Test 2: Recipe execution 
    println!("\n2️⃣  Testing RECIPE execution...");
    let recipe_start = Instant::now();
    
    let recipe = Recipe {
        name: "comparison_recipe".to_string(),
        description: Some("Recipe for comparison test".to_string()),
        version: "1.0.0".to_string(),
        author: None,
        created_at: None,
        parameters: None,
        browsers: vec!["chrome".to_string()],
        steps: vec![
            RecipeStep {
                name: Some("Navigate via recipe".to_string()),
                description: None,
                action: "navigate".to_string(),
                arguments: {
                    let mut args = serde_json::Map::new();
                    args.insert("url".to_string(), serde_json::Value::String("https://example.com".to_string()));
                    args
                },
                continue_on_error: None,
                retry_count: None,
                retry_delay_ms: None,
                condition: None,
                session_id: Some("recipe_test".to_string()),
                browser: Some("chrome".to_string()),
            },
            RecipeStep {
                name: Some("Screenshot via recipe".to_string()),
                description: None,
                action: "screenshot".to_string(),
                arguments: {
                    let mut args = serde_json::Map::new();
                    args.insert("save_path".to_string(), serde_json::Value::String("recipe_comparison_screenshot.png".to_string()));
                    args
                },
                continue_on_error: None,
                retry_count: None,
                retry_delay_ms: None,
                condition: None,
                session_id: Some("recipe_test".to_string()),
                browser: Some("chrome".to_string()),
            },
        ],
    };

    let context = ExecutionContext {
        session_id: Some("recipe_test".to_string()),
        variables: HashMap::new(),
        continue_on_error: false,
    };

    let executor = RecipeExecutor::new(&server);
    let result = executor.execute_recipe(&recipe, None, context).await
        .expect("Recipe execution should not error");
    
    let recipe_time = recipe_start.elapsed();

    // Compare results
    println!("\n🔍 COMPARISON RESULTS:");
    println!("Direct execution time: {}ms", direct_time.as_millis());
    println!("Recipe execution time (reported): {}ms", result.execution_time_ms);
    println!("Recipe execution time (measured): {}ms", recipe_time.as_millis());
    
    let direct_file_exists = std::path::Path::new("direct_comparison_screenshot.png").exists();
    let recipe_file_exists = std::path::Path::new("recipe_comparison_screenshot.png").exists();
    
    println!("Direct screenshot created: {}", direct_file_exists);
    println!("Recipe screenshot created: {}", recipe_file_exists);
    
    if direct_file_exists && !recipe_file_exists {
        println!("🚨 CONFIRMED: Direct works, Recipe doesn't create files!");
    }
    
    if direct_time.as_millis() > 100 && result.execution_time_ms < 10 {
        println!("🚨 CONFIRMED: Direct takes realistic time, Recipe completes instantly!");
    }
}
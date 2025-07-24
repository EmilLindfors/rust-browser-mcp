use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    object,
    transport::StreamableHttpClientTransport,
};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Advanced WebDriver MCP example showcasing performance monitoring and testing capabilities
/// This example demonstrates the unique performance and monitoring features of the crate

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    println!("🚀 WebDriver MCP Advanced Monitoring Example");
    println!("==============================================");
    println!("This example showcases advanced performance monitoring and testing features.");
    println!();

    // Start the HTTP server in the background
    let server_handle = tokio::spawn(async {
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(&[
            "run",
            "--features",
            "http-server",
            "--bin",
            "rust-browser-mcp",
            "--",
            "--transport",
            "http",
            "--bind",
            "127.0.0.1:8091",
        ]);
        cmd.kill_on_drop(true);
        let mut child = cmd.spawn().expect("Failed to start HTTP server");
        child.wait().await.expect("Server process failed")
    });

    // Wait for server to start
    println!("⏳ Starting HTTP server...");
    sleep(Duration::from_secs(3)).await;

    // Connect to the HTTP server
    let transport = StreamableHttpClientTransport::from_uri("http://127.0.0.1:8091/mcp");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "webdriver-mcp-advanced-monitor".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let client = client_info.serve(transport).await?;

    // List available tools
    let tools = client.list_all_tools().await?;
    println!("✅ Connected! Found {} tools including advanced monitoring tools\n", tools.len());

    // Demo 1: Performance Testing with multiple test cases
    println!("📊 Demo 1: Advanced Performance Testing");
    println!("========================================");
    
    // Use the built-in performance test tool
    let perf_result = client
        .call_tool(CallToolRequestParam {
            name: "run_performance_test".into(),
            arguments: Some(object!({
                "test_cases": [
                    {
                        "name": "Google Homepage Load",
                        "url": "https://www.google.com",
                        "actions": [
                            {"type": "wait", "duration": 1000},
                            {"type": "find", "selector": "input[name='q']"},
                            {"type": "type", "text": "WebDriver MCP"},
                            {"type": "click", "selector": "input[value='Google Search']"},
                        ]
                    },
                    {
                        "name": "Example.com Load",
                        "url": "https://example.com",
                        "actions": [
                            {"type": "wait", "duration": 2000},
                            {"type": "scroll", "selector": "body"}
                        ]
                    }
                ],
                "iterations": 3,
                "collect_metrics": true
            })),
        })
        .await?;

    if let Some(content) = perf_result.content.first() {
        println!("Performance Test Results:\n{}", 
                 serde_json::to_string_pretty(&content.raw)?);
    }
    println!();

    // Demo 2: Memory Usage Monitoring
    println!("🧠 Demo 2: Memory Usage Monitoring");
    println!("===================================");

    // Start memory monitoring
    let memory_result = client
        .call_tool(CallToolRequestParam {
            name: "monitor_memory_usage".into(),
            arguments: Some(object!({
                "duration_seconds": 10,
                "interval_ms": 1000,
                "include_gc_info": true
            })),
        })
        .await?;

    if let Some(content) = memory_result.content.first() {
        println!("Memory Monitoring Results:\n{}", 
                 serde_json::to_string_pretty(&content.raw)?);
    }
    println!();

    // Demo 3: Resource Usage Monitoring  
    println!("📈 Demo 3: Resource Usage Monitoring");
    println!("=====================================");

    let resource_result = client
        .call_tool(CallToolRequestParam {
            name: "monitor_resource_usage".into(),
            arguments: Some(object!({
                "duration_seconds": 8,
                "network_filter": ".*",
                "include_network": true,
                "include_fps": true,
                "include_cpu": true
            })),
        })
        .await?;

    if let Some(content) = resource_result.content.first() {
        println!("Resource Monitoring Results:\n{}", 
                 serde_json::to_string_pretty(&content.raw)?);
    }
    println!();

    // Demo 4: Real-world scenario - E-commerce performance audit
    println!("🛒 Demo 4: E-commerce Performance Audit");
    println!("========================================");
    
    println!("Simulating e-commerce site performance audit...");
    
    // Navigate to a demo e-commerce site
    client
        .call_tool(CallToolRequestParam {
            name: "navigate".into(),
            arguments: Some(object!({ "url": "https://demo.opencart.com/" })),
        })
        .await?;

    sleep(Duration::from_secs(2)).await;

    // Get performance metrics for the main page
    let main_page_perf = client
        .call_tool(CallToolRequestParam {
            name: "get_performance_metrics".into(),
            arguments: Some(object!({
                "include_navigation": true,
                "include_resources": true,
                "include_paint": true
            })),
        })
        .await?;

    println!("Main page performance:");
    if let Some(content) = main_page_perf.content.first() {
        let metrics_str = match &content.raw {
            rmcp::model::RawContent::Text(text) => &text.text,
            _ => "{}"
        };
        let metrics: Value = serde_json::from_str(metrics_str)?;
        if let Some(timing) = metrics.get("navigationTiming") {
            if let (Some(load_time), Some(dom_time)) = (
                timing.get("loadEventEnd").and_then(|v| v.as_f64()),
                timing.get("domContentLoadedEventEnd").and_then(|v| v.as_f64())
            ) {
                println!("  - Page Load Time: {:.2}ms", load_time);
                println!("  - DOM Ready Time: {:.2}ms", dom_time);
            }
        }
    }

    // Test search functionality performance
    println!("\nTesting search functionality...");
    let search_start = Instant::now();
    
    client
        .call_tool(CallToolRequestParam {
            name: "send_keys".into(),
            arguments: Some(object!({
                "selector": "input[name='search']",
                "text": "laptop"
            })),
        })
        .await?;

    client
        .call_tool(CallToolRequestParam {
            name: "click".into(),
            arguments: Some(object!({
                "selector": "button[type='submit']"
            })),
        })
        .await?;

    // Wait for search results and measure time
    sleep(Duration::from_secs(2)).await;
    let search_duration = search_start.elapsed();
    
    println!("  - Search Response Time: {:.2}ms", search_duration.as_millis());

    // Take final screenshot for audit report
    println!("\nCapturing final state screenshot...");
    client
        .call_tool(CallToolRequestParam {
            name: "screenshot".into(),
            arguments: None,
        })
        .await?;

    // Get console logs for any errors
    let console_logs = client
        .call_tool(CallToolRequestParam {
            name: "get_console_logs".into(),
            arguments: None,
        })
        .await?;

    if let Some(content) = console_logs.content.first() {
        println!("Console logs captured for audit");
        let logs_text = match &content.raw {
            rmcp::model::RawContent::Text(text) => &text.text,
            _ => ""
        };
        if logs_text.contains("error") || logs_text.contains("Error") {
            println!("⚠️  Errors detected in console logs");
        } else {
            println!("✅ No errors found in console logs");
        }
    }

    // Demo 5: Multi-driver health monitoring
    println!("\n🏥 Demo 5: Driver Health Monitoring");
    println!("====================================");

    // Get current driver health
    let health_result = client
        .call_tool(CallToolRequestParam {
            name: "get_healthy_endpoints".into(),
            arguments: None,
        })
        .await?;

    println!("Current healthy endpoints:");
    if let Some(content) = health_result.content.first() {
        let content_str = match &content.raw {
            rmcp::model::RawContent::Text(text) => &text.text,
            _ => "Non-text content"
        };
        println!("{}", content_str);
    }

    // Refresh health check
    client
        .call_tool(CallToolRequestParam {
            name: "refresh_driver_health".into(),
            arguments: None,
        })
        .await?;

    println!("Health check refreshed successfully");

    // List managed drivers
    let drivers_result = client
        .call_tool(CallToolRequestParam {
            name: "list_managed_drivers".into(),
            arguments: None,
        })
        .await?;

    println!("Managed drivers:");
    if let Some(content) = drivers_result.content.first() {
        let content_str = match &content.raw {
            rmcp::model::RawContent::Text(text) => &text.text,
            _ => "Non-text content"
        };
        println!("{}", content_str);
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    client
        .call_tool(CallToolRequestParam {
            name: "stop_all_drivers".into(),
            arguments: None,
        })
        .await?;

    client.cancel().await?;
    server_handle.abort();

    println!("🎉 Advanced monitoring demo completed!");
    println!("\n💡 Key Features Demonstrated:");
    println!("   • Built-in performance testing with custom test cases");
    println!("   • Real-time memory and resource monitoring");
    println!("   • E-commerce site performance auditing");
    println!("   • Console log analysis for error detection");
    println!("   • Multi-driver health monitoring and management");
    println!("   • Comprehensive metrics collection and reporting");
    println!("\n🚀 This crate provides enterprise-grade WebDriver testing capabilities");
    println!("   with built-in monitoring, performance analysis, and health management!");

    Ok(())
}
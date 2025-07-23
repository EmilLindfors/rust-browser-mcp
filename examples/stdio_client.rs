use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::CallToolRequestParam,
    object,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

    println!("🚀 WebDriver MCP Client - STDIO Transport Example");
    println!("Connecting to WebDriver MCP server using stdio...\n");

    // Connect to the webdriver-mcp server using stdio
    let client = ()
        .serve(TokioChildProcess::new(Command::new("cargo").configure(
            |cmd| {
                cmd.arg("run").arg("--bin").arg("webdriver-mcp");
            },
        ))?)
        .await?;

    // Initialize connection
    let server_info = client.peer_info();
    tracing::info!("Connected to WebDriver MCP server: {server_info:#?}");

    // List available tools
    let tools = client.list_all_tools().await?;
    println!("✅ Connected! Found {} available tools\n", tools.len());

    // Example workflow: Browser automation demo
    println!("🌐 Starting browser automation demo...");

    // Start a Firefox driver (fallback to Chrome if Firefox not available)
    let start_result = client
        .call_tool(CallToolRequestParam {
            name: "start_driver".into(),
            arguments: Some(object!({ "driver_type": "firefox" })),
        })
        .await;

    if start_result.is_err() {
        println!("Firefox not available, trying Chrome...");
        client
            .call_tool(CallToolRequestParam {
                name: "start_driver".into(),
                arguments: Some(object!({ "driver_type": "chrome" })),
            })
            .await?;
    }

    println!("✅ WebDriver started successfully");

    // Navigate to a website
    println!("🧭 Navigating to Rust website...");
    client
        .call_tool(CallToolRequestParam {
            name: "navigate".into(),
            arguments: Some(object!({ "url": "https://www.rust-lang.org" })),
        })
        .await?;

    // Get page title
    let title_result = client
        .call_tool(CallToolRequestParam {
            name: "get_title".into(),
            arguments: None,
        })
        .await?;
    println!("✅ Page loaded: {:?}", title_result.content.first());

    // Take a screenshot
    println!("📸 Taking screenshot...");
    let _screenshot_result = client
        .call_tool(CallToolRequestParam {
            name: "screenshot".into(),
            arguments: None,
        })
        .await?;
    println!("✅ Screenshot captured");

    // Find elements on the page
    println!("🔍 Finding navigation links...");
    let find_result = client
        .call_tool(CallToolRequestParam {
            name: "find_elements".into(),
            arguments: Some(object!({ "selector": "nav a" })),
        })
        .await?;
    println!(
        "✅ Found navigation elements: {:?}",
        find_result.content.first()
    );

    // Execute JavaScript
    println!("⚡ Executing JavaScript...");
    let script_result = client
        .call_tool(CallToolRequestParam {
            name: "execute_script".into(),
            arguments: Some(object!({ "script": "return document.readyState;" })),
        })
        .await?;
    println!("✅ Page ready state: {:?}", script_result.content.first());

    // Cleanup
    println!("🧹 Cleaning up...");
    client
        .call_tool(CallToolRequestParam {
            name: "stop_all_drivers".into(),
            arguments: None,
        })
        .await?;

    client.cancel().await?;

    println!("🎉 STDIO client demo completed successfully!");
    Ok(())
}

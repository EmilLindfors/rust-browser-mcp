use anyhow::Result;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    object,
    transport::StreamableHttpClientTransport,
};
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

    println!("🚀 WebDriver MCP Client - HTTP Transport Example");
    println!("Starting HTTP server and connecting...\n");

    // Start the HTTP server in the background
    let server_handle = tokio::spawn(async {
        let mut cmd = tokio::process::Command::new("cargo");
        cmd.args(&[
            "run",
            "--features",
            "http-server",
            "--bin",
            "webdriver-mcp",
            "--",
            "--transport",
            "http",
            "--bind",
            "127.0.0.1:8090",
        ]);
        cmd.kill_on_drop(true);
        let mut child = cmd.spawn().expect("Failed to start HTTP server");
        child.wait().await.expect("Server process failed")
    });

    // Wait for server to start
    println!("⏳ Waiting for HTTP server to start...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Connect to the HTTP server
    let transport = StreamableHttpClientTransport::from_uri("http://127.0.0.1:8090/mcp");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "webdriver-mcp-http-client".to_string(),
            version: "0.1.0".to_string(),
        },
    };

    let client = client_info.serve(transport).await?;

    // Initialize connection
    let server_info = client.peer_info();
    tracing::info!("Connected to WebDriver MCP HTTP server: {server_info:#?}");

    // List available tools
    let tools = client.list_all_tools().await?;
    println!(
        "✅ Connected via HTTP! Found {} available tools\n",
        tools.len()
    );

    // Example workflow: Browser automation demo
    println!("🌐 Starting browser automation demo via HTTP...");

    // Start a Chrome driver (fallback to Firefox if Chrome not available)
    let start_result = client
        .call_tool(CallToolRequestParam {
            name: "start_driver".into(),
            arguments: Some(object!({ "driver_type": "chrome" })),
        })
        .await;

    if start_result.is_err() {
        println!("Chrome not available, trying Firefox...");
        client
            .call_tool(CallToolRequestParam {
                name: "start_driver".into(),
                arguments: Some(object!({ "driver_type": "firefox" })),
            })
            .await?;
    }

    println!("✅ WebDriver started successfully");

    // Navigate to a test page
    println!("🧭 Navigating to HTTPBin (great for testing)...");
    client
        .call_tool(CallToolRequestParam {
            name: "navigate".into(),
            arguments: Some(object!({ "url": "https://httpbin.org/forms/post" })),
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

    // Fill out a form (demonstrating advanced workflow)
    println!("📝 Filling out test form...");
    client
        .call_tool(CallToolRequestParam {
            name: "fill_and_submit_form".into(),
            arguments: Some(object!({
                "fields": {
                    "input[name='custname']": "HTTP Client Test",
                    "input[name='custtel']": "555-1234",
                    "input[name='custemail']": "test@example.com"
                },
                "submit_selector": "input[type='submit']"
            })),
        })
        .await?;
    println!("✅ Form submitted successfully");

    // Take a screenshot of the result
    println!("📸 Taking screenshot of result...");
    let _screenshot_result = client
        .call_tool(CallToolRequestParam {
            name: "screenshot".into(),
            arguments: None,
        })
        .await?;
    println!("✅ Screenshot captured");

    // Get the current URL to verify the form submission
    let url_result = client
        .call_tool(CallToolRequestParam {
            name: "get_current_url".into(),
            arguments: None,
        })
        .await?;
    println!(
        "✅ Current URL after form: {:?}",
        url_result.content.first()
    );

    // Execute JavaScript to get form data
    println!("⚡ Getting page information via JavaScript...");
    let script_result = client
        .call_tool(CallToolRequestParam {
            name: "execute_script".into(),
            arguments: Some(object!({
                "script": "return {title: document.title, url: window.location.href, readyState: document.readyState};"
            })),
        })
        .await?;
    println!("✅ Page info: {:?}", script_result.content.first());

    // List managed drivers
    let drivers_result = client
        .call_tool(CallToolRequestParam {
            name: "list_managed_drivers".into(),
            arguments: None,
        })
        .await?;
    println!("🖥️  Managed drivers: {:?}", drivers_result.content.first());

    // Cleanup
    println!("🧹 Cleaning up...");
    client
        .call_tool(CallToolRequestParam {
            name: "stop_all_drivers".into(),
            arguments: None,
        })
        .await?;

    client.cancel().await?;
    server_handle.abort();

    println!("🎉 HTTP client demo completed successfully!");
    Ok(())
}

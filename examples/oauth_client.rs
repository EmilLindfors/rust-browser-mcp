use anyhow::{Context, Result};
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParam, ClientCapabilities, ClientInfo, Implementation},
    object,
    transport::{StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig},
    service::Service,
};
use serde_json::Value;
use std::io::{self, Write};
use tokio::time::{sleep, Duration};

/// Example OAuth client for WebDriver MCP
/// Demonstrates how to authenticate and use the OAuth-protected MCP server

struct OAuthMcpClient {
    base_url: String,
    access_token: Option<String>,
}

impl OAuthMcpClient {
    fn new(base_url: String) -> Self {
        Self {
            base_url,
            access_token: None,
        }
    }

    /// Manual OAuth flow - user needs to visit URL and get token
    async fn authenticate_manual(&mut self) -> Result<()> {
        println!("🔐 Manual OAuth Authentication");
        println!("===============================");
        println!();
        println!("1. Open your browser and visit: {}/oauth/authorize", self.base_url);
        println!("2. Complete the authorization process");
        println!("3. Copy the access token from the success page");
        println!();
        
        print!("📝 Enter your access token: ");
        io::stdout().flush()?;
        
        let mut token = String::new();
        io::stdin().read_line(&mut token)?;
        let token = token.trim().to_string();
        
        if token.is_empty() {
            return Err(anyhow::anyhow!("No token provided"));
        }
        
        self.access_token = Some(token);
        println!("✅ Token stored successfully!");
        Ok(())
    }

    /// Create an authenticated MCP client
    async fn create_mcp_client(&self) -> Result<Service<StreamableHttpClientTransport>> {
        let token = self.access_token.as_ref()
            .context("Not authenticated. Call authenticate_manual() first")?;

        // Create HTTP client with authorization header
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                .context("Failed to create authorization header")?
        );

        let http_client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        // Create transport with custom HTTP client
        let config = StreamableHttpClientTransportConfig {
            uri: format!("{}/mcp", self.base_url).into(),
            ..Default::default()
        };
        let transport = StreamableHttpClientTransport::with_client(http_client, config);

        let client_info = ClientInfo {
            protocol_version: Default::default(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "oauth-webdriver-mcp-client".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let client = client_info.serve(transport).await
            .context("Failed to connect to MCP server")?;

        Ok(client)
    }

    /// List available tools
    async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>> {
        println!("📋 Listing available tools...");
        
        let client = self.create_mcp_client().await?;
        let tools = client.list_all_tools().await
            .context("Failed to list tools")?;
        
        Ok(tools)
    }

    /// Call a specific tool
    async fn call_tool(&self, tool_name: &str, arguments: Option<Value>) -> Result<rmcp::model::CallToolResult> {
        println!("🔧 Calling tool: {}", tool_name);
        
        let client = self.create_mcp_client().await?;
        let result = client.call_tool(CallToolRequestParam {
            name: tool_name.into(),
            arguments: arguments.or(Some(object!({}))),
        }).await
        .context("Failed to call tool")?;
        
        Ok(result)
    }

    /// Test WebDriver functionality
    async fn test_webdriver(&self) -> Result<()> {
        println!("🌐 Testing WebDriver functionality...");
        println!();

        // Test 1: Start browser driver
        println!("1️⃣ Starting WebDriver...");
        let start_result = self.call_tool("start_driver", Some(serde_json::json!({
            "driver_type": "firefox"
        }))).await;
        
        match start_result {
            Ok(result) => println!("   ✅ Driver started: {:?}", result.content.first()),
            Err(e) => println!("   ⚠️  Driver start warning: {}", e),
        }

        sleep(Duration::from_secs(2)).await;

        // Test 2: Navigate to a webpage
        println!("2️⃣ Navigating to example.com...");
        let nav_result = self.call_tool("navigate", Some(serde_json::json!({
            "url": "https://example.com"
        }))).await?;
        println!("   ✅ Navigation result: {:?}", nav_result.content.first());

        sleep(Duration::from_secs(3)).await;

        // Test 3: Get page title
        println!("3️⃣ Getting page title...");
        let title_result = self.call_tool("get_title", None).await?;
        println!("   ✅ Page title: {:?}", title_result.content.first());

        // Test 4: Take a screenshot
        println!("4️⃣ Taking screenshot...");
        let screenshot_result = self.call_tool("screenshot", None).await?;
        let screenshot_len = screenshot_result.content.first()
            .map(|c| match &c.raw {
                rmcp::model::RawContent::Text(text) => text.len(),
                rmcp::model::RawContent::Resource(_) => 0,
            })
            .unwrap_or(0);
        println!("   ✅ Screenshot taken (base64 length: {} chars)", screenshot_len);

        // Test 5: Get current URL
        println!("5️⃣ Getting current URL...");
        let url_result = self.call_tool("get_current_url", None).await?;
        println!("   ✅ Current URL: {:?}", url_result.content.first());

        Ok(())
    }
}

async fn interactive_demo(client: &OAuthMcpClient) -> Result<()> {
    println!("\n🎮 Interactive Demo Mode");
    println!("======================");
    println!("Available commands:");
    println!("  1 - List all tools");
    println!("  2 - Test WebDriver functionality");
    println!("  3 - Navigate to custom URL");
    println!("  4 - Get current URL");
    println!("  5 - Quit browser session");
    println!("  q - Quit demo");
    println!();

    loop {
        print!("demo> ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            "1" => {
                match client.list_tools().await {
                    Ok(tools) => {
                        println!("\n📋 Available Tools:");
                        for tool in tools {
                            println!("  • {} - {}", tool.name, tool.description.unwrap_or("No description".into()));
                        }
                        println!();
                    }
                    Err(e) => println!("❌ Error: {}", e),
                }
            }
            "2" => {
                if let Err(e) = client.test_webdriver().await {
                    println!("❌ WebDriver test failed: {}", e);
                }
            }
            "3" => {
                print!("Enter URL: ");
                io::stdout().flush()?;
                let mut url = String::new();
                io::stdin().read_line(&mut url)?;
                let url = url.trim();
                
                match client.call_tool("navigate", Some(serde_json::json!({"url": url}))).await {
                    Ok(result) => println!("✅ Navigated: {:?}", result.content.first()),
                    Err(e) => println!("❌ Navigation failed: {}", e),
                }
            }
            "4" => {
                match client.call_tool("get_current_url", None).await {
                    Ok(result) => println!("📍 Current URL: {:?}", result.content.first()),
                    Err(e) => println!("❌ Failed to get URL: {}", e),
                }
            }
            "5" => {
                match client.call_tool("stop_all_drivers", None).await {
                    Ok(result) => println!("🚪 All drivers stopped: {:?}", result.content.first()),
                    Err(e) => println!("❌ Failed to stop drivers: {}", e),
                }
            }
            "q" => {
                println!("👋 Goodbye!");
                break;
            }
            "" => continue,
            _ => {
                println!("❓ Unknown command: {}", input);
                println!("Available: 1, 2, 3, 4, 5, q");
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 WebDriver MCP OAuth Client Demo");
    println!("==================================");
    println!();

    // Create client
    let server_url = std::env::var("MCP_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let mut client = OAuthMcpClient::new(server_url.clone());
    
    println!("🔗 Server URL: {}", server_url);
    println!();

    // Check if token is provided via environment
    if let Ok(token) = std::env::var("MCP_ACCESS_TOKEN") {
        println!("🔑 Using token from environment variable");
        client.access_token = Some(token);
    } else {
        // Manual authentication
        client.authenticate_manual().await?;
    }

    println!();

    // Verify authentication by listing tools
    match client.list_tools().await {
        Ok(tools) => {
            println!("✅ Authentication successful!");
            println!("📋 Found {} tools:", tools.len());
            for tool in &tools {
                println!("  • {}", tool.name);
            }
            println!();
        }
        Err(e) => {
            println!("❌ Authentication failed: {}", e);
            println!("💡 Make sure:");
            println!("   1. The MCP server is running");
            println!("   2. Your access token is valid");
            println!("   3. The server URL is correct");
            return Ok(());
        }
    }

    // Ask user what they want to do
    println!("What would you like to do?");
    println!("  1 - Run automated WebDriver test");
    println!("  2 - Interactive demo mode");
    println!();
    print!("Choose (1 or 2): ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    let choice = choice.trim();

    match choice {
        "1" => {
            println!("\n🚀 Running automated WebDriver test...");
            if let Err(e) = client.test_webdriver().await {
                println!("❌ Test failed: {}", e);
            } else {
                println!("\n🎉 All tests completed successfully!");
            }
        }
        "2" => {
            interactive_demo(&client).await?;
        }
        _ => {
            println!("❓ Invalid choice, running automated test...");
            if let Err(e) = client.test_webdriver().await {
                println!("❌ Test failed: {}", e);
            }
        }
    }

    println!("\n✨ Demo completed!");
    Ok(())
}
use anyhow::{Context, Result};
use reqwest::Client;
use std::io::{self, Write};

/// Simplified OAuth client for WebDriver MCP
/// Demonstrates OAuth flow and shows how to use the token
struct SimpleOAuthClient {
    client: Client,
    base_url: String,
    access_token: Option<String>,
}

impl SimpleOAuthClient {
    fn new(base_url: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            access_token: None,
        }
    }

    /// Manual OAuth flow - user needs to visit URL and get token
    async fn authenticate_manual(&mut self) -> Result<()> {
        println!("🔐 OAuth Authentication Flow");
        println!("=============================");
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

    /// Test token validity by making a simple HTTP request
    async fn test_token(&self) -> Result<()> {
        let token = self.access_token.as_ref()
            .context("Not authenticated. Call authenticate_manual() first")?;

        println!("🔍 Testing token validity...");

        // Test with a simple HTTP request to the protected endpoint
        let response = self.client
            .post(&format!("{}/mcp", self.base_url))
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream")
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "ping"
            }))
            .send()
            .await
            .context("Failed to send test request")?;

        if response.status().is_success() {
            println!("✅ Token is valid! You can now use it with MCP clients.");
        } else {
            let status = response.status();
            let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            println!("❌ Token test failed: HTTP {} - {}", status, text);
        }

        Ok(())
    }

    /// Show examples of how to use the token
    fn show_usage_examples(&self) {
        let token = match &self.access_token {
            Some(token) => token,
            None => {
                println!("❌ No token available. Authenticate first.");
                return;
            }
        };

        println!("\n🚀 How to use your OAuth token:");
        println!("===============================");
        
        println!("\n1️⃣ **Direct curl commands:**");
        println!("```bash");
        println!("# List available tools");
        println!("curl -H \"Authorization: Bearer {}\" \\", token);
        println!("     -H \"Content-Type: application/json\" \\");
        println!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}}' \\");
        println!("     {}/mcp", self.base_url);
        println!();
        println!("# Navigate to a webpage");
        println!("curl -H \"Authorization: Bearer {}\" \\", token);
        println!("     -H \"Content-Type: application/json\" \\");
        println!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"navigate\",\"arguments\":{{\"url\":\"https://example.com\"}}}}}}' \\");
        println!("     {}/mcp", self.base_url);
        println!("```");

        println!("\n2️⃣ **Environment variables for MCP clients:**");
        println!("```bash");
        println!("export MCP_SERVER_URL=\"{}/mcp\"", self.base_url);
        println!("export MCP_ACCESS_TOKEN=\"{}\"", token);
        println!("```");

        println!("\n3️⃣ **Claude MCP configuration:**");
        println!("```json");
        println!("{{");
        println!("  \"mcpServers\": {{");
        println!("    \"webdriver\": {{");
        println!("      \"command\": \"your-mcp-http-client\",");
        println!("      \"env\": {{");
        println!("        \"MCP_SERVER_URL\": \"{}/mcp\",", self.base_url);
        println!("        \"MCP_AUTH_HEADER\": \"Authorization: Bearer {}\"", token);
        println!("      }}");
        println!("    }}");
        println!("  }}");
        println!("}}");
        println!("```");

        println!("\n4️⃣ **Python MCP client example:**");
        println!("```python");
        println!("import os");
        println!("import asyncio");
        println!("from mcp import ClientSession, StdioServerParameters");
        println!();
        println!("async def main():");
        println!("    # Set environment variables");
        println!("    os.environ['MCP_SERVER_URL'] = '{}/mcp'", self.base_url);
        println!("    os.environ['MCP_AUTH_HEADER'] = 'Authorization: Bearer {}'", token);
        println!("    ");
        println!("    # Connect to MCP server with authentication");
        println!("    # (Implementation depends on your MCP client library)");
        println!("```");

        println!("\n💡 **Token Management Tips:**");
        println!("• This token expires in 1 hour (for demo mode)");
        println!("• Store it securely and don't commit it to version control");
        println!("• Generate a new token when this one expires");
        println!("• In production, implement proper token refresh logic");
    }
}

async fn interactive_menu(client: &mut SimpleOAuthClient) -> Result<()> {
    loop {
        println!("\n🎮 OAuth Client Menu");
        println!("====================");
        println!("1. 🔐 Get OAuth token");
        println!("2. 🧪 Test token validity");
        println!("3. 📖 Show usage examples");
        println!("4. 🌐 Open authorization URL in browser (if supported)");
        println!("5. ❌ Exit");
        print!("\nChoose an option (1-5): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        match choice {
            "1" => {
                if let Err(e) = client.authenticate_manual().await {
                    println!("❌ Authentication failed: {}", e);
                }
            }
            "2" => {
                if let Err(e) = client.test_token().await {
                    println!("❌ Token test failed: {}", e);
                }
            }
            "3" => {
                client.show_usage_examples();
            }
            "4" => {
                let auth_url = format!("{}/oauth/authorize", client.base_url);
                println!("🌐 Authorization URL: {}", auth_url);
                
                // Try to open in browser (best effort)
                if let Err(_) = webbrowser::open(&auth_url) {
                    println!("💡 Could not auto-open browser. Please copy the URL above manually.");
                } else {
                    println!("✅ Opened authorization URL in your default browser!");
                }
            }
            "5" => {
                println!("👋 Goodbye!");
                break;
            }
            "" => continue, // Empty input, just re-show menu
            _ => {
                println!("❓ Invalid choice. Please enter 1-5.");
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 WebDriver MCP OAuth Client");
    println!("==============================");
    println!("This tool helps you get OAuth tokens for the WebDriver MCP server");
    println!("and shows you how to use them with various MCP clients.");
    println!();

    // Get server URL from environment or use default
    let server_url = std::env::var("MCP_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    let mut client = SimpleOAuthClient::new(server_url.clone());
    
    println!("🔗 Server URL: {}", server_url);

    // Check if token is provided via environment
    if let Ok(token) = std::env::var("MCP_ACCESS_TOKEN") {
        println!("🔑 Using token from environment variable");
        client.access_token = Some(token);
        
        // Test the token
        if let Err(e) = client.test_token().await {
            println!("⚠️  Environment token test failed: {}", e);
            println!("You may need to get a new token.");
        }
    }

    // Check command line arguments for non-interactive mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--get-token" => {
                client.authenticate_manual().await?;
                client.show_usage_examples();
                return Ok(());
            }
            "--test-token" => {
                client.test_token().await?;
                return Ok(());
            }
            "--help" => {
                println!("Usage:");
                println!("  {} [OPTIONS]", args[0]);
                println!();
                println!("Options:");
                println!("  --get-token    Get OAuth token and show usage examples");
                println!("  --test-token   Test existing token (from env MCP_ACCESS_TOKEN)");
                println!("  --help         Show this help message");
                println!();
                println!("Environment Variables:");
                println!("  MCP_SERVER_URL      Server URL (default: http://localhost:3000)");
                println!("  MCP_ACCESS_TOKEN    Existing token to test");
                return Ok(());
            }
            _ => {
                println!("❓ Unknown option: {}", args[1]);
                println!("Use --help for usage information.");
                return Ok(());
            }
        }
    }

    // Interactive mode
    interactive_menu(&mut client).await?;

    Ok(())
}
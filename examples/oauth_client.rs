use anyhow::{Context, Result};
use std::io::{self, Write};

/// Simple OAuth client example for WebDriver MCP
/// This example shows how to manually get and use an OAuth token
/// with the WebDriver MCP server
/// 
/// NOTE: This is a simplified example. For a full OAuth implementation using rmcp's 
/// built-in OAuth support, see the rust-sdk/examples/clients/src/auth/oauth_client.rs

#[tokio::main]
async fn main() -> Result<()> {
    println!("WebDriver MCP OAuth Client Demo");
    println!("===============================");
    println!();
    println!("This example demonstrates OAuth authentication with WebDriver MCP.");
    println!();
    
    // Get server URL from environment or use default
    let server_url = std::env::var("MCP_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_string());
    
    println!("Server URL: {}", server_url);
    println!();

    // Check if token is provided via environment
    if let Ok(token) = std::env::var("MCP_ACCESS_TOKEN") {
        println!("Using token from environment variable");
        test_token_with_http(&server_url, &token).await?;
    } else {
        // Manual authentication flow
        println!("Manual OAuth Authentication");
        println!("===========================");
        println!();
        println!("1. Open your browser and visit: {}/oauth/authorize", server_url);
        println!("2. Complete the authorization process");
        println!("3. Copy the access token from the success page");
        println!();

        print!("Enter your access token: ");
        io::stdout().flush()?;

        let mut token = String::new();
        io::stdin().read_line(&mut token)?;
        let token = token.trim().to_string();

        if token.is_empty() {
            return Err(anyhow::anyhow!("No token provided"));
        }

        println!("Token stored successfully!");
        println!();
        
        test_token_with_http(&server_url, &token).await?;
    }

    show_usage_examples(&server_url);
    
    Ok(())
}

async fn test_token_with_http(server_url: &str, token: &str) -> Result<()> {
    println!("Testing token validity...");
    
    let client = reqwest::Client::new();
    
    // Test with a simple MCP request
    let response = client
        .post(&format!("{}/mcp", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/list"
        }))
        .send()
        .await
        .context("Failed to send test request")?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        
        if let Some(tools) = result.get("result").and_then(|r| r.get("tools")) {
            let tool_count = tools.as_array().map(|arr| arr.len()).unwrap_or(0);
            println!("Authentication successful! Found {} tools available.", tool_count);
            
            if let Some(tool_array) = tools.as_array() {
                println!("\nAvailable tools:");
                for tool in tool_array.iter().take(10) { // Show first 10 tools
                    if let Some(name) = tool.get("name").and_then(|n| n.as_str()) {
                        println!("  - {}", name);
                    }
                }
                if tool_array.len() > 10 {
                    println!("  ... and {} more", tool_array.len() - 10);
                }
            }
        } else {
            println!("Token is valid but unexpected response format");
        }
        
        println!();
        
        // Test a WebDriver operation
        println!("Testing WebDriver functionality...");
        test_webdriver_operations(&client, server_url, token).await?;
        
    } else {
        let status = response.status();
        let text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        println!("Token test failed: HTTP {} - {}", status, text);
        println!("Make sure:");
        println!("  1. The MCP server is running");
        println!("  2. Your access token is valid");
        println!("  3. The server URL is correct");
    }

    Ok(())
}

async fn test_webdriver_operations(client: &reqwest::Client, server_url: &str, token: &str) -> Result<()> {
    // Test starting a driver
    let response = client
        .post(&format!("{}/mcp", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "start_driver",
                "arguments": {
                    "driver_type": "firefox"
                }
            }
        }))
        .send()
        .await
        .context("Failed to start driver")?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        println!("  - Driver start: OK");
        
        if let Some(error) = result.get("error") {
            println!("    Warning: {}", error);
        }
    } else {
        println!("  - Driver start: Failed ({})", response.status());
    }

    // Test navigation
    let response = client
        .post(&format!("{}/mcp", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "navigate",
                "arguments": {
                    "url": "https://example.com"
                }
            }
        }))
        .send()
        .await
        .context("Failed to navigate")?;

    if response.status().is_success() {
        println!("  - Navigation: OK");
    } else {
        println!("  - Navigation: Failed ({})", response.status());
    }

    // Test getting page title
    let response = client
        .post(&format!("{}/mcp", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_title",
                "arguments": {}
            }
        }))
        .send()
        .await
        .context("Failed to get title")?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        if let Some(content) = result.get("result")
            .and_then(|r| r.get("content"))
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
        {
            println!("  - Page title: {}", content);
        } else {
            println!("  - Page title: Retrieved (but couldn't parse)");
        }
    } else {
        println!("  - Page title: Failed ({})", response.status());
    }

    // Cleanup
    let _cleanup_response = client
        .post(&format!("{}/mcp", server_url))
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "stop_all_drivers",
                "arguments": {}
            }
        }))
        .send()
        .await;

    println!("  - Cleanup: Done");
    println!();
    println!("WebDriver functionality test completed!");

    Ok(())
}

fn show_usage_examples(server_url: &str) {
    println!("\nHow to use your OAuth token:");
    println!("============================");
    
    println!("\n1. Direct curl commands:");
    println!("```bash");
    println!("# List available tools");
    println!("curl -H \"Authorization: Bearer YOUR_TOKEN\" \\");
    println!("     -H \"Content-Type: application/json\" \\");
    println!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}}' \\");
    println!("     {}/mcp", server_url);
    println!();
    println!("# Navigate to a webpage");
    println!("curl -H \"Authorization: Bearer YOUR_TOKEN\" \\");
    println!("     -H \"Content-Type: application/json\" \\");
    println!("     -d '{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/call\",\"params\":{{\"name\":\"navigate\",\"arguments\":{{\"url\":\"https://example.com\"}}}}}}' \\");
    println!("     {}/mcp", server_url);
    println!("```");

    println!("\n2. Environment variables for MCP clients:");
    println!("```bash");
    println!("export MCP_SERVER_URL=\"{}/mcp\"", server_url);
    println!("export MCP_ACCESS_TOKEN=\"your_token_here\"");
    println!("```");

    println!("\n3. Claude MCP configuration:");
    println!("```json");
    println!("{{");
    println!("  \"mcpServers\": {{");
    println!("    \"webdriver\": {{");
    println!("      \"command\": \"your-mcp-http-client\",");
    println!("      \"env\": {{");
    println!("        \"MCP_SERVER_URL\": \"{}/mcp\",", server_url);
    println!("        \"MCP_AUTH_HEADER\": \"Authorization: Bearer YOUR_TOKEN\"");
    println!("      }}");
    println!("    }}");
    println!("  }}");
    println!("}}");
    println!("```");

    println!("\nToken Management Tips:");
    println!("- This token expires in 1 hour (for demo mode)");
    println!("- Store it securely and don't commit it to version control");
    println!("- Generate a new token when this one expires");
    println!("- In production, implement proper token refresh logic");
    
    println!("\nFor a full OAuth implementation using rmcp's built-in support,");
    println!("see: rust-sdk/examples/clients/src/auth/oauth_client.rs");
}
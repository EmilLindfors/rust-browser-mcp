# WebDriver MCP Examples

This directory contains examples demonstrating different ways to use the WebDriver MCP server.

## Examples Overview

### 1. Basic Examples

- **`stdio_client.rs`** - Basic client using stdio transport
- **`http_client.rs`** - Basic client using HTTP transport (no auth)

### 2. OAuth Examples

- **`oauth_demo_server.rs`** - Complete OAuth-protected MCP server with demo authentication
- **`oauth_client.rs`** - Client that authenticates with OAuth-protected server
- **`keycloak-setup.md`** - Step-by-step guide for setting up Keycloak authentication

## Running the Examples

### Prerequisites

Make sure you have geckodriver installed:

```bash
# Download and install geckodriver
wget https://github.com/mozilla/geckodriver/releases/latest/download/geckodriver-v0.34.0-linux64.tar.gz
tar -xzf geckodriver-v0.34.0-linux64.tar.gz
sudo mv geckodriver /usr/local/bin/
```

### OAuth Demo Server

The OAuth demo server provides a complete example of OAuth-protected WebDriver MCP server:

```bash
# Start the OAuth demo server
cargo run --example oauth_demo_server --features http-server

# The server will be available at:
# - Main page: http://localhost:3000
# - OAuth authorization: http://localhost:3000/oauth/authorize  
# - Protected MCP endpoint: http://localhost:3000/mcp
```

**Features:**
- Beautiful web interface for OAuth authorization
- Demo mode (no external OAuth provider needed)
- Complete WebDriver MCP functionality
- Token-based authentication
- Interactive authorization flow

### OAuth Client

Test the OAuth-protected server with the example client:

```bash
# Run the OAuth client
cargo run --example oauth_client

# Or with environment variables:
MCP_SERVER_URL=http://localhost:3000 cargo run --example oauth_client

# With pre-existing token:
MCP_ACCESS_TOKEN=your_token_here cargo run --example oauth_client
```

**Client Features:**
- Manual OAuth flow (opens browser)
- Automated WebDriver testing
- Interactive demo mode
- Tool listing and calling
- Error handling and user-friendly output

## OAuth Flow Walkthrough

### 1. Start the Demo Server

```bash
cargo run --example oauth_demo_server --features http-server
```

You'll see output like:
```
🚀 Starting WebDriver MCP Demo OAuth Server
📍 Server will be available at: http://127.0.0.1:3000
🔐 Authorization URL: http://127.0.0.1:3000/oauth/authorize
🤖 MCP Endpoint: http://127.0.0.1:3000/mcp (requires token)

✅ Server started successfully!
👉 Visit http://127.0.0.1:3000 to get started
```

### 2. Get an Access Token

**Option A: Via Web Browser**
1. Visit http://localhost:3000
2. Click "Get Access Token"
3. Click "Authorize Access"
4. Copy the generated token

**Option B: Via Client**  
```bash
cargo run --example oauth_client
# Follow the prompts to get a token
```

### 3. Use the Token

Test the protected MCP endpoint:

```bash
curl -H "Authorization: Bearer YOUR_TOKEN_HERE" \
     -H "Content-Type: application/json" \
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
     http://localhost:3000/mcp
```

## Production Setup with Keycloak

For production use with real OAuth provider (Keycloak):

### 1. Set up Keycloak

Follow the detailed guide in [`keycloak-setup.md`](keycloak-setup.md)

### 2. Configure the Server

Update the OAuth configuration in your server:

```rust
// Use environment variables for production
let oauth_config = OAuthConfig {
    server_url: env::var("KEYCLOAK_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    realm: env::var("KEYCLOAK_REALM")
        .unwrap_or_else(|_| "webdriver-mcp".to_string()),
    client_id: env::var("KEYCLOAK_CLIENT_ID")
        .unwrap_or_else(|_| "webdriver-mcp-server".to_string()),
    client_secret: env::var("KEYCLOAK_CLIENT_SECRET").ok(),
    redirect_uri: env::var("OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/oauth/callback".to_string()),
    scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
};
```

### 3. Run with Production Config

```bash
# Set environment variables
export KEYCLOAK_URL="http://localhost:8080"
export KEYCLOAK_REALM="webdriver-mcp"
export KEYCLOAK_CLIENT_ID="webdriver-mcp-server"
export KEYCLOAK_CLIENT_SECRET="your-client-secret"
export OAUTH_REDIRECT_URI="http://localhost:3000/oauth/callback"

# Start the main server (not demo)
cargo run --features http-server -- --transport http --bind 127.0.0.1:3000
```

## Integration with Claude

To use the OAuth-protected server with Claude or other MCP clients:

### 1. MCP Client Configuration

```json
{
  "mcpServers": {
    "webdriver": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-http"],
      "env": {
        "MCP_SERVER_URL": "http://localhost:3000/mcp",
        "MCP_AUTH_HEADER": "Authorization: Bearer YOUR_TOKEN_HERE"
      }
    }
  }
}
```

### 2. Token Management

For production use, implement token refresh logic:

```javascript
// Pseudo-code for MCP client
const token = await refreshTokenIfNeeded(currentToken);
const headers = {
  'Authorization': `Bearer ${token}`,
  'Content-Type': 'application/json'
};
```

## Testing and Debugging

### Enable Debug Logging

```bash
RUST_LOG=debug cargo run --example oauth_demo_server --features http-server
```

### Test Individual Components

```bash
# Test OAuth flow only
curl http://localhost:3000/oauth/authorize

# Test token validation
curl -H "Authorization: Bearer invalid_token" http://localhost:3000/mcp

# Test health endpoint
curl http://localhost:3000/health
```

### Common Issues

1. **"Invalid token"** - Check token format and expiration
2. **"WebDriver not found"** - Install geckodriver
3. **"Connection refused"** - Ensure server is running on correct port
4. **CORS errors** - Check if client origin is allowed

## Security Considerations

### Development vs Production

**Development (Demo Mode):**
- Uses simple in-memory token storage
- No external dependencies
- Tokens don't expire (for testing)
- HTTP is acceptable

**Production:**
- Use proper OAuth provider (Keycloak, Auth0, etc.)
- Implement token refresh
- Use HTTPS only
- Set appropriate CORS policies
- Monitor token usage
- Implement rate limiting

### Token Security

- Never log tokens in production
- Use short expiration times
- Implement token rotation
- Store tokens securely on client side
- Use HTTPS for all OAuth flows

## Extending the Examples

### Adding New OAuth Providers

To add support for other OAuth providers:

1. Implement provider-specific client in `src/keycloak.rs`
2. Update `OAuthConfig` with provider-specific fields
3. Add discovery endpoints for provider metadata
4. Test with provider's sandbox environment

### Custom Authentication Logic

To add custom authentication (e.g., API keys, JWT):

1. Create new middleware in `src/oauth.rs`
2. Implement token validation logic
3. Add configuration options
4. Update examples with new auth method

### Advanced WebDriver Features

Examples can be extended to demonstrate:
- Multiple browser sessions
- Cross-browser testing
- Screenshot comparison
- Page performance metrics
- Mobile browser automation

## Contributing

When adding new examples:

1. Follow the existing naming convention
2. Add comprehensive documentation
3. Include error handling
4. Add example to this README
5. Test with both demo and production modes
6. Consider security implications
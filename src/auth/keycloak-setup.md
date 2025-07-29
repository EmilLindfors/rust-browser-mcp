# Keycloak Setup for WebDriver MCP OAuth

This guide shows how to configure Keycloak for OAuth authentication with the WebDriver MCP server.

## 1. Keycloak Installation

### Using Docker (Recommended for Development)

```bash
# Start Keycloak with Docker
docker run -d \
  --name keycloak \
  -p 8080:8080 \
  -e KEYCLOAK_ADMIN=admin \
  -e KEYCLOAK_ADMIN_PASSWORD=admin \
  quay.io/keycloak/keycloak:latest \
  start-dev
```

### Using Docker Compose

Create `docker-compose.yml`:

```yaml
version: '3.8'
services:
  keycloak:
    image: quay.io/keycloak/keycloak:latest
    environment:
      KEYCLOAK_ADMIN: admin
      KEYCLOAK_ADMIN_PASSWORD: admin
      KC_DB: h2
    ports:
      - "8080:8080"
    command: ["start-dev"]
    volumes:
      - keycloak_data:/opt/keycloak/data

volumes:
  keycloak_data:
```

Start with: `docker-compose up -d`

## 2. Keycloak Configuration

### Access Keycloak Admin Console
- URL: http://localhost:8080/admin
- Username: `admin`
- Password: `admin`

### Create a Realm
1. Click "Create Realm"
2. Name: `webdriver-mcp`
3. Click "Create"

### Create a Client
1. Go to "Clients" → "Create client"
2. **General Settings:**
   - Client type: `OpenID Connect`
   - Client ID: `webdriver-mcp-server`
   - Name: `WebDriver MCP Server`

3. **Capability config:**
   - Client authentication: `ON`
   - Authorization: `OFF`
   - Standard flow: `ON`
   - Direct access grants: `ON`

4. **Login settings:**
   - Valid redirect URIs: `http://localhost:3000/oauth/callback`
   - Web origins: `http://localhost:3000`

5. **Advanced Settings:**
   - Proof Key for Code Exchange Code Challenge Method: `S256`

### Get Client Secret
1. Go to "Clients" → "webdriver-mcp-server" → "Credentials"
2. Copy the "Client secret" value

### Create Users (Optional)
1. Go to "Users" → "Create new user"
2. Fill in username, email, etc.
3. Go to "Credentials" tab → "Set password"

## 3. WebDriver MCP Configuration

Update your WebDriver MCP server configuration:

```rust
// In src/oauth.rs or src/main.rs
let oauth_config = OAuthConfig {
    server_url: "http://localhost:8080".to_string(),
    realm: "webdriver-mcp".to_string(),
    client_id: "webdriver-mcp-server".to_string(),
    client_secret: Some("YOUR_CLIENT_SECRET_HERE".to_string()),
    redirect_uri: "http://localhost:3000/oauth/callback".to_string(),
    scopes: vec![
        "openid".to_string(),
        "profile".to_string(),
        "email".to_string()
    ],
};
```

## 4. Environment Variables (Recommended)

Instead of hardcoding values, use environment variables:

```bash
export KEYCLOAK_URL="http://localhost:8080"
export KEYCLOAK_REALM="webdriver-mcp"
export KEYCLOAK_CLIENT_ID="webdriver-mcp-server"
export KEYCLOAK_CLIENT_SECRET="your-secret-here"
export OAUTH_REDIRECT_URI="http://localhost:3000/oauth/callback"
```

Then in your code:

```rust
use std::env;

let oauth_config = OAuthConfig {
    server_url: env::var("KEYCLOAK_URL")
        .unwrap_or_else(|_| "http://localhost:8080".to_string()),
    realm: env::var("KEYCLOAK_REALM")
        .unwrap_or_else(|_| "master".to_string()),
    client_id: env::var("KEYCLOAK_CLIENT_ID")
        .unwrap_or_else(|_| "webdriver-mcp".to_string()),
    client_secret: env::var("KEYCLOAK_CLIENT_SECRET").ok(),
    redirect_uri: env::var("OAUTH_REDIRECT_URI")
        .unwrap_or_else(|_| "http://localhost:3000/oauth/callback".to_string()),
    scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
};
```

## 5. Testing the Setup

1. **Start Keycloak:**
   ```bash
   docker-compose up -d
   ```

2. **Start WebDriver MCP Server:**
   ```bash
   cargo run --features http-server -- --transport http --bind 127.0.0.1:3000
   ```

3. **Test OAuth Flow:**
   - Visit: http://localhost:3000/oauth/authorize
   - You should be redirected to Keycloak login
   - Login with your Keycloak user
   - You should be redirected back with a success message

4. **Test MCP with Token:**
   ```bash
   # Use the token from the OAuth flow
   curl -H "Authorization: Bearer YOUR_TOKEN_HERE" \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
        http://localhost:3000/mcp
   ```

## 6. Production Considerations

### Security Settings
- Use HTTPS in production
- Set proper CORS origins
- Use strong client secrets
- Enable token rotation
- Set appropriate session timeouts

### Keycloak Production Setup
```yaml
# docker-compose.prod.yml
version: '3.8'
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: keycloak
      POSTGRES_USER: keycloak
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  keycloak:
    image: quay.io/keycloak/keycloak:latest
    environment:
      KC_DB: postgres
      KC_DB_URL: jdbc:postgresql://postgres/keycloak
      KC_DB_USERNAME: keycloak
      KC_DB_PASSWORD: password
      KC_HOSTNAME: your-domain.com
      KC_HOSTNAME_STRICT: false
      KC_HTTP_ENABLED: true
      KC_HOSTNAME_STRICT_HTTPS: false
      KEYCLOAK_ADMIN: admin
      KEYCLOAK_ADMIN_PASSWORD: secure-admin-password
    ports:
      - "8080:8080"
    depends_on:
      - postgres
    command: ["start", "--optimized"]

volumes:
  postgres_data:
```

### Environment Variables for Production
```bash
KEYCLOAK_URL="https://auth.your-domain.com"
KEYCLOAK_REALM="production"
KEYCLOAK_CLIENT_ID="webdriver-mcp-prod"
KEYCLOAK_CLIENT_SECRET="super-secure-secret"
OAUTH_REDIRECT_URI="https://mcp.your-domain.com/oauth/callback"
```

## Troubleshooting

### Common Issues

1. **"Invalid redirect URI"**
   - Check that the redirect URI in Keycloak matches exactly
   - Ensure no trailing slashes

2. **"Client not found"**
   - Verify client ID matches
   - Check that you're using the correct realm

3. **"Invalid client credentials"**
   - Double-check the client secret
   - Ensure client authentication is enabled

4. **CORS errors**
   - Add your domain to "Web origins" in Keycloak client settings
   - Check CORS configuration in your MCP server

### Debug Mode
Enable debug logging:
```bash
RUST_LOG=debug cargo run --features http-server -- --transport http
```

This will show OAuth flow details and help identify issues.
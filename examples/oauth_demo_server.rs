use anyhow::Result;
use axum::{
    body::Body,
    extract::{Form, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use uuid::Uuid;
use webdriver_mcp::{WebDriverServer, oauth::AccessToken};

/// Demo OAuth server showing OAuth integration with WebDriver MCP
/// This example runs in demo mode (no Keycloak required)
const BIND_ADDRESS: &str = "127.0.0.1:3000";

/// Simple in-memory session store for demo
#[derive(Clone)]
struct DemoOAuthStore {
    sessions: Arc<RwLock<HashMap<String, DemoSession>>>,
    tokens: Arc<RwLock<HashMap<String, AccessToken>>>,
}

#[derive(Clone, Debug)]
struct DemoSession {
    #[allow(dead_code)]
    state: String,
    #[allow(dead_code)]
    created_at: chrono::DateTime<chrono::Utc>,
}

impl DemoOAuthStore {
    fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn create_session(&self, state: String) {
        let session = DemoSession {
            state: state.clone(),
            created_at: chrono::Utc::now(),
        };
        self.sessions.write().await.insert(state, session);
    }

    async fn validate_session(&self, state: &str) -> bool {
        self.sessions.read().await.contains_key(state)
    }

    async fn remove_session(&self, state: &str) {
        self.sessions.write().await.remove(state);
    }

    async fn store_token(&self, token: AccessToken) {
        self.tokens.write().await.insert(token.token.clone(), token);
    }

    async fn validate_token(&self, token: &str) -> Option<AccessToken> {
        self.tokens.read().await.get(token).cloned()
    }
}

/// Demo authorization page
const AUTHORIZE_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <title>WebDriver MCP - Demo Authorization</title>
    <style>
        body { 
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
            max-width: 600px; 
            margin: 50px auto; 
            padding: 20px; 
            background: #f5f5f5;
        }
        .container { 
            background: white;
            padding: 30px;
            border-radius: 10px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            text-align: center; 
        }
        .logo {
            font-size: 2em;
            margin-bottom: 20px;
            color: #333;
        }
        .description {
            margin: 20px 0;
            color: #666;
            line-height: 1.6;
        }
        .permissions {
            text-align: left;
            background: #f8f9fa;
            padding: 20px;
            border-radius: 5px;
            margin: 20px 0;
        }
        .permissions h3 {
            margin-top: 0;
            color: #333;
        }
        .permissions ul {
            margin: 10px 0;
        }
        .permissions li {
            margin: 5px 0;
            color: #555;
        }
        .btn { 
            padding: 12px 30px; 
            margin: 10px; 
            border: none; 
            border-radius: 5px; 
            cursor: pointer; 
            font-size: 16px;
            text-decoration: none;
            display: inline-block;
        }
        .btn-primary { 
            background-color: #007bff; 
            color: white; 
        }
        .btn-primary:hover {
            background-color: #0056b3;
        }
        .btn-secondary { 
            background-color: #6c757d; 
            color: white; 
        }
        .btn-secondary:hover {
            background-color: #545b62;
        }
        .demo-note {
            background: #fff3cd;
            border: 1px solid #ffeaa7;
            color: #856404;
            padding: 15px;
            border-radius: 5px;
            margin: 20px 0;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">🤖 WebDriver MCP</div>
        <h1>Authorization Request</h1>
        
        <div class="demo-note">
            <strong>Demo Mode:</strong> This is a demonstration OAuth server. 
            In production, you would be redirected to your identity provider (like Keycloak).
        </div>
        
        <div class="description">
            The application is requesting access to your WebDriver MCP server.
        </div>
        
        <div class="permissions">
            <h3>This will allow the application to:</h3>
            <ul>
                <li>🌐 Control web browsers (Chrome, Firefox, Safari)</li>
                <li>📄 Navigate to web pages and interact with elements</li>
                <li>📸 Take screenshots and extract page content</li>
                <li>🔍 Find elements using CSS selectors and XPath</li>
                <li>⌨️ Send keyboard input and mouse clicks</li>
                <li>📊 Execute JavaScript in the browser context</li>
            </ul>
        </div>
        
        <p><strong>Do you want to authorize this request?</strong></p>
        
        <form method="post" action="/oauth/callback">
            <input type="hidden" name="state" value="{STATE}">
            <button type="submit" name="action" value="authorize" class="btn btn-primary">
                ✅ Authorize Access
            </button>
            <button type="submit" name="action" value="deny" class="btn btn-secondary">
                ❌ Deny Access
            </button>
        </form>
        
        <div style="margin-top: 30px; font-size: 0.9em; color: #666;">
            <p>🔒 Your authorization will create a secure access token</p>
            <p>⏰ Token expires in 1 hour for security</p>
        </div>
    </div>
</body>
</html>
"#;

#[derive(Deserialize)]
struct AuthorizeQuery {
    #[allow(dead_code)]
    response_type: Option<String>,
    #[allow(dead_code)]
    client_id: Option<String>,
    #[allow(dead_code)]
    redirect_uri: Option<String>,
    #[allow(dead_code)]
    scope: Option<String>,
    state: Option<String>,
}

/// Handle OAuth authorization request
async fn oauth_authorize(
    Query(query): Query<AuthorizeQuery>,
    State(store): State<Arc<DemoOAuthStore>>,
) -> impl IntoResponse {
    let state = query.state.unwrap_or_else(|| Uuid::new_v4().to_string());
    
    // Store session
    store.create_session(state.clone()).await;
    
    // Return authorization page with state
    let html = AUTHORIZE_HTML.replace("{STATE}", &state);
    Html(html)
}

#[derive(Deserialize)]
struct CallbackForm {
    state: String,
    action: String,
}

/// Handle OAuth callback (authorization response)
async fn oauth_callback(
    State(store): State<Arc<DemoOAuthStore>>,
    Form(form): Form<CallbackForm>,
) -> impl IntoResponse {
    // Validate session
    if !store.validate_session(&form.state).await {
        return Html(r#"
        <h1>❌ Invalid Session</h1>
        <p>The authorization session has expired or is invalid.</p>
        <a href="/oauth/authorize">Try again</a>
        "#).into_response();
    }

    // Clean up session
    store.remove_session(&form.state).await;

    if form.action != "authorize" {
        return Html(r#"
        <h1>🚫 Access Denied</h1>
        <p>Authorization was denied by the user.</p>
        <a href="/oauth/authorize">Try again</a>
        "#).into_response();
    }

    // Create access token
    let token = AccessToken {
        token: format!("demo_token_{}", Uuid::new_v4()),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600), // 1 hour
        refresh_token: Some(format!("demo_refresh_{}", Uuid::new_v4())),
        scope: Some("webdriver".to_string()),
        user_id: "demo_user".to_string(),
    };

    // Store token
    store.store_token(token.clone()).await;

    // Return success page with token
    Html(format!(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>Authorization Successful</title>
        <style>
            body {{ 
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
                max-width: 600px; 
                margin: 50px auto; 
                padding: 20px; 
                background: #f5f5f5;
            }}
            .container {{ 
                background: white;
                padding: 30px;
                border-radius: 10px;
                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
                text-align: center; 
            }}
            .success {{
                color: #28a745;
                font-size: 3em;
                margin-bottom: 20px;
            }}
            .token {{
                background: #f8f9fa;
                padding: 15px;
                border-radius: 5px;
                font-family: monospace;
                word-break: break-all;
                margin: 20px 0;
                border: 1px solid #e9ecef;
            }}
            .example {{
                background: #e7f3ff;
                padding: 20px;
                border-radius: 5px;
                margin: 20px 0;
                text-align: left;
            }}
            .example pre {{
                background: #f1f1f1;
                padding: 15px;
                border-radius: 3px;
                overflow-x: auto;
                margin: 10px 0;
            }}
        </style>
    </head>
    <body>
        <div class="container">
            <div class="success">✅</div>
            <h1>Authorization Successful!</h1>
            
            <p>Your access token has been generated:</p>
            
            <div class="token">
                <strong>Access Token:</strong><br>
                <code>{}</code>
            </div>
            
            <div class="example">
                <h3>🚀 How to use your token:</h3>
                
                <p><strong>1. Test MCP connection:</strong></p>
                <pre>curl -H "Authorization: Bearer {}" \\
     -H "Content-Type: application/json" \\
     -d '{{"jsonrpc":"2.0","id":1,"method":"tools/list"}}' \\
     http://localhost:3000/mcp</pre>
     
                <p><strong>2. In your MCP client configuration:</strong></p>
                <pre>{{
  "mcpServers": {{
    "webdriver": {{
      "command": "npx",
      "args": ["@modelcontextprotocol/server-http"],
      "env": {{
        "MCP_SERVER_URL": "http://localhost:3000/mcp",
        "MCP_SERVER_AUTH": "Bearer {}"
      }}
    }}
  }}
}}</pre>

                <p><strong>3. Token expires in:</strong> 1 hour</p>
            </div>
            
            <p>You can now use this token to access the WebDriver MCP server!</p>
            
            <div style="margin-top: 30px;">
                <a href="/oauth/authorize" style="color: #007bff; text-decoration: none;">
                    🔄 Generate New Token
                </a>
            </div>
        </div>
    </body>
    </html>
    "#, token.token, token.token, token.token)).into_response()
}

/// Token validation middleware
async fn validate_token_middleware(
    State(store): State<Arc<DemoOAuthStore>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = request.headers().get("Authorization");
    let token = match auth_header {
        Some(header) => {
            let header_str = header.to_str().unwrap_or("");
            if let Some(token) = header_str.strip_prefix("Bearer ") {
                token.to_string()
            } else {
                return (StatusCode::UNAUTHORIZED, "Invalid authorization header format. Use: Authorization: Bearer <token>").into_response();
            }
        }
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing Authorization header. Include: Authorization: Bearer <token>").into_response();
        }
    };

    // Validate token
    match store.validate_token(&token).await {
        Some(token_info) => {
            // Add user info to request for downstream use
            request.extensions_mut().insert(token_info);
            next.run(request).await
        }
        None => (StatusCode::UNAUTHORIZED, "Invalid or expired token").into_response(),
    }
}

/// Landing page
async fn index() -> Html<&'static str> {
    Html(r#"
    <!DOCTYPE html>
    <html>
    <head>
        <title>WebDriver MCP Demo OAuth Server</title>
        <style>
            body { 
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; 
                max-width: 800px; 
                margin: 50px auto; 
                padding: 20px; 
                background: #f5f5f5;
            }
            .container { 
                background: white;
                padding: 30px;
                border-radius: 10px;
                box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            }
            .header {
                text-align: center;
                margin-bottom: 30px;
            }
            .logo {
                font-size: 3em;
                margin-bottom: 10px;
            }
            .endpoints {
                background: #f8f9fa;
                padding: 20px;
                border-radius: 5px;
                margin: 20px 0;
            }
            .endpoint {
                margin: 10px 0;
                padding: 10px;
                background: white;
                border-radius: 3px;
                border-left: 4px solid #007bff;
            }
            .btn {
                display: inline-block;
                padding: 10px 20px;
                background: #007bff;
                color: white;
                text-decoration: none;
                border-radius: 5px;
                margin: 10px 0;
            }
            .btn:hover {
                background: #0056b3;
            }
        </style>
    </head>
    <body>
        <div class="container">
            <div class="header">
                <div class="logo">🤖</div>
                <h1>WebDriver MCP Demo OAuth Server</h1>
                <p>OAuth-protected WebDriver automation for Claude and other MCP clients</p>
            </div>
            
            <h2>🚀 Quick Start</h2>
            <p>This server provides OAuth-protected access to WebDriver automation tools.</p>
            
            <a href="/oauth/authorize" class="btn">🔐 Get Access Token</a>
            
            <div class="endpoints">
                <h3>📡 Available Endpoints:</h3>
                <div class="endpoint">
                    <strong>Authorization:</strong> <code>/oauth/authorize</code><br>
                    Start the OAuth flow to get an access token
                </div>
                <div class="endpoint">
                    <strong>MCP Server:</strong> <code>/mcp</code> (Protected)<br>
                    WebDriver MCP server endpoint - requires Bearer token
                </div>
            </div>
            
            <h3>💡 Example Usage:</h3>
            <ol>
                <li>Click "Get Access Token" above</li>
                <li>Authorize the application</li>
                <li>Copy your access token</li>
                <li>Use the token to access <code>/mcp</code> endpoints</li>
            </ol>
            
            <h3>🔧 Test Commands:</h3>
            <pre style="background: #f1f1f1; padding: 15px; border-radius: 5px; overflow-x: auto;">
# List available tools
curl -H "Authorization: Bearer YOUR_TOKEN" \\
     -H "Content-Type: application/json" \\
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \\
     http://localhost:3000/mcp

# Open a webpage
curl -H "Authorization: Bearer YOUR_TOKEN" \\
     -H "Content-Type: application/json" \\
     -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"webdriver_navigate","arguments":{"url":"https://example.com"}}}' \\
     http://localhost:3000/mcp
            </pre>
        </div>
    </body>
    </html>
    "#)
}

/// Health check endpoint
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "webdriver-mcp-oauth-demo",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into())
        )
        .init();

    println!("🚀 Starting WebDriver MCP Demo OAuth Server");
    println!("📍 Server will be available at: http://{BIND_ADDRESS}");
    println!("🔐 Authorization URL: http://{BIND_ADDRESS}/oauth/authorize");
    println!("🤖 MCP Endpoint: http://{BIND_ADDRESS}/mcp (requires token)");

    // Create WebDriver MCP server
    let webdriver_server = WebDriverServer::new()
        .map_err(|e| anyhow::anyhow!("Failed to create WebDriver server: {}", e))?;

    // Create demo OAuth store
    let oauth_store = Arc::new(DemoOAuthStore::new());

    // Create MCP service - this handles the actual WebDriver functionality
    use rmcp::transport::streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager};
    let mcp_service = StreamableHttpService::new(
        move || Ok(webdriver_server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create protected MCP routes (require OAuth token)
    let protected_mcp = Router::new()
        .nest_service("/mcp", mcp_service)
        .layer(middleware::from_fn_with_state(
            oauth_store.clone(),
            validate_token_middleware,
        ));

    // Create OAuth routes
    let oauth_routes = Router::new()
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/callback", post(oauth_callback))
        .with_state(oauth_store);

    // Combine all routes
    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .merge(oauth_routes)
        .merge(protected_mcp)
        .layer(CorsLayer::permissive());

    // Start server
    let addr = BIND_ADDRESS.parse::<SocketAddr>()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    println!("\n✅ Server started successfully!");
    println!("👉 Visit http://{BIND_ADDRESS} to get started");
    println!("⏹️  Press Ctrl+C to stop\n");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            println!("\n🛑 Shutting down server...");
        })
        .await?;

    println!("👋 Server stopped");
    Ok(())
}
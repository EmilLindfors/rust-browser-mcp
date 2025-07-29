use std::collections::HashMap;
use std::sync::Arc;

use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Query, State},
    http::{Request, StatusCode, HeaderMap, header::{SET_COOKIE, COOKIE}},
    middleware::Next,
    response::{Html, IntoResponse, Json, Redirect, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use oauth2::TokenResponse;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::auth::keycloak::KeycloakClient;

pub use crate::auth::keycloak::KeycloakConfig as OAuthConfig;

pub use crate::auth::keycloak::KeycloakAuthState as OAuthSession;

/// Access token information
#[derive(Clone, Debug)]
pub struct AccessToken {
    pub token: String,
    pub token_type: String,
    pub expires_in: Option<u64>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
    pub user_id: String,
}

/// OAuth store for managing sessions and tokens
#[derive(Clone)]
pub struct OAuthStore {
    sessions: Arc<RwLock<HashMap<String, OAuthSession>>>,
    tokens: Arc<RwLock<HashMap<String, AccessToken>>>,
    keycloak_client: Option<Arc<KeycloakClient>>,
}

impl OAuthStore {
    pub fn new(config: OAuthConfig) -> Self {
        let keycloak_client = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                KeycloakClient::new(config).await.map(Arc::new).ok()
            })
        });
        
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            keycloak_client,
        }
    }

    pub async fn create_session(&self, state: String, auth_state: OAuthSession) {
        self.sessions.write().await.insert(state, auth_state);
    }

    pub async fn get_session(&self, state: &str) -> Option<OAuthSession> {
        self.sessions.read().await.get(state).cloned()
    }

    pub async fn remove_session(&self, state: &str) -> Option<OAuthSession> {
        self.sessions.write().await.remove(state)
    }

    pub async fn store_token(&self, token_id: String, token: AccessToken) {
        self.tokens.write().await.insert(token_id, token);
    }

    pub async fn validate_token(&self, token: &str) -> Option<AccessToken> {
        // First check local cache
        if let Some(cached_token) = self.tokens.read().await.get(token).cloned() {
            return Some(cached_token);
        }

        // If not in cache and we have Keycloak client, validate remotely
        if let Some(keycloak) = &self.keycloak_client {
            if let Ok(introspection) = keycloak.introspect_token(token).await {
                if introspection.is_active() && !introspection.is_expired() {
                    let access_token = AccessToken {
                        token: token.to_string(),
                        token_type: "Bearer".to_string(),
                        expires_in: introspection.exp,
                        refresh_token: None,
                        scope: introspection.scope,
                        user_id: introspection.sub.unwrap_or_else(|| "unknown".to_string()),
                    };
                    
                    // Cache the validated token
                    self.tokens.write().await.insert(token.to_string(), access_token.clone());
                    return Some(access_token);
                }
            }
        }

        None
    }

    pub fn get_keycloak_client(&self) -> Option<&Arc<KeycloakClient>> {
        self.keycloak_client.as_ref()
    }
}

/// Authorization template for rendering OAuth consent page
#[derive(Template)]
#[template(source = r#"
<!DOCTYPE html>
<html>
<head>
    <title>WebDriver MCP - Authorization</title>
    <style>
        body { font-family: Arial, sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }
        .container { text-align: center; }
        .btn { padding: 10px 20px; margin: 10px; border: none; border-radius: 5px; cursor: pointer; }
        .btn-primary { background-color: #007bff; color: white; }
        .btn-secondary { background-color: #6c757d; color: white; }
    </style>
</head>
<body>
    <div class="container">
        <h1>WebDriver MCP Authorization</h1>
        <p>The application is requesting access to your WebDriver MCP server.</p>
        <p>This will allow the application to:</p>
        <ul style="text-align: left;">
            <li>Access browser automation tools</li>
            <li>Execute WebDriver commands</li>
            <li>Manage browser sessions</li>
        </ul>
        <form method="post" action="/oauth/callback">
            <input type="hidden" name="state" value="{{ state }}">
            <button type="submit" name="action" value="authorize" class="btn btn-primary">Authorize</button>
            <button type="submit" name="action" value="deny" class="btn btn-secondary">Deny</button>
        </form>
    </div>
</body>
</html>
"#, ext = "html")]
struct AuthorizeTemplate {
    state: String,
}

/// OAuth authorization handler
pub async fn oauth_authorize(
    Query(params): Query<HashMap<String, String>>,
    State(store): State<Arc<OAuthStore>>,
) -> impl IntoResponse {
    // If we have a Keycloak client, redirect to Keycloak
    if let Some(keycloak) = store.get_keycloak_client() {
        match keycloak.get_authorization_url() {
            Ok((auth_url, auth_state)) => {
                // Store the auth state
                store.create_session(auth_state.csrf_token.clone(), auth_state).await;
                
                // Redirect to Keycloak
                return Redirect::to(auth_url.as_str()).into_response();
            }
            Err(e) => {
                tracing::error!("Failed to get authorization URL: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "OAuth configuration error").into_response();
            }
        }
    }
    
    // Fallback to simple authorization page if no Keycloak client
    let state = params.get("state").cloned().unwrap_or_else(|| Uuid::new_v4().to_string());
    let template = AuthorizeTemplate { state: state.clone() };
    
    // Create a simple demo session
    let demo_session = OAuthSession {
        csrf_token: state.clone(),
        pkce_verifier: Some("demo_verifier".to_string()),
        nonce: Some("demo_nonce".to_string()),
        created_at: Utc::now(),
    };
    
    store.create_session(state, demo_session).await;
    Html(template.render().unwrap()).into_response()
}

/// OAuth callback handler for both Keycloak and local flows
#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackForm {
    state: String,
    action: String,
}

/// OAuth 2.0 Authorization Server Metadata (RFC 8414)
#[derive(Serialize)]
pub struct AuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub response_types_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
}

/// OAuth 2.0 Protected Resource Metadata (RFC 9728)
#[derive(Serialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub authorization_servers: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub bearer_methods_supported: Vec<String>,
}

pub async fn oauth_callback_get(
    Query(query): Query<CallbackQuery>,
    State(store): State<Arc<OAuthStore>>,
) -> impl IntoResponse {
    // Handle OAuth error responses
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_else(|| "Unknown error".to_string());
        tracing::warn!("OAuth error: {} - {}", error, description);
        return (StatusCode::BAD_REQUEST, format!("OAuth error: {description}")).into_response();
    }

    // Must have code and state for successful authorization
    let code = match query.code {
        Some(code) => code,
        None => return (StatusCode::BAD_REQUEST, "Missing authorization code").into_response(),
    };

    let state = match query.state {
        Some(state) => state,
        None => return (StatusCode::BAD_REQUEST, "Missing state parameter").into_response(),
    };

    // Validate session
    let session = match store.get_session(&state).await {
        Some(session) => session,
        None => return (StatusCode::BAD_REQUEST, "Invalid or expired session").into_response(),
    };

    // Clean up session
    store.remove_session(&state).await;

    // If we have Keycloak client, exchange code with Keycloak
    if let Some(keycloak) = store.get_keycloak_client() {
        match keycloak.exchange_code(&code, &session).await {
            Ok(token_response) => {
                let access_token_str = token_response.access_token().secret();
                let _token_id = Uuid::new_v4().to_string();
                
                // Get user info from Keycloak
                let user_id = match keycloak.get_user_info(access_token_str).await {
                    Ok(user_info) => {
                        user_info.get("sub")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string()
                    }
                    Err(_) => "unknown".to_string(),
                };

                let access_token = AccessToken {
                    token: access_token_str.to_string(),
                    token_type: "Bearer".to_string(),
                    expires_in: token_response.expires_in().map(|d| d.as_secs()),
                    refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
                    scope: token_response.scopes().map(|scopes| {
                        scopes.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ")
                    }),
                    user_id,
                };

                store.store_token(access_token.token.clone(), access_token.clone()).await;

                // Set HTTP-only cookie with the access token
                let mut headers = HeaderMap::new();
                let cookie_value = format!(
                    "auth_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
                    access_token.token,
                    access_token.expires_in.unwrap_or(3600)
                );
                headers.insert(SET_COOKIE, cookie_value.parse().unwrap());

                return (headers, Html(
                    "<h1>Authorization Successful</h1><p>Authentication complete! You can now access protected resources.</p><script>window.location.href='/';</script>".to_string()
                )).into_response();
            }
            Err(e) => {
                tracing::error!("Failed to exchange code with Keycloak: {}", e);
                return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to complete authorization").into_response();
            }
        }
    }

    // Fallback for demo mode
    let token_id = Uuid::new_v4().to_string();
    let access_token = AccessToken {
        token: token_id.clone(),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600),
        refresh_token: Some(Uuid::new_v4().to_string()),
        scope: Some("webdriver".to_string()),
        user_id: "demo_user".to_string(),
    };

    store.store_token(token_id.clone(), access_token.clone()).await;

    // Set HTTP-only cookie with the access token
    let mut headers = HeaderMap::new();
    let cookie_value = format!(
        "auth_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
        access_token.token,
        access_token.expires_in.unwrap_or(3600)
    );
    headers.insert(SET_COOKIE, cookie_value.parse().unwrap());

    (headers, Html(
        "<h1>Authorization Successful (Demo Mode)</h1><p>Authentication complete! You can now access protected resources.</p><script>window.location.href='/';</script>".to_string()
    )).into_response()
}

pub async fn oauth_callback_post(
    State(store): State<Arc<OAuthStore>>,
    Form(form): Form<CallbackForm>,
) -> impl IntoResponse {
    if form.action != "authorize" {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    // This is for the simple demo flow
    let _session = match store.get_session(&form.state).await {
        Some(session) => session,
        None => return (StatusCode::BAD_REQUEST, "Invalid session").into_response(),
    };

    store.remove_session(&form.state).await;

    let token_id = Uuid::new_v4().to_string();
    let access_token = AccessToken {
        token: token_id.clone(),
        token_type: "Bearer".to_string(),
        expires_in: Some(3600),
        refresh_token: Some(Uuid::new_v4().to_string()),
        scope: Some("webdriver".to_string()),
        user_id: "demo_user".to_string(),
    };

    store.store_token(token_id.clone(), access_token.clone()).await;

    // Set HTTP-only cookie with the access token
    let mut headers = HeaderMap::new();
    let cookie_value = format!(
        "auth_token={}; HttpOnly; SameSite=Strict; Path=/; Max-Age={}",
        access_token.token,
        access_token.expires_in.unwrap_or(3600)
    );
    headers.insert(SET_COOKIE, cookie_value.parse().unwrap());

    (headers, Html(
        "<h1>Authorization Successful (Demo Mode)</h1><p>Authentication complete! You can now access protected resources.</p><script>window.location.href='/';</script>".to_string()
    )).into_response()
}

/// Token validation middleware with session ID injection
pub async fn validate_token_middleware(
    State(store): State<Arc<OAuthStore>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // Try to extract token from Authorization header first
    let token = if let Some(auth_header) = request.headers().get("Authorization") {
        let header_str = auth_header.to_str().unwrap_or("");
        header_str.strip_prefix("Bearer ").map(|token| token.to_string())
    } else {
        None
    };

    // If no Authorization header, try to extract from cookies
    let token = token.or_else(|| {
        request.headers().get(COOKIE)
            .and_then(|cookie_header| cookie_header.to_str().ok())
            .and_then(|cookies| {
                // Parse cookies to find auth_token
                for cookie in cookies.split(';') {
                    let cookie = cookie.trim();
                    if let Some(token) = cookie.strip_prefix("auth_token=") {
                        return Some(token.to_string());
                    }
                }
                None
            })
    });

    // Check if we have a token from either source
    let token = match token {
        Some(token) => token,
        None => {
            return (StatusCode::UNAUTHORIZED, "Missing authentication token").into_response();
        }
    };

    // Validate token
    match store.validate_token(&token).await {
        Some(token_info) => {
            // Add user info to request extensions for downstream use
            request.extensions_mut().insert(token_info.clone());
            
            // Inject Mcp-Session-Id header if not present (for StreamableHttpService compatibility)
            if !request.headers().contains_key("mcp-session-id") {
                // Use a deterministic session ID based on user_id to maintain consistency
                let session_id = format!("oauth-session-{}", token_info.user_id);
                request.headers_mut().insert(
                    "mcp-session-id",
                    session_id.parse().unwrap()
                );
                tracing::debug!("Injected session ID for OAuth user: {}", session_id);
            }
            
            next.run(request).await
        }
        None => (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    }
}

/// Helper function to extract base URL from request
fn get_base_url_from_request(request: &Request<Body>) -> String {
    let scheme = "http"; // TODO: Detect HTTPS
    let host = request
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost:8080");
    format!("{scheme}://{host}")
}

/// OAuth 2.0 Authorization Server Metadata endpoint (RFC 8414)
pub async fn oauth_server_metadata(
    State(_store): State<Arc<OAuthStore>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let base_url = get_base_url_from_request(&request);
    
    let metadata = AuthorizationServerMetadata {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{base_url}/oauth/authorize"),
        token_endpoint: format!("{base_url}/oauth/callback"), // Using callback as token endpoint for demo
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        code_challenge_methods_supported: vec!["S256".to_string()],
        scopes_supported: vec!["webdriver".to_string(), "openid".to_string()],
    };
    
    Json(metadata)
}

/// OAuth 2.0 Protected Resource Metadata endpoint (RFC 9728)
pub async fn protected_resource_metadata(
    State(_store): State<Arc<OAuthStore>>,
    request: Request<Body>,
) -> impl IntoResponse {
    let base_url = get_base_url_from_request(&request);
    
    let metadata = ProtectedResourceMetadata {
        resource: format!("{base_url}/mcp"),
        authorization_servers: vec![base_url.clone()],
        scopes_supported: vec!["webdriver".to_string()],
        bearer_methods_supported: vec!["header".to_string()],
    };
    
    Json(metadata)
}

/// Create OAuth router with all endpoints
pub fn create_oauth_router(store: Arc<OAuthStore>) -> Router {
    Router::new()
        .route("/oauth/authorize", get(oauth_authorize))
        .route("/oauth/callback", get(oauth_callback_get).post(oauth_callback_post))
        .route("/.well-known/oauth-authorization-server", get(oauth_server_metadata))
        .route("/.well-known/oauth-protected-resource", get(protected_resource_metadata))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(store)
}

// Keycloak client functionality is now in keycloak.rs module
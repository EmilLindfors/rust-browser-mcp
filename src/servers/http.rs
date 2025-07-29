use anyhow::Result;
use tokio::signal;
use rust_browser_mcp::WebDriverServer;

use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
    session::SessionManager,
};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc as StdArc;
use tower_http::cors::CorsLayer;
use rust_browser_mcp::auth::oauth::{OAuthConfig, OAuthStore, create_oauth_router};
use axum::{middleware, extract::State, http::{Request, StatusCode}, response::Response, body::Body};
use axum::middleware::Next;
use std::sync::Arc;

pub async fn run_http_server(server: WebDriverServer, bind_addr: &str, no_auth: bool) -> Result<()> {
    // Convert server to HTTP mode
    let config = server.get_client_manager().get_config().clone();
    let mut server = rust_browser_mcp::WebDriverServer::with_config_and_mode(config, rust_browser_mcp::tools::ServerMode::Http)?;
    tracing::info!(
        "WebDriver MCP Server listening on HTTP at {} (Ctrl+C to stop)",
        bind_addr
    );

    // HTTP mode: Start drivers proactively for better performance
    server.ensure_drivers_started().await?;

    // Create a clone of the server for cleanup before moving it into the closure
    let server_for_cleanup = server.clone();

    // Create MCP service
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    let router = if no_auth {
        // Create unprotected MCP routes (no OAuth)
        axum::Router::new()
            .fallback_service(service)
            .layer(CorsLayer::permissive())
    } else {
        // Create OAuth store with default configuration
        let oauth_config = OAuthConfig::default();
        let oauth_store = Arc::new(OAuthStore::new(oauth_config));

        // Create session manager for OAuth users
        let session_manager = Arc::new(
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager::default()
        );

        // Create OAuth-to-session mapping
        let oauth_sessions: Arc<RwLock<HashMap<String, StdArc<str>>>> = Arc::new(RwLock::new(HashMap::new()));

        // Create protected MCP routes with OAuth middleware and session management
        let protected_service = tower::ServiceBuilder::new()
            .layer(middleware::from_fn_with_state(
                (oauth_store.clone(), session_manager.clone(), oauth_sessions.clone()),
                oauth_session_middleware,
            ))
            .service(service);

        // Create OAuth router
        let oauth_router = create_oauth_router(oauth_store);

        // Combine all routes
        axum::Router::new()
            .merge(oauth_router)
            .fallback_service(protected_service)
            .layer(CorsLayer::permissive())
    };

    let tcp_listener = tokio::net::TcpListener::bind(bind_addr).await?;

    if no_auth {
        tracing::info!("MCP endpoint (no auth): http://{}/", bind_addr);
    } else {
        tracing::info!("OAuth endpoints available at:");
        tracing::info!("  Authorization: http://{}/oauth/authorize", bind_addr);
        tracing::info!("  Callback: http://{}/oauth/callback", bind_addr);
        tracing::info!("Protected MCP endpoint: http://{}/", bind_addr);
    }

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async move {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            tracing::info!("Received shutdown signal (Ctrl+C), initiating graceful shutdown...");
            
            // Cleanup WebDriver processes before shutdown with timeout
            let cleanup_timeout = std::time::Duration::from_secs(8);
            tracing::info!("Starting WebDriver cleanup with {:?} timeout...", cleanup_timeout);
            
            match tokio::time::timeout(cleanup_timeout, server_for_cleanup.cleanup()).await {
                Ok(Ok(())) => tracing::info!("WebDriver cleanup completed successfully"),
                Ok(Err(e)) => tracing::warn!("Error during WebDriver cleanup: {}", e),
                Err(_) => {
                    tracing::warn!("WebDriver cleanup timed out after {:?}, forcing server shutdown", cleanup_timeout);
                    tracing::warn!("Some WebDriver processes may still be running");
                }
            }
            
            tracing::info!("Graceful shutdown sequence completed");
        })
        .await?;

    tracing::info!("WebDriver MCP HTTP Server stopped");
    Ok(())
}

/// OAuth session middleware that ensures sessions exist for OAuth-authenticated users
async fn oauth_session_middleware(
    State((oauth_store, session_manager, oauth_sessions)): State<(Arc<OAuthStore>, Arc<LocalSessionManager>, Arc<RwLock<HashMap<String, StdArc<str>>>>)>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    // First validate the OAuth token using the existing middleware logic
    let token = if let Some(auth_header) = request.headers().get("Authorization") {
        let header_str = auth_header.to_str().unwrap_or("");
        header_str.strip_prefix("Bearer ").map(|token| token.to_string())
    } else {
        None
    };

    let token = token.or_else(|| {
        request.headers().get("cookie")
            .and_then(|cookie_header| cookie_header.to_str().ok())
            .and_then(|cookies| {
                for cookie in cookies.split(';') {
                    let cookie = cookie.trim();
                    if let Some(token) = cookie.strip_prefix("auth_token=") {
                        return Some(token.to_string());
                    }
                }
                None
            })
    });

    let token = match token {
        Some(token) => token,
        None => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Missing authentication token".into())
                .unwrap();
        }
    };

    // Validate token
    let token_info = match oauth_store.validate_token(&token).await {
        Some(token_info) => token_info,
        None => {
            return Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .body("Invalid token".into())
                .unwrap();
        }
    };

    // Get or create session for this OAuth user
    let user_key = token_info.user_id.clone();
    let session_id = {
        let oauth_sessions_read = oauth_sessions.read().await;
        oauth_sessions_read.get(&user_key).cloned()
    };

    let session_id = match session_id {
        Some(existing_session_id) => {
            // Verify session still exists
            if let Ok(true) = session_manager.has_session(&existing_session_id).await {
                existing_session_id
            } else {
                // Session was dropped, remove from mapping and create new one
                oauth_sessions.write().await.remove(&user_key);
                match session_manager.create_session().await {
                    Ok((new_session_id, _transport)) => {
                        oauth_sessions.write().await.insert(user_key.clone(), new_session_id.clone());
                        tracing::debug!("Created new session {} for OAuth user {}", new_session_id, user_key);
                        new_session_id
                    },
                    Err(e) => {
                        tracing::error!("Failed to create session for OAuth user {}: {}", user_key, e);
                        return Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .body("Failed to create session".into())
                            .unwrap();
                    }
                }
            }
        },
        None => {
            // No existing session, create one
            match session_manager.create_session().await {
                Ok((new_session_id, _transport)) => {
                    oauth_sessions.write().await.insert(user_key.clone(), new_session_id.clone());
                    tracing::debug!("Created session {} for OAuth user {}", new_session_id, user_key);
                    new_session_id
                },
                Err(e) => {
                    tracing::error!("Failed to create session for OAuth user {}: {}", user_key, e);
                    return Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body("Failed to create session".into())
                        .unwrap();
                }
            }
        }
    };

    // Add session ID header for StreamableHttpService
    request.headers_mut().insert(
        "mcp-session-id",
        session_id.as_ref().parse().unwrap()
    );

    // Add user info to request extensions
    request.extensions_mut().insert(token_info);
    
    next.run(request).await
}
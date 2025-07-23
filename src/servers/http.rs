use anyhow::Result;
use tokio::signal;
use rust_browser_mcp::WebDriverServer;

use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};
use tower_http::cors::CorsLayer;
use rust_browser_mcp::oauth::{OAuthConfig, OAuthStore, create_oauth_router, validate_token_middleware};
use axum::middleware;
use std::sync::Arc;

pub async fn run_http_server(server: WebDriverServer, bind_addr: &str) -> Result<()> {
    tracing::info!(
        "WebDriver MCP Server listening on HTTP at {} (Ctrl+C to stop)",
        bind_addr
    );

    // Create OAuth store with default configuration
    let oauth_config = OAuthConfig::default();
    let oauth_store = Arc::new(OAuthStore::new(oauth_config));

    // Create MCP service
    let service = StreamableHttpService::new(
        move || Ok(server.clone()),
        LocalSessionManager::default().into(),
        Default::default(),
    );

    // Create protected MCP routes with OAuth middleware
    let protected_mcp = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            oauth_store.clone(),
            validate_token_middleware,
        ));

    // Create OAuth router
    let oauth_router = create_oauth_router(oauth_store);

    // Combine all routes
    let router = axum::Router::new()
        .merge(oauth_router)
        .merge(protected_mcp)
        .layer(CorsLayer::permissive());

    let tcp_listener = tokio::net::TcpListener::bind(bind_addr).await?;

    tracing::info!("OAuth endpoints available at:");
    tracing::info!("  Authorization: http://{}/oauth/authorize", bind_addr);
    tracing::info!("  Callback: http://{}/oauth/callback", bind_addr);
    tracing::info!("Protected MCP endpoint: http://{}/mcp", bind_addr);

    axum::serve(tcp_listener, router)
        .with_graceful_shutdown(async {
            signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
            tracing::info!("Received shutdown signal, stopping HTTP server...");
        })
        .await?;

    tracing::info!("WebDriver MCP HTTP Server stopped");
    Ok(())
}
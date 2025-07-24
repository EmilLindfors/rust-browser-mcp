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

pub async fn run_http_server(server: WebDriverServer, bind_addr: &str, no_auth: bool) -> Result<()> {
    tracing::info!(
        "WebDriver MCP Server listening on HTTP at {} (Ctrl+C to stop)",
        bind_addr
    );

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

        // Create protected MCP routes with OAuth middleware
        let protected_service = tower::ServiceBuilder::new()
            .layer(middleware::from_fn_with_state(
                oauth_store.clone(),
                validate_token_middleware,
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
use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tokio::signal;
use rust_browser_mcp::WebDriverServer;

pub async fn run_stdio_server(server: WebDriverServer) -> Result<()> {
    tracing::info!("WebDriver MCP Server listening on stdio (Ctrl+C to stop)");

    // Set up graceful shutdown
    tokio::select! {
        result = server.serve(stdio()) => {
            match result {
                Ok(service) => {
                    if let Err(e) = service.waiting().await {
                        tracing::error!("Service error: {}", e);
                        return Err(e.into());
                    }
                }
                Err(e) => {
                    tracing::error!("Server serve error: {}", e);
                    return Err(e.into());
                }
            }
        }
        _ = signal::ctrl_c() => {
            tracing::info!("Received shutdown signal, stopping server...");
        }
    }

    tracing::info!("WebDriver MCP Server stopped");
    Ok(())
}
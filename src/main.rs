use anyhow::Result;
use clap::{Parser, ValueEnum};
use rust_browser_mcp::WebDriverServer;

mod servers;
use servers::{run_http_server, run_stdio_server};

#[derive(Parser)]
#[command(name = "rust-browser-mcp")]
#[command(about = "Rust Browser MCP Server - Browser automation for Claude")]
#[command(version)]
struct Cli {
    /// Server transport mode
    #[arg(short, long, default_value = "stdio")]
    transport: TransportMode,

    /// HTTP server bind address (only used with --transport=http)
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: String,
}

#[derive(Clone, ValueEnum)]
enum TransportMode {
    /// Standard I/O transport (default)
    Stdio,
    /// HTTP streaming server
    Http,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let server = WebDriverServer::new().inspect_err(|e| {
        tracing::error!("Failed to create WebDriver server: {}", e);
    })?;

    match cli.transport {
        TransportMode::Stdio => {
            tracing::info!(
                "Starting WebDriver MCP Server on stdio with auto-detection and auto-start"
            );
            run_stdio_server(server).await
        }
        TransportMode::Http => {
            tracing::info!("Starting WebDriver MCP Server on HTTP at {}", cli.bind);
            run_http_server(server, &cli.bind).await
        }
    }
}

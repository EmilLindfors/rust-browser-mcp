use anyhow::Result;
use clap::{Parser, ValueEnum};
use rust_browser_mcp::{Config, WebDriverServer};

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

    /// Disable OAuth authentication for HTTP server
    #[arg(long)]
    no_auth: bool,

    /// Browser driver to use
    #[arg(short, long, default_value = "chrome")]
    browser: BrowserType,

    /// Enable Chrome performance memory APIs for enhanced memory monitoring
    #[arg(long)]
    enable_performance_memory: bool,
}

#[derive(Clone, ValueEnum)]
enum TransportMode {
    /// Standard I/O transport (default)
    Stdio,
    /// HTTP streaming server
    Http,
}

#[derive(Clone, ValueEnum)]
enum BrowserType {
    /// Google Chrome browser
    Chrome,
    /// Mozilla Firefox browser
    Firefox,
    /// Microsoft Edge browser
    Edge,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let default_log_level = if cfg!(debug_assertions) {
        "info"
    } else {
        "error"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_log_level)),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Create config with preferred browser and CLI options
    let mut config = Config::from_env();
    let preferred_browser = match cli.browser {
        BrowserType::Chrome => "chrome".to_string(),
        BrowserType::Firefox => "firefox".to_string(),
        BrowserType::Edge => "edge".to_string(),
    };

    config.preferred_driver = Some(preferred_browser.clone());
    // When a specific browser is chosen via CLI, only start that browser instead of all concurrent drivers
    config.concurrent_drivers = vec![preferred_browser];

    // Override performance memory setting from CLI if provided
    if cli.enable_performance_memory {
        config.enable_performance_memory = true;
    }

    let server = WebDriverServer::with_config(config).inspect_err(|e| {
        tracing::error!("Failed to create WebDriver server: {}", e);
    })?;

    match cli.transport {
        TransportMode::Stdio => {
            tracing::info!(
                "Starting WebDriver MCP Server on stdio with auto-detection and auto-start"
            );
            // Temporarily skip buffered stdio to test regular stdio
            tracing::info!("Using regular stdio transport for debugging");
            run_stdio_server(server).await
        }
        TransportMode::Http => {
            tracing::info!("Starting WebDriver MCP Server on HTTP at {}", cli.bind);
            run_http_server(server, &cli.bind, cli.no_auth).await
        }
    }
}

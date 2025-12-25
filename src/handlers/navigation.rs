//! Navigation handlers for browser control
//!
//! Handles URL navigation operations:
//! - Navigate to URLs
//! - Browser history (back, forward)
//! - Page refresh
//! - Current URL retrieval
//! - Page load status

use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    tools::{error_response, success_response},
};
use super::extract_session_id;

/// JavaScript to set up console log monitoring in the browser
const CONSOLE_MONITOR_SCRIPT: &str = r#"
    try {
        if (!window.__mcpConsoleLogs) {
            window.__mcpConsoleLogs = [];

            const originalConsole = {
                log: console.log,
                error: console.error,
                warn: console.warn,
                info: console.info,
                debug: console.debug
            };

            ['log', 'error', 'warn', 'info', 'debug'].forEach(level => {
                console[level] = function(...args) {
                    originalConsole[level].apply(console, args);
                    window.__mcpConsoleLogs.push({
                        level: level,
                        message: args.map(arg => {
                            if (typeof arg === 'object') {
                                try {
                                    return JSON.stringify(arg, null, 2);
                                } catch (e) {
                                    return String(arg);
                                }
                            }
                            return String(arg);
                        }).join(' '),
                        timestamp: Date.now(),
                        url: window.location.href
                    });
                };
            });

            window.onerror = function(message, source, lineno, colno, error) {
                window.__mcpConsoleLogs.push({
                    level: 'error',
                    message: message + ' at ' + source + ':' + lineno + ':' + colno,
                    timestamp: Date.now(),
                    url: window.location.href,
                    stack: error ? error.stack : null
                });
                return false;
            };

            window.addEventListener('unhandledrejection', function(event) {
                window.__mcpConsoleLogs.push({
                    level: 'error',
                    message: 'Unhandled Promise Rejection: ' + event.reason,
                    timestamp: Date.now(),
                    url: window.location.href
                });
            });
        }
        return true;
    } catch (e) {
        return false;
    }
"#;

/// Set up console log monitoring for a browser session
pub async fn setup_console_monitoring(client: &fantoccini::Client) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    client.execute(CONSOLE_MONITOR_SCRIPT, vec![]).await?;
    Ok(())
}

/// Navigate to a URL
pub async fn handle_navigate(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let url = arguments
        .as_ref()
        .and_then(|args| args.get("url"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("url parameter required", None))?;

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.goto(url).await {
            Ok(_) => {
                // Set up console monitoring immediately after navigation
                if let Err(e) = setup_console_monitoring(&client).await {
                    eprintln!("Warning: Failed to setup console monitoring: {}", e);
                }
                Ok(success_response(format!(
                    "Successfully navigated to {url} (session: {session})"
                )))
            },
            Err(e) => Ok(error_response(format!("Failed to navigate: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get the current page URL
pub async fn handle_get_current_url(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.current_url().await {
            Ok(url) => Ok(success_response(format!(
                "Current URL: {url} (session: {session})"
            ))),
            Err(e) => Ok(error_response(format!("Failed to get current URL: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Navigate back in browser history
pub async fn handle_back(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.back().await {
            Ok(_) => Ok(success_response(format!(
                "Successfully navigated back (session: {session})"
            ))),
            Err(e) => Ok(error_response(format!("Failed to navigate back: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Navigate forward in browser history
pub async fn handle_forward(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.forward().await {
            Ok(_) => Ok(success_response(format!(
                "Successfully navigated forward (session: {session})"
            ))),
            Err(e) => Ok(error_response(format!("Failed to navigate forward: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Refresh the current page
pub async fn handle_refresh(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.refresh().await {
            Ok(_) => {
                // Set up console monitoring immediately after refresh
                if let Err(e) = setup_console_monitoring(&client).await {
                    eprintln!("Warning: Failed to setup console monitoring: {}", e);
                }
                Ok(success_response(format!(
                    "Successfully refreshed page (session: {session})"
                )))
            },
            Err(e) => Ok(error_response(format!("Failed to refresh page: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get page load status (document.readyState)
pub async fn handle_get_page_load_status(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client.execute("return document.readyState;", vec![]).await {
                Ok(result) => {
                    let status = result.as_str().unwrap_or("unknown");
                    Ok(success_response(format!(
                        "Page load status: {status} (session: {session})"
                    )))
                }
                Err(e) => Ok(error_response(format!("Failed to get page load status: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

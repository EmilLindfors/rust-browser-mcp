//! Page content handlers
//!
//! Handles page-level operations:
//! - Getting page title and source
//! - Getting element text
//! - Taking screenshots
//! - Executing JavaScript
//! - Resizing browser window

use base64::{Engine as _, engine::general_purpose};
use fantoccini::Locator;
use rmcp::{ErrorData as McpError, model::{CallToolResult, Content}};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    tools::{error_response, success_response},
};
use super::extract_session_id;

/// Get the current page title
pub async fn handle_get_title(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.title().await {
            Ok(title) => Ok(success_response(format!(
                "Page title: {title} (session: {session})"
            ))),
            Err(e) => Ok(error_response(format!("Failed to get title: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get text content of an element
pub async fn handle_get_text(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.find(Locator::Css(selector)).await {
            Ok(element) => match element.text().await {
                Ok(text) => Ok(success_response(format!(
                    "Element text: {text} (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to get element text: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to find element {selector}: {e}"
            ))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Execute JavaScript in the page context
pub async fn handle_execute_script(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let script = arguments
        .as_ref()
        .and_then(|args| args.get("script"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("script parameter required", None))?;

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.execute(script, vec![]).await {
            Ok(result) => Ok(success_response(format!(
                "Script result: {result:?} (session: {session})"
            ))),
            Err(e) => Ok(error_response(format!("Failed to execute script: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Take a screenshot of the current page
pub async fn handle_screenshot(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    let save_path = arguments
        .as_ref()
        .and_then(|args| args.get("save_path"))
        .and_then(|v| v.as_str());

    match client_manager.get_or_create_client(session_id).await {
        Ok((_session, client)) => match client.screenshot().await {
            Ok(png_data) => {
                // Validate that we have valid PNG data
                if png_data.is_empty() {
                    return Ok(error_response("Screenshot data is empty".to_string()));
                }

                // Check if data starts with PNG signature
                if png_data.len() < 4 || &png_data[0..4] != b"\x89PNG" {
                    return Ok(error_response("Screenshot data is not valid PNG format".to_string()));
                }

                // Save to disk if path is provided
                if let Some(path) = save_path {
                    match std::fs::write(path, &png_data) {
                        Ok(_) => {
                            // Also return the image data for display
                            let base64_data = general_purpose::STANDARD.encode(&png_data);
                            Ok(CallToolResult {
                                content: vec![
                                    Content::text(format!("Screenshot saved to: {} ({} bytes)", path, png_data.len())),
                                    Content::image(
                                        base64_data,
                                        "image/png",
                                    )
                                ],
                                is_error: Some(false),
                            })
                        }
                        Err(e) => Ok(error_response(format!("Failed to save screenshot to {path}: {e}"))),
                    }
                } else {
                    // Just return the image data
                    let base64_data = general_purpose::STANDARD.encode(&png_data);
                    Ok(CallToolResult {
                        content: vec![
                            Content::text(format!("Screenshot taken ({} bytes)", png_data.len())),
                            Content::image(
                                base64_data,
                                "image/png",
                            )
                        ],
                        is_error: Some(false),
                    })
                }
            }
            Err(e) => Ok(error_response(format!("Failed to take screenshot: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Resize the browser window
pub async fn handle_resize_window(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    let width = arguments
        .as_ref()
        .and_then(|args| args.get("width"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| McpError::invalid_params("width parameter required", None))?;

    let height = arguments
        .as_ref()
        .and_then(|args| args.get("height"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| McpError::invalid_params("height parameter required", None))?;

    // Validate dimensions
    if width <= 0.0 || height <= 0.0 {
        return Ok(error_response("Width and height must be positive numbers".to_string()));
    }

    if width > 10000.0 || height > 10000.0 {
        return Ok(error_response("Width and height must be less than 10000 pixels".to_string()));
    }

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.set_window_size(width as u32, height as u32).await {
            Ok(_) => {
                // Verify the resize by getting the current size
                match client.get_window_size().await {
                    Ok((actual_width, actual_height)) => Ok(success_response(format!(
                        "Window resized to {}x{} pixels (session: {})",
                        actual_width, actual_height, session
                    ))),
                    Err(_) => Ok(success_response(format!(
                        "Window resize command sent ({}x{}) (session: {})",
                        width, height, session
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!("Failed to resize window: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get the page HTML source
pub async fn handle_get_page_source(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => match client.source().await {
            Ok(html) => Ok(success_response(format!(
                "Page HTML source (session: {session}):\n\n{html}"
            ))),
            Err(e) => Ok(error_response(format!("Failed to get page source: {e}"))),
        },
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

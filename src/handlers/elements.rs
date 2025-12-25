//! Element interaction handlers
//!
//! Handles DOM element operations:
//! - Finding elements (single and multiple)
//! - Element interaction (click, send_keys, hover, scroll)
//! - Element information (attributes, properties, computed styles)
//! - Waiting for elements and conditions
//! - Form filling and submission

use fantoccini::Locator;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    tools::{error_response, success_response},
};
use super::{extract_session_id, extract_wait_timeout};

/// Click an element by CSS selector
pub async fn handle_click(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let wait_timeout = extract_wait_timeout(arguments);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, wait_timeout)
                .await
            {
                Ok(element) => match element.click().await {
                    Ok(_) => Ok(success_response(format!(
                        "Successfully clicked element {selector} (session: {session})"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to click element: {e}"))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find element {selector}: {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Send keys to an element by CSS selector
pub async fn handle_send_keys(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let text = arguments
        .as_ref()
        .and_then(|args| args.get("text"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("text parameter required", None))?;

    let wait_timeout = extract_wait_timeout(arguments);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, wait_timeout)
                .await
            {
                Ok(element) => match element.send_keys(text).await {
                    Ok(_) => Ok(success_response(format!(
                        "Successfully sent keys to element {selector} (session: {session})"
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to send keys: {e}"))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find element {selector}: {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Wait for an element to appear
pub async fn handle_wait_for_element(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let timeout_seconds = arguments
        .as_ref()
        .and_then(|args| args.get("timeout_seconds"))
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, Some(timeout_seconds))
                .await
            {
                Ok(_element) => Ok(success_response(format!(
                    "Element '{selector}' found within {timeout_seconds:.1}s (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!(
                    "Element '{selector}' not found within {timeout_seconds:.1}s: {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Wait for a JavaScript condition to become true
pub async fn handle_wait_for_condition(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let condition = arguments
        .as_ref()
        .and_then(|args| args.get("condition"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("condition parameter required", None))?;

    let timeout_seconds = arguments
        .as_ref()
        .and_then(|args| args.get("timeout_seconds"))
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);

    let check_interval_ms = arguments
        .as_ref()
        .and_then(|args| args.get("check_interval_ms"))
        .and_then(|v| v.as_f64())
        .unwrap_or(100.0) as u64;

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let start_time = std::time::Instant::now();
            let timeout_duration = std::time::Duration::from_secs_f64(timeout_seconds);
            let check_interval = std::time::Duration::from_millis(check_interval_ms);

            loop {
                // Check if condition is true
                match client.execute(condition, vec![]).await {
                    Ok(result) => {
                        // Check if result is truthy
                        let is_true = match result {
                            serde_json::Value::Bool(b) => b,
                            serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                            serde_json::Value::String(s) => !s.is_empty(),
                            serde_json::Value::Array(arr) => !arr.is_empty(),
                            serde_json::Value::Object(obj) => !obj.is_empty(),
                            serde_json::Value::Null => false,
                        };

                        if is_true {
                            let elapsed = start_time.elapsed();
                            return Ok(success_response(format!(
                                "Condition '{}' became true after {:.1}s (session: {})",
                                condition,
                                elapsed.as_secs_f64(),
                                session
                            )));
                        }
                    }
                    Err(e) => {
                        // JavaScript error - condition might be malformed
                        return Ok(error_response(format!(
                            "Error evaluating condition '{}': {}",
                            condition, e
                        )));
                    }
                }

                // Check timeout
                if start_time.elapsed() >= timeout_duration {
                    return Ok(error_response(format!(
                        "Condition '{}' did not become true within {:.1}s (session: {})",
                        condition, timeout_seconds, session
                    )));
                }

                // Wait before next check
                tokio::time::sleep(check_interval).await;
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get comprehensive element information
pub async fn handle_get_element_info(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let include_computed_styles = arguments
        .as_ref()
        .and_then(|args| args.get("include_computed_styles"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let wait_timeout = arguments
        .as_ref()
        .and_then(|args| args.get("wait_timeout"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let _element = if wait_timeout > 0.0 {
                match client_manager
                    .find_element_with_wait(&client, selector, Some(wait_timeout))
                    .await
                {
                    Ok(element) => element,
                    Err(e) => {
                        return Ok(error_response(format!(
                            "Element '{selector}' not found within {wait_timeout:.1}s: {e}"
                        )));
                    }
                }
            } else {
                match client.find(Locator::Css(selector)).await {
                    Ok(element) => element,
                    Err(e) => {
                        return Ok(error_response(format!(
                            "Element '{selector}' not found: {e}"
                        )));
                    }
                }
            };

            // JavaScript to get comprehensive element information
            let info_script = format!(
                r#"
                try {{
                    const element = document.querySelector('{}');
                    if (!element) {{
                        return {{ error: 'Element not found' }};
                    }}

                    const rect = element.getBoundingClientRect();
                    const style = window.getComputedStyle(element);

                    const info = {{
                        tagName: element.tagName.toLowerCase(),
                        id: element.id || null,
                        className: element.className || null,

                        // Visibility
                        isVisible: rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none',
                        isInViewport: rect.top >= 0 && rect.left >= 0 && rect.bottom <= window.innerHeight && rect.right <= window.innerWidth,

                        // Size and position
                        boundingRect: {{
                            x: Math.round(rect.x),
                            y: Math.round(rect.y),
                            width: Math.round(rect.width),
                            height: Math.round(rect.height),
                            top: Math.round(rect.top),
                            right: Math.round(rect.right),
                            bottom: Math.round(rect.bottom),
                            left: Math.round(rect.left)
                        }},

                        // Offset dimensions
                        offsetWidth: element.offsetWidth,
                        offsetHeight: element.offsetHeight,
                        offsetTop: element.offsetTop,
                        offsetLeft: element.offsetLeft,

                        // Client dimensions
                        clientWidth: element.clientWidth,
                        clientHeight: element.clientHeight,

                        // Scroll dimensions
                        scrollWidth: element.scrollWidth,
                        scrollHeight: element.scrollHeight,
                        scrollTop: element.scrollTop,
                        scrollLeft: element.scrollLeft,

                        // Key computed styles
                        computedStyles: {{
                            display: style.display,
                            visibility: style.visibility,
                            opacity: style.opacity,
                            position: style.position,
                            zIndex: style.zIndex,
                            overflow: style.overflow,
                            overflowX: style.overflowX,
                            overflowY: style.overflowY
                        }}{}
                    }};

                    return info;
                }} catch (e) {{
                    return {{ error: e.message }};
                }}
                "#,
                selector.replace('\'', "\\'"),
                if include_computed_styles {
                    r#",
                        allComputedStyles: {
                            width: style.width,
                            height: style.height,
                            margin: style.margin,
                            padding: style.padding,
                            border: style.border,
                            backgroundColor: style.backgroundColor,
                            color: style.color,
                            fontSize: style.fontSize,
                            fontFamily: style.fontFamily,
                            lineHeight: style.lineHeight,
                            textAlign: style.textAlign,
                            transform: style.transform,
                            transition: style.transition,
                            animation: style.animation
                        }"#
                } else {
                    ""
                }
            );

            match client.execute(&info_script, vec![]).await {
                Ok(result) => {
                    if let Ok(info) = serde_json::from_value::<serde_json::Map<String, serde_json::Value>>(result.clone()) {
                        if let Some(error) = info.get("error") {
                            Ok(error_response(format!("JavaScript error: {}", error)))
                        } else {
                            let formatted_info = serde_json::to_string_pretty(&info)
                                .unwrap_or_else(|_| format!("{:?}", info));
                            Ok(success_response(format!(
                                "Element info for '{}' (session: {}):\n{}",
                                selector, session, formatted_info
                            )))
                        }
                    } else {
                        Ok(error_response(format!("Failed to parse element info: {:?}", result)))
                    }
                }
                Err(e) => Ok(error_response(format!("Failed to get element info: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get an element's HTML attribute
pub async fn handle_get_element_attribute(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let attribute = arguments
        .as_ref()
        .and_then(|args| args.get("attribute"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("attribute parameter required", None))?;

    let wait_timeout = extract_wait_timeout(arguments);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, wait_timeout)
                .await
            {
                Ok(element) => match element.attr(attribute).await {
                    Ok(attr_value) => {
                        let value_text = attr_value.unwrap_or_else(|| {
                            format!("[attribute '{attribute}' not found or empty]")
                        });
                        Ok(success_response(format!(
                            "Element '{selector}' attribute '{attribute}': {value_text} (session: {session})"
                        )))
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to get attribute '{attribute}' from element '{selector}': {e}"
                    ))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find element '{selector}': {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get an element's JavaScript property
pub async fn handle_get_element_property(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let property = arguments
        .as_ref()
        .and_then(|args| args.get("property"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("property parameter required", None))?;

    let wait_timeout = extract_wait_timeout(arguments);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, wait_timeout)
                .await
            {
                Ok(element) => match element.prop(property).await {
                    Ok(prop_value) => {
                        let value_text = match prop_value {
                            Some(s) => s,
                            None => "[null/undefined]".to_string(),
                        };
                        Ok(success_response(format!(
                            "Element '{selector}' property '{property}': {value_text} (session: {session})"
                        )))
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to get property '{property}' from element '{selector}': {e}"
                    ))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find element '{selector}': {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Find a single element by CSS selector
pub async fn handle_find_element(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let parent_selector = arguments
        .as_ref()
        .and_then(|args| args.get("parent_selector"))
        .and_then(|v| v.as_str());

    let wait_timeout = arguments
        .as_ref()
        .and_then(|args| args.get("wait_timeout"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            // If parent_selector is provided, find within parent
            let search_result = if let Some(parent_sel) = parent_selector {
                // First find the parent element
                let parent_element = if wait_timeout > 0.0 {
                    match client_manager
                        .find_element_with_wait(&client, parent_sel, Some(wait_timeout))
                        .await
                    {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Parent element '{}' not found within {:.1}s: {}",
                                parent_sel, wait_timeout, e
                            )));
                        }
                    }
                } else {
                    match client.find(Locator::Css(parent_sel)).await {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Parent element '{}' not found: {}",
                                parent_sel, e
                            )));
                        }
                    }
                };

                // Then find child element within parent
                parent_element.find(Locator::Css(selector)).await
                    .map_err(|e| format!("Child element '{}' not found within parent '{}': {}", selector, parent_sel, e))
            } else {
                // Standard search without parent
                if wait_timeout > 0.0 {
                    client_manager
                        .find_element_with_wait(&client, selector, Some(wait_timeout))
                        .await
                        .map_err(|e| format!("Element '{}' not found within {:.1}s: {}", selector, wait_timeout, e))
                } else {
                    client.find(Locator::Css(selector)).await
                        .map_err(|e| format!("Element '{}' not found: {}", selector, e))
                }
            };

            match search_result {
                Ok(element) => {
                    let tag_name = element
                        .tag_name()
                        .await
                        .unwrap_or_else(|_| "unknown".to_string());
                    let text_content = element
                        .text()
                        .await
                        .unwrap_or_else(|_| "[no text]".to_string());
                    let text_preview = if text_content.len() > 100 {
                        format!("{}...", &text_content[..97])
                    } else {
                        text_content
                    };

                    let scope_msg = if let Some(parent_sel) = parent_selector {
                        format!(" within parent '{}'", parent_sel)
                    } else {
                        String::new()
                    };

                    Ok(success_response(format!(
                        "Found element '{}'{} (session: {}): <{}> - Text: \"{}\"",
                        selector, scope_msg, session, tag_name, text_preview
                    )))
                }
                Err(e) => Ok(error_response(e)),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Find multiple elements by CSS selector
pub async fn handle_find_elements(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let parent_selector = arguments
        .as_ref()
        .and_then(|args| args.get("parent_selector"))
        .and_then(|v| v.as_str());

    let wait_timeout = arguments
        .as_ref()
        .and_then(|args| args.get("wait_timeout"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            // If parent_selector is provided, find within parent
            let search_result = if let Some(parent_sel) = parent_selector {
                // First find the parent element
                let parent_element = if wait_timeout > 0.0 {
                    match client_manager
                        .find_element_with_wait(&client, parent_sel, Some(wait_timeout))
                        .await
                    {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Parent element '{}' not found within {:.1}s: {}",
                                parent_sel, wait_timeout, e
                            )));
                        }
                    }
                } else {
                    match client.find(Locator::Css(parent_sel)).await {
                        Ok(element) => element,
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Parent element '{}' not found: {}",
                                parent_sel, e
                            )));
                        }
                    }
                };

                // Then find child elements within parent
                parent_element.find_all(Locator::Css(selector)).await
                    .map_err(|e| format!("Child elements '{}' not found within parent '{}': {}", selector, parent_sel, e))
            } else {
                // Standard search without parent
                client.find_all(Locator::Css(selector)).await
                    .map_err(|e| format!("Elements '{}' not found: {}", selector, e))
            };

            match search_result {
                Ok(elements) => {
                    let scope_msg = if let Some(parent_sel) = parent_selector {
                        format!(" within parent '{}'", parent_sel)
                    } else {
                        String::new()
                    };

                    let mut result_text = format!(
                        "Found {} element(s) matching '{}'{} (session: {}):\n\n",
                        elements.len(),
                        selector,
                        scope_msg,
                        session
                    );

                    for (i, element) in elements.iter().enumerate() {
                        let tag_name = element
                            .tag_name()
                            .await
                            .unwrap_or_else(|_| "unknown".to_string());
                        let text_content = element
                            .text()
                            .await
                            .unwrap_or_else(|_| "[no text]".to_string());
                        let text_preview = if text_content.len() > 100 {
                            format!("{}...", &text_content[..97])
                        } else {
                            text_content
                        };

                        result_text.push_str(&format!(
                            "{}. <{}> - Text: \"{}\"\n",
                            i + 1,
                            tag_name,
                            text_preview
                        ));
                    }

                    Ok(success_response(result_text))
                }
                Err(e) => Ok(error_response(e)),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Scroll to an element
pub async fn handle_scroll_to_element(
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
        Ok((session, client)) => {
            // First, try to find the element
            match client.find(Locator::Css(selector)).await {
                Ok(_element) => {
                    // Scroll the element into view using JavaScript with CSS selector
                    let scroll_script = format!(
                        "var element = document.querySelector('{}'); if (element) {{ element.scrollIntoView({{behavior: 'smooth', block: 'center'}}); }}",
                        selector.replace("'", "\\'")
                    );

                    match client.execute(&scroll_script, vec![]).await {
                        Ok(_) => {
                            // Wait a moment for smooth scrolling to complete
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            Ok(success_response(format!(
                                "Successfully scrolled to element '{selector}' (session: {session})"
                            )))
                        }
                        Err(e) => {
                            Ok(error_response(format!("Failed to scroll to element: {e}")))
                        }
                    }
                }
                Err(e) => Ok(error_response(format!(
                    "Failed to find element '{selector}': {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Hover over an element
pub async fn handle_hover(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let selector = arguments
        .as_ref()
        .and_then(|args| args.get("selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

    let wait_timeout = extract_wait_timeout(arguments);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            match client_manager
                .find_element_with_wait(&client, selector, wait_timeout)
                .await
            {
                Ok(_element) => {
                    // Use JavaScript to trigger mouse hover events
                    let hover_script = format!(
                        r#"
                        var element = document.querySelector('{}');
                        if (element) {{
                            var events = ['mouseenter', 'mouseover'];
                            events.forEach(function(eventType) {{
                                var event = new MouseEvent(eventType, {{
                                    'view': window,
                                    'bubbles': true,
                                    'cancelable': true
                                }});
                                element.dispatchEvent(event);
                            }});
                        }}
                        "#,
                        selector.replace("'", "\\'")
                    );

                    match client.execute(&hover_script, vec![]).await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully hovered over element '{selector}' (session: {session})"
                        ))),
                        Err(e) => {
                            Ok(error_response(format!("Failed to hover over element: {e}")))
                        }
                    }
                }
                Err(e) => Ok(error_response(format!(
                    "Failed to find element '{selector}': {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Fill form fields and submit
pub async fn handle_fill_and_submit_form(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let fields = arguments
        .as_ref()
        .and_then(|args| args.get("fields"))
        .and_then(|v| v.as_object())
        .ok_or_else(|| McpError::invalid_params("fields parameter required", None))?;

    let submit_selector = arguments
        .as_ref()
        .and_then(|args| args.get("submit_selector"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("submit_selector parameter required", None))?;

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let mut filled_fields = Vec::new();

            // Fill each field
            for (field_selector, value) in fields {
                if let Some(text_value) = value.as_str() {
                    match client.find(Locator::Css(field_selector)).await {
                        Ok(element) => {
                            // Clear the field first
                            if let Err(e) = element.clear().await {
                                return Ok(error_response(format!(
                                    "Failed to clear field '{field_selector}': {e}"
                                )));
                            }

                            // Then send keys
                            if let Err(e) = element.send_keys(text_value).await {
                                return Ok(error_response(format!(
                                    "Failed to fill field '{field_selector}': {e}"
                                )));
                            }

                            filled_fields.push(field_selector.clone());
                        }
                        Err(e) => {
                            return Ok(error_response(format!(
                                "Failed to find field '{field_selector}': {e}"
                            )));
                        }
                    }
                }
            }

            // Submit the form
            match client.find(Locator::Css(submit_selector)).await {
                Ok(submit_element) => match submit_element.click().await {
                    Ok(_) => Ok(success_response(format!(
                        "Successfully filled {} fields and submitted form (session: {}). Fields: {}",
                        filled_fields.len(),
                        session,
                        filled_fields.join(", ")
                    ))),
                    Err(e) => Ok(error_response(format!("Failed to submit form: {e}"))),
                },
                Err(e) => Ok(error_response(format!(
                    "Failed to find submit element '{submit_selector}': {e}"
                ))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Smart login form handler with auto-detection
pub async fn handle_login_form(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let username = arguments
        .as_ref()
        .and_then(|args| args.get("username"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("username parameter required", None))?;

    let password = arguments
        .as_ref()
        .and_then(|args| args.get("password"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("password parameter required", None))?;

    // Get optional custom selectors
    let username_selector = arguments
        .as_ref()
        .and_then(|args| args.get("username_selector"))
        .and_then(|v| v.as_str());

    let password_selector = arguments
        .as_ref()
        .and_then(|args| args.get("password_selector"))
        .and_then(|v| v.as_str());

    let submit_selector = arguments
        .as_ref()
        .and_then(|args| args.get("submit_selector"))
        .and_then(|v| v.as_str());

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            // Define common login field selectors to try
            let default_username_selectors = vec![
                "input[type='email']",
                "input[type='text'][name*='user']",
                "input[type='text'][name*='email']",
                "input[name='username']",
                "input[name='email']",
                "input[id*='user']",
                "input[id*='email']",
                "#username",
                "#email",
                "[placeholder*='email' i]",
                "[placeholder*='username' i]",
            ];

            let default_password_selectors = vec![
                "input[type='password']",
                "input[name='password']",
                "#password",
                "[placeholder*='password' i]",
            ];

            let default_submit_selectors = vec![
                "button[type='submit']",
                "input[type='submit']",
                "button:contains('Sign in')",
                "button:contains('Login')",
                "button:contains('Log in')",
                "[role='button']:contains('Sign in')",
                "[role='button']:contains('Login')",
                "button",
            ];

            // Try to find and fill username field
            let username_found = if let Some(selector) = username_selector {
                // Use custom selector
                match client.find(Locator::Css(selector)).await {
                    Ok(element) => {
                        if let Err(e) = element.clear().await {
                            return Ok(error_response(format!(
                                "Failed to clear username field '{selector}': {e}"
                            )));
                        }
                        if let Err(e) = element.send_keys(username).await {
                            return Ok(error_response(format!(
                                "Failed to fill username field '{selector}': {e}"
                            )));
                        }
                        true
                    }
                    Err(e) => {
                        return Ok(error_response(format!(
                            "Failed to find username field with custom selector '{selector}': {e}"
                        )));
                    }
                }
            } else {
                // Try default selectors
                let mut found = false;
                for selector in &default_username_selectors {
                    if let Ok(element) = client.find(Locator::Css(selector)).await {
                        if element.clear().await.is_ok() && element.send_keys(username).await.is_ok() {
                            found = true;
                            break;
                        }
                    }
                }
                found
            };

            if !username_found {
                return Ok(error_response(
                    "Could not find username/email field. Try providing a custom username_selector".to_string()
                ));
            }

            // Try to find and fill password field
            let password_found = if let Some(selector) = password_selector {
                // Use custom selector
                match client.find(Locator::Css(selector)).await {
                    Ok(element) => {
                        if let Err(e) = element.clear().await {
                            return Ok(error_response(format!(
                                "Failed to clear password field '{selector}': {e}"
                            )));
                        }
                        if let Err(e) = element.send_keys(password).await {
                            return Ok(error_response(format!(
                                "Failed to fill password field '{selector}': {e}"
                            )));
                        }
                        true
                    }
                    Err(e) => {
                        return Ok(error_response(format!(
                            "Failed to find password field with custom selector '{selector}': {e}"
                        )));
                    }
                }
            } else {
                // Try default selectors
                let mut found = false;
                for selector in &default_password_selectors {
                    if let Ok(element) = client.find(Locator::Css(selector)).await {
                        if element.clear().await.is_ok() && element.send_keys(password).await.is_ok() {
                            found = true;
                            break;
                        }
                    }
                }
                found
            };

            if !password_found {
                return Ok(error_response(
                    "Could not find password field. Try providing a custom password_selector".to_string()
                ));
            }

            // Try to find and click submit button
            if let Some(selector) = submit_selector {
                // Use custom selector
                match client.find(Locator::Css(selector)).await {
                    Ok(element) => match element.click().await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully filled login form and submitted (session: {session})"
                        ))),
                        Err(e) => Ok(error_response(format!(
                            "Login form filled but failed to click submit button. Error: {e}"
                        ))),
                    },
                    Err(e) => Ok(error_response(format!(
                        "Failed to find submit button with custom selector '{selector}': {e}"
                    ))),
                }
            } else {
                // Try default selectors
                let mut submit_clicked = false;
                for selector in &default_submit_selectors {
                    if let Ok(element) = client.find(Locator::Css(selector)).await {
                        if element.click().await.is_ok() {
                            submit_clicked = true;
                            break;
                        }
                    }
                }
                if submit_clicked {
                    Ok(success_response(format!(
                        "Successfully filled login form and submitted (session: {session})"
                    )))
                } else {
                    Ok(error_response(
                        "Could not find submit button. Try providing a custom submit_selector".to_string()
                    ))
                }
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

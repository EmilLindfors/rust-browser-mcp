use base64::{Engine as _, engine::general_purpose};
use fantoccini::Locator;
use rmcp::{ErrorData as McpError, ServerHandler, model::*};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    config::Config,
    tools::{ToolDefinitions, error_response, success_response},
};

#[derive(Clone)]
pub struct WebDriverServer {
    client_manager: ClientManager,
}

impl WebDriverServer {
    pub fn new() -> crate::error::Result<Self> {
        let config = Config::from_env();
        Ok(Self {
            client_manager: ClientManager::new(config)?,
        })
    }

    pub fn with_config(config: Config) -> crate::error::Result<Self> {
        Ok(Self {
            client_manager: ClientManager::new(config)?,
        })
    }

    /// Cleanup method to stop any managed driver processes
    pub fn cleanup(&self) -> crate::error::Result<()> {
        // Access the driver manager through the client manager would require exposing it
        // For now, we rely on the Drop implementation of DriverManager
        tracing::info!("WebDriver MCP Server shutting down...");
        Ok(())
    }

    fn extract_session_id(arguments: &Option<Map<String, Value>>) -> Option<String> {
        arguments
            .as_ref()
            .and_then(|args| args.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    async fn handle_navigate(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let url = arguments
            .as_ref()
            .and_then(|args| args.get("url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("url parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.goto(url).await {
                Ok(_) => Ok(success_response(format!(
                    "Successfully navigated to {url} (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to navigate: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_click(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
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

    async fn handle_send_keys(
        &self,
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

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
                    .find_element_with_wait(&client, selector, wait_timeout)
                    .await
                {
                    Ok(element) => match element.send_keys(text).await {
                        Ok(_) => Ok(success_response(format!(
                            "Successfully sent keys to {selector} (session: {session})"
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

    async fn handle_get_title(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_get_text(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_execute_script(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let script = arguments
            .as_ref()
            .and_then(|args| args.get("script"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("script parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_screenshot(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((_session, client)) => match client.screenshot().await {
                Ok(png_data) => {
                    let base64_data = general_purpose::STANDARD.encode(&png_data);
                    Ok(CallToolResult {
                        content: vec![Content::image(
                            format!("data:image/png;base64,{base64_data}"),
                            "image/png",
                        )],
                        is_error: Some(false),
                    })
                }
                Err(e) => Ok(error_response(format!("Failed to take screenshot: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_current_url(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_back(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_forward(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_refresh(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.refresh().await {
                Ok(_) => Ok(success_response(format!(
                    "Successfully refreshed page (session: {session})"
                ))),
                Err(e) => Ok(error_response(format!("Failed to refresh page: {e}"))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_get_page_load_status(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match client.execute("return document.readyState", vec![]).await {
                    Ok(result) => {
                        let ready_state = result.as_str().unwrap_or("unknown");
                        let status_msg = match ready_state {
                            "complete" => "Page fully loaded",
                            "interactive" => "Page loaded but resources may still be loading",
                            "loading" => "Page still loading",
                            _ => "Unknown page state",
                        };
                        Ok(success_response(format!(
                            "Page load status: {status_msg} ({ready_state}) (session: {session})"
                        )))
                    }
                    Err(e) => Ok(error_response(format!(
                        "Failed to check page load status: {e}"
                    ))),
                }
            }
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_wait_for_element(
        &self,
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

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
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

    async fn handle_get_element_attribute(
        &self,
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

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
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

    async fn handle_get_page_source(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_get_element_property(
        &self,
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

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
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

    async fn handle_find_element(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.find(Locator::Css(selector)).await {
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

                    Ok(success_response(format!(
                        "Found element '{selector}' (session: {session}): <{tag_name}> - Text: \"{text_preview}\""
                    )))
                }
                Err(e) => Ok(error_response(format!(
                    "Failed to find element '{selector}': {e}"
                ))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_find_elements(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => match client.find_all(Locator::Css(selector)).await {
                Ok(elements) => {
                    let mut result_text = format!(
                        "Found {} element(s) matching '{}' (session: {}):\n\n",
                        elements.len(),
                        selector,
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
                Err(e) => Ok(error_response(format!(
                    "Failed to find elements '{selector}': {e}"
                ))),
            },
            Err(e) => Ok(error_response(format!(
                "Failed to create webdriver client: {e}"
            ))),
        }
    }

    async fn handle_scroll_to_element(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    async fn handle_hover(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let selector = arguments
            .as_ref()
            .and_then(|args| args.get("selector"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("selector parameter required", None))?;

        let wait_timeout = arguments
            .as_ref()
            .and_then(|args| args.get("wait_timeout"))
            .and_then(|v| v.as_f64());

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
            Ok((session, client)) => {
                match self
                    .client_manager
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

    async fn handle_fill_and_submit_form(
        &self,
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

        let session_id = Self::extract_session_id(arguments);

        match self.client_manager.get_or_create_client(session_id).await {
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

    // WebDriver lifecycle management handlers

    async fn handle_start_driver(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_type_str = arguments
            .as_ref()
            .and_then(|args| args.get("driver_type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Missing or invalid driver_type parameter", None)
            })?;

        let driver_type = match driver_type_str {
            "firefox" => crate::driver::DriverType::Firefox,
            "chrome" => crate::driver::DriverType::Chrome,
            "edge" => crate::driver::DriverType::Edge,
            _ => {
                return Err(McpError::invalid_params(
                    "Invalid driver_type. Must be 'firefox', 'chrome', or 'edge'",
                    None,
                ));
            }
        };

        // Get access to the driver manager through client manager
        match self
            .client_manager
            .get_driver_manager()
            .start_driver_manually(driver_type.clone())
            .await
        {
            Ok(endpoint) => {
                let pid_info = self
                    .client_manager
                    .get_driver_manager()
                    .get_managed_processes_status()
                    .iter()
                    .find(|(dt, _, _)| dt == &driver_type)
                    .map(|(_, pid, port)| format!(" (PID: {pid}, Port: {port})"))
                    .unwrap_or_default();

                Ok(success_response(format!(
                    "Successfully started {} WebDriver at {}{}",
                    driver_type.browser_name(),
                    endpoint,
                    pid_info
                )))
            }
            Err(e) => Ok(error_response(format!(
                "Failed to start {} WebDriver: {}",
                driver_type.browser_name(),
                e
            ))),
        }
    }

    async fn handle_stop_driver(
        &self,
        arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let driver_type_str = arguments
            .as_ref()
            .and_then(|args| args.get("driver_type"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                McpError::invalid_params("Missing or invalid driver_type parameter", None)
            })?;

        let driver_type = match driver_type_str {
            "firefox" => crate::driver::DriverType::Firefox,
            "chrome" => crate::driver::DriverType::Chrome,
            "edge" => crate::driver::DriverType::Edge,
            _ => {
                return Err(McpError::invalid_params(
                    "Invalid driver_type. Must be 'firefox', 'chrome', or 'edge'",
                    None,
                ));
            }
        };

        match self
            .client_manager
            .get_driver_manager()
            .stop_driver_by_type(&driver_type)
            .await
        {
            Ok(()) => Ok(success_response(format!(
                "Successfully stopped {} WebDriver processes",
                driver_type.browser_name()
            ))),
            Err(e) => Ok(error_response(format!(
                "Failed to stop {} WebDriver: {}",
                driver_type.browser_name(),
                e
            ))),
        }
    }

    async fn handle_stop_all_drivers(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        match self
            .client_manager
            .get_driver_manager()
            .stop_all_drivers()
            .await
        {
            Ok(()) => Ok(success_response(
                "Successfully stopped all managed WebDriver processes".to_string(),
            )),
            Err(e) => Ok(error_response(format!(
                "Failed to stop WebDriver processes: {e}"
            ))),
        }
    }

    async fn handle_list_managed_drivers(
        &self,
        _arguments: &Option<Map<String, Value>>,
    ) -> Result<CallToolResult, McpError> {
        let processes = self
            .client_manager
            .get_driver_manager()
            .get_managed_processes_status();

        if processes.is_empty() {
            Ok(success_response(
                "No managed WebDriver processes currently running".to_string(),
            ))
        } else {
            let mut status_lines = vec!["Managed WebDriver processes:".to_string()];
            for (driver_type, pid, port) in processes {
                status_lines.push(format!(
                    "  - {} (PID: {}, Port: {})",
                    driver_type.browser_name(),
                    pid,
                    port
                ));
            }
            Ok(success_response(status_lines.join("\n")))
        }
    }
}

impl ServerHandler for WebDriverServer {
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult {
            tools: ToolDefinitions::list_all(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        match request.name.as_ref() {
            "navigate" => self.handle_navigate(&request.arguments).await,
            "click" => self.handle_click(&request.arguments).await,
            "send_keys" => self.handle_send_keys(&request.arguments).await,
            "get_title" => self.handle_get_title(&request.arguments).await,
            "get_text" => self.handle_get_text(&request.arguments).await,
            "execute_script" => self.handle_execute_script(&request.arguments).await,
            "screenshot" => self.handle_screenshot(&request.arguments).await,
            "get_current_url" => self.handle_get_current_url(&request.arguments).await,
            "back" => self.handle_back(&request.arguments).await,
            "forward" => self.handle_forward(&request.arguments).await,
            "refresh" => self.handle_refresh(&request.arguments).await,
            "get_page_load_status" => self.handle_get_page_load_status(&request.arguments).await,
            "wait_for_element" => self.handle_wait_for_element(&request.arguments).await,
            "get_attribute" => self.handle_get_element_attribute(&request.arguments).await,
            "get_page_source" => self.handle_get_page_source(&request.arguments).await,
            "get_property" => self.handle_get_element_property(&request.arguments).await,
            "find_elements" => self.handle_find_elements(&request.arguments).await,
            "find_element" => self.handle_find_element(&request.arguments).await,
            "scroll_to_element" => self.handle_scroll_to_element(&request.arguments).await,
            "hover" => self.handle_hover(&request.arguments).await,
            "fill_and_submit_form" => self.handle_fill_and_submit_form(&request.arguments).await,
            // WebDriver lifecycle management
            "start_driver" => self.handle_start_driver(&request.arguments).await,
            "stop_driver" => self.handle_stop_driver(&request.arguments).await,
            "stop_all_drivers" => self.handle_stop_all_drivers(&request.arguments).await,
            "list_managed_drivers" => self.handle_list_managed_drivers(&request.arguments).await,
            _ => Err(McpError::method_not_found::<CallToolRequestMethod>()),
        }
    }
}

impl Default for WebDriverServer {
    fn default() -> Self {
        Self::new().expect("Failed to create WebDriverServer with default config")
    }
}

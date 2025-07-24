use std::sync::Arc;

use rmcp::model::{Content, Tool};
use serde_json::json;

pub struct ToolDefinitions;

impl ToolDefinitions {
    pub fn list_all() -> Vec<Tool> {
        vec![
            Self::navigation_tool(),
            Self::find_element_tool(),
            Self::click_tool(),
            Self::send_keys_tool(),
            Self::get_title_tool(),
            Self::get_text_tool(),
            Self::execute_script_tool(),
            Self::get_current_url_tool(),
            Self::back_tool(),
            Self::forward_tool(),
            Self::refresh_tool(),
            Self::get_page_load_status_tool(),
            Self::screenshot_tool(),
            Self::wait_for_element_tool(),
            Self::get_element_attribute_tool(),
            Self::get_page_source_tool(),
            Self::get_element_property_tool(),
            Self::find_elements_tool(),
            Self::scroll_to_element_tool(),
            Self::hover_tool(),
            Self::fill_and_submit_form_tool(),
            Self::login_form_tool(),
            Self::get_console_logs_tool(),
            // WebDriver lifecycle management tools
            Self::start_driver_tool(),
            Self::stop_driver_tool(),
            Self::stop_all_drivers_tool(),
            Self::list_managed_drivers_tool(),
        ]
    }

    fn navigation_tool() -> Tool {
        Tool {
            name: "navigate".into(),
            description: Some("Navigate to a URL".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "url": {
                            "type": "string",
                            "description": "URL to navigate to"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["url"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn find_element_tool() -> Tool {
        Tool {
            name: "find_element".into(),
            description: Some("Find an element by CSS selector".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to find element"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["selector"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn click_tool() -> Tool {
        Tool {
            name: "click".into(),
            description: Some("Click an element by CSS selector".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element to click"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds for element to appear (default: 0 = no wait)"
                    }
                },
                "required": ["selector"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn send_keys_tool() -> Tool {
        Tool {
            name: "send_keys".into(),
            description: Some("Send keys/text to an element".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element to send keys to"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to send"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds for element to appear (default: 0 = no wait)"
                    }
                },
                "required": ["selector", "text"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn get_title_tool() -> Tool {
        Tool {
            name: "get_title".into(),
            description: Some("Get the page title".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_text_tool() -> Tool {
        Tool {
            name: "get_text".into(),
            description: Some("Get text content of an element".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector of element"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["selector"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn execute_script_tool() -> Tool {
        Tool {
            name: "execute_script".into(),
            description: Some("Execute JavaScript code".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "script": {
                            "type": "string",
                            "description": "JavaScript code to execute"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["script"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_current_url_tool() -> Tool {
        Tool {
            name: "get_current_url".into(),
            description: Some("Get the current URL of the browser".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn back_tool() -> Tool {
        Tool {
            name: "back".into(),
            description: Some("Navigate back to the previous page in browser history".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn forward_tool() -> Tool {
        Tool {
            name: "forward".into(),
            description: Some("Navigate forward to the next page in browser history".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn refresh_tool() -> Tool {
        Tool {
            name: "refresh".into(),
            description: Some("Refresh/reload the current page".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_page_load_status_tool() -> Tool {
        Tool {
            name: "get_page_load_status".into(),
            description: Some("Check if the page has finished loading".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn screenshot_tool() -> Tool {
        Tool {
            name: "screenshot".into(),
            description: Some("Take a screenshot of the current page and optionally save to disk".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        },
                        "save_path": {
                            "type": "string",
                            "description": "Optional file path to save the screenshot (e.g., '/path/to/screenshot.png')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn wait_for_element_tool() -> Tool {
        Tool {
            name: "wait_for_element".into(),
            description: Some(
                "Wait for an element to appear on the page with configurable timeout".into(),
            ),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector of element to wait for"
                        },
                        "timeout_seconds": {
                            "type": "number",
                            "description": "Maximum time to wait in seconds (default: 10)"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["selector"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_element_attribute_tool() -> Tool {
        Tool {
            name: "get_attribute".into(),
            description: Some("Get an HTML attribute value from an element (href, src, class, id, etc.)".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element"
                    },
                    "attribute": {
                        "type": "string", 
                        "description": "HTML attribute name (e.g., href, src, class, id, alt)"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds for element to appear (default: 0 = no wait)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["selector", "attribute"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn get_page_source_tool() -> Tool {
        Tool {
            name: "get_page_source".into(),
            description: Some("Get the full HTML source code of the current page".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    }
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_element_property_tool() -> Tool {
        Tool {
            name: "get_property".into(),
            description: Some("Get a DOM property value from an element (value, checked, disabled, etc.)".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element"
                    },
                    "property": {
                        "type": "string",
                        "description": "DOM property name (e.g., value, checked, disabled, selected)"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds for element to appear (default: 0 = no wait)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["selector", "property"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn find_elements_tool() -> Tool {
        Tool {
            name: "find_elements".into(),
            description: Some(
                "Find all elements matching a CSS selector and get basic info about each".into(),
            ),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to find elements"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["selector"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn scroll_to_element_tool() -> Tool {
        Tool {
            name: "scroll_to_element".into(),
            description: Some("Scroll to make an element visible on the page".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector of element to scroll to"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default')"
                        }
                    },
                    "required": ["selector"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn hover_tool() -> Tool {
        Tool {
            name: "hover".into(),
            description: Some("Hover over an element to reveal dropdowns or tooltips".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element to hover over"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds for element to appear (default: 0 = no wait)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["selector"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn fill_and_submit_form_tool() -> Tool {
        Tool {
            name: "fill_and_submit_form".into(),
            description: Some("Fill out a form with multiple fields and submit it".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "form_selector": {
                        "type": "string",
                        "description": "CSS selector of the form element (optional, for validation)"
                    },
                    "fields": {
                        "type": "object",
                        "description": "Object mapping CSS selectors to values to fill",
                        "additionalProperties": {
                            "type": "string"
                        }
                    },
                    "submit_selector": {
                        "type": "string",
                        "description": "CSS selector of the submit button or element"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["fields", "submit_selector"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn login_form_tool() -> Tool {
        Tool {
            name: "login_form".into(),
            description: Some("Automatically fill and submit a login form with username/email and password".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "username": {
                        "type": "string",
                        "description": "Username or email to enter in the login form"
                    },
                    "password": {
                        "type": "string",
                        "description": "Password to enter in the login form"
                    },
                    "username_selector": {
                        "type": "string",
                        "description": "Optional custom CSS selector for username field (will auto-detect if not provided)"
                    },
                    "password_selector": {
                        "type": "string",
                        "description": "Optional custom CSS selector for password field (will auto-detect if not provided)"
                    },
                    "submit_selector": {
                        "type": "string",
                        "description": "Optional custom CSS selector for submit button (will auto-detect if not provided)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["username", "password"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn get_console_logs_tool() -> Tool {
        Tool {
            name: "get_console_logs".into(),
            description: Some("Capture browser console logs, errors, and warnings for debugging".into()),
            input_schema: Arc::new(json!({
                "type": "object", 
                "properties": {
                    "level": {
                        "type": "string",
                        "enum": ["all", "error", "warn", "info", "debug"],
                        "description": "Filter logs by level (default: 'all')"
                    },
                    "since_timestamp": {
                        "type": "number",
                        "description": "Optional: Only return logs since this timestamp (milliseconds)"
                    },
                    "wait_timeout": {
                        "type": "number",
                        "description": "Wait up to this many seconds before capturing logs to allow JavaScript execution (default: 2.0 seconds)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                }
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn start_driver_tool() -> Tool {
        Tool {
            name: "start_driver".into(),
            description: Some("Start a WebDriver process (geckodriver, chromedriver, etc.)".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "driver_type": {
                            "type": "string",
                            "enum": ["firefox", "chrome", "edge"],
                            "description": "Type of WebDriver to start"
                        }
                    },
                    "required": ["driver_type"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn stop_driver_tool() -> Tool {
        Tool {
            name: "stop_driver".into(),
            description: Some("Stop a specific type of WebDriver process".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "driver_type": {
                            "type": "string",
                            "enum": ["firefox", "chrome", "edge"],
                            "description": "Type of WebDriver to stop"
                        }
                    },
                    "required": ["driver_type"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn stop_all_drivers_tool() -> Tool {
        Tool {
            name: "stop_all_drivers".into(),
            description: Some("Stop all managed WebDriver processes".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn list_managed_drivers_tool() -> Tool {
        Tool {
            name: "list_managed_drivers".into(),
            description: Some(
                "List all currently managed WebDriver processes with their status".into(),
            ),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }
}

pub fn success_response(message: String) -> rmcp::model::CallToolResult {
    rmcp::model::CallToolResult {
        content: vec![Content::text(message)],
        is_error: Some(false),
    }
}

pub fn error_response(message: String) -> rmcp::model::CallToolResult {
    rmcp::model::CallToolResult {
        content: vec![Content::text(message)],
        is_error: Some(true),
    }
}

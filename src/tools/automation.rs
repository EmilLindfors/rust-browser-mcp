use std::sync::Arc;
use rmcp::model::Tool;
use serde_json::json;

pub struct AutomationTools;

impl AutomationTools {
    pub fn get_tools() -> Vec<Tool> {
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
            Self::resize_window_tool(),
            Self::wait_for_element_tool(),
            Self::wait_for_condition_tool(),
            Self::get_element_info_tool(),
            Self::get_element_attribute_tool(),
            Self::get_page_source_tool(),
            Self::get_element_property_tool(),
            Self::find_elements_tool(),
            Self::scroll_to_element_tool(),
            Self::hover_tool(),
            Self::fill_and_submit_form_tool(),
            Self::login_form_tool(),
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
            description: Some("Find an element by CSS selector, optionally scoped within a parent element. This tool is ideal for finding nested elements in complex structures like charts, forms, or components without requiring complex CSS selectors.".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to find element (e.g., '.button', '#myId', '[data-test=\"item\"]')"
                        },
                        "parent_selector": {
                            "type": "string",
                            "description": "Optional CSS selector for parent container to search within. When provided, the search will be limited to descendants of the matching parent element. Useful for scoped searches like finding '.legend-item' within '.chart-container' or '.error-message' within '.form-section'."
                        },
                        "wait_timeout": {
                            "type": "number",
                            "description": "Wait up to this many seconds for element to appear (default: 0 = no wait). Applies to both parent and child elements."
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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

    fn resize_window_tool() -> Tool {
        Tool {
            name: "resize_window".into(),
            description: Some("Resize the browser window to specific width and height dimensions".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "required": ["width", "height"],
                    "properties": {
                        "width": {
                            "type": "number",
                            "description": "Window width in pixels"
                        },
                        "height": {
                            "type": "number",
                            "description": "Window height in pixels"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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

    fn wait_for_condition_tool() -> Tool {
        Tool {
            name: "wait_for_condition".into(),
            description: Some(
                "Wait for a JavaScript condition to become true with configurable timeout".into(),
            ),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "condition": {
                            "type": "string",
                            "description": "JavaScript expression that should evaluate to true (e.g., 'document.readyState === \"complete\"', 'window.myChart && window.myChart.isReady()')"
                        },
                        "timeout_seconds": {
                            "type": "number",
                            "description": "Maximum time to wait in seconds (default: 10)"
                        },
                        "check_interval_ms": {
                            "type": "number",
                            "description": "How often to check the condition in milliseconds (default: 100)"
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
                        }
                    },
                    "required": ["condition"]
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
            annotations: None,
        }
    }

    fn get_element_info_tool() -> Tool {
        Tool {
            name: "get_element_info".into(),
            description: Some("Get comprehensive information about an element including visibility, size, position, and styling".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "selector": {
                        "type": "string",
                        "description": "CSS selector of element to inspect"
                    },
                    "include_computed_styles": {
                        "type": "boolean",
                        "description": "Include computed CSS styles (default: false)"
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                "Find all elements matching a CSS selector, optionally scoped within a parent element. Returns basic info about each element found. Perfect for finding multiple items like chart data points, form fields, list items, or menu options within a specific container.".into(),
            ),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "selector": {
                            "type": "string",
                            "description": "CSS selector to find elements (e.g., '.data-point', 'li', '[role=\"menuitem\"]')"
                        },
                        "parent_selector": {
                            "type": "string",
                            "description": "Optional CSS selector for parent container to search within. When provided, only elements within the matching parent will be found. Excellent for scoped searches like finding all '.tooltip-item' elements within '.active-tooltip' or all '.legend-entry' within '.chart-legend'."
                        },
                        "wait_timeout": {
                            "type": "number",
                            "description": "Wait up to this many seconds for parent element to appear (default: 0 = no wait). Child elements are found immediately once parent is located."
                        },
                        "session_id": {
                            "type": "string",
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
                            "description": "Optional session ID (defaults to 'default'). Use 'firefox_*' or 'chrome_*' prefixes to specify browser preference."
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
}
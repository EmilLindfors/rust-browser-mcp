use std::sync::Arc;
use rmcp::model::Tool;
use serde_json::json;

pub struct PerformanceTools;

impl PerformanceTools {
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::get_console_logs_tool(),
            Self::get_performance_metrics_tool(),
            Self::monitor_memory_usage_tool(),
            Self::run_performance_test_tool(),
            Self::monitor_resource_usage_tool(),
        ]
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

    fn get_performance_metrics_tool() -> Tool {
        Tool {
            name: "get_performance_metrics".into(),
            description: Some("Get comprehensive performance metrics including timing, navigation, and resource loading data".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "include_resources": {
                        "type": "boolean",
                        "description": "Include resource timing data (default: true)"
                    },
                    "include_navigation": {
                        "type": "boolean", 
                        "description": "Include navigation timing data (default: true)"
                    },
                    "include_paint": {
                        "type": "boolean",
                        "description": "Include paint timing data (default: true)"
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

    fn monitor_memory_usage_tool() -> Tool {
        Tool {
            name: "monitor_memory_usage".into(),
            description: Some("Monitor JavaScript heap memory usage and detect potential memory leaks".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "duration_seconds": {
                        "type": "number",
                        "description": "Duration to monitor in seconds (default: 10)"
                    },
                    "interval_ms": {
                        "type": "number",
                        "description": "Sampling interval in milliseconds (default: 1000)"
                    },
                    "include_gc_info": {
                        "type": "boolean",
                        "description": "Include garbage collection information if available (default: true)"
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

    fn run_performance_test_tool() -> Tool {
        Tool {
            name: "run_performance_test".into(),
            description: Some("Run automated performance test with user interactions and collect comprehensive metrics".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "test_actions": {
                        "type": "array",
                        "description": "Array of actions to perform during test",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {
                                    "type": "string",
                                    "enum": ["click", "scroll", "wait", "navigate"],
                                    "description": "Type of action to perform"
                                },
                                "selector": {
                                    "type": "string",
                                    "description": "CSS selector for click/scroll actions"
                                },
                                "url": {
                                    "type": "string", 
                                    "description": "URL for navigate actions"
                                },
                                "duration_ms": {
                                    "type": "number",
                                    "description": "Duration for wait actions in milliseconds"
                                }
                            },
                            "required": ["type"]
                        }
                    },
                    "iterations": {
                        "type": "number",
                        "description": "Number of test iterations (default: 1)"
                    },
                    "collect_screenshots": {
                        "type": "boolean",
                        "description": "Take screenshots during test (default: false)"
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID (defaults to 'default')"
                    }
                },
                "required": ["test_actions"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn monitor_resource_usage_tool() -> Tool {
        Tool {
            name: "monitor_resource_usage".into(),
            description: Some("Monitor network requests, CPU usage, and rendering performance metrics".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "duration_seconds": {
                        "type": "number",
                        "description": "Duration to monitor in seconds (default: 30)"
                    },
                    "include_network": {
                        "type": "boolean",
                        "description": "Monitor network requests (default: true)"
                    },
                    "include_cpu": {
                        "type": "boolean",
                        "description": "Monitor CPU usage if available (default: true)"
                    },
                    "include_fps": {
                        "type": "boolean",
                        "description": "Monitor frame rate performance (default: true)"
                    },
                    "network_filter": {
                        "type": "string",
                        "description": "Filter network requests by URL pattern (regex)"
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
}
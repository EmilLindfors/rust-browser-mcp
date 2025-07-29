use std::sync::Arc;
use rmcp::model::Tool;
use serde_json::json;

pub struct DriverManagementTools;

impl DriverManagementTools {
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::get_healthy_endpoints_tool(),
            Self::refresh_driver_health_tool(),
            Self::list_managed_drivers_tool(),
            Self::start_driver_tool(),
            Self::stop_driver_tool(),
            Self::stop_all_drivers_tool(),
            Self::force_cleanup_orphaned_processes_tool(),
        ]
    }

    fn get_healthy_endpoints_tool() -> Tool {
        Tool {
            name: "get_healthy_endpoints".into(),
            description: Some("Get list of healthy WebDriver endpoints".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn refresh_driver_health_tool() -> Tool {
        Tool {
            name: "refresh_driver_health".into(),
            description: Some("Refresh health status of all WebDriver endpoints".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn list_managed_drivers_tool() -> Tool {
        Tool {
            name: "list_managed_drivers".into(),
            description: Some("List all managed WebDriver processes and their status".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn start_driver_tool() -> Tool {
        Tool {
            name: "start_driver".into(),
            description: Some("Start a specific WebDriver process".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "driver_type": {
                            "type": "string",
                            "description": "Type of driver to start (chrome, firefox, edge)"
                        }
                    },
                    "required": ["driver_type"]
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn stop_driver_tool() -> Tool {
        Tool {
            name: "stop_driver".into(),
            description: Some("Stop a specific WebDriver process".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {
                        "driver_type": {
                            "type": "string",
                            "description": "Type of driver to stop (chrome, firefox, edge)"
                        }
                    },
                    "required": ["driver_type"]
                }).as_object().unwrap().clone()),
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
                    "properties": {}
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn force_cleanup_orphaned_processes_tool() -> Tool {
        Tool {
            name: "force_cleanup_orphaned_processes".into(),
            description: Some("Force cleanup of all orphaned browser and WebDriver processes. Use this for emergency cleanup when sessions are stuck or consuming excessive resources.".into()),
            input_schema: Arc::new(
                json!({
                    "type": "object",
                    "properties": {}
                }).as_object().unwrap().clone()),
            annotations: None,
        }
    }
}
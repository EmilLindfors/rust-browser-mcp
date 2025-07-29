use std::sync::Arc;
use rmcp::model::Tool;
use serde_json::json;

pub struct RecipeTools;

impl RecipeTools {
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::create_recipe_tool(),
            Self::list_recipes_tool(),
            Self::get_recipe_tool(),
            Self::execute_recipe_tool(),
            Self::delete_recipe_tool(),
            Self::create_recipe_template_tool(),
        ]
    }

    fn create_recipe_tool() -> Tool {
        Tool {
            name: "create_recipe".into(),
            description: Some("Create a new browser automation recipe from JSON".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "recipe_json": {
                        "type": "string",
                        "description": "JSON string containing the recipe definition"
                    }
                },
                "required": ["recipe_json"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn list_recipes_tool() -> Tool {
        Tool {
            name: "list_recipes".into(),
            description: Some("List all available browser automation recipes".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {}
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn get_recipe_tool() -> Tool {
        Tool {
            name: "get_recipe".into(),
            description: Some("Get the JSON definition of a specific recipe".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the recipe to retrieve"
                    }
                },
                "required": ["name"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn execute_recipe_tool() -> Tool {
        Tool {
            name: "execute_recipe".into(),
            description: Some("Execute a browser automation recipe with optional parameters. Supports multi-browser execution when browsers are specified in the recipe.".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the recipe to execute"
                    },
                    "parameters": {
                        "type": "object",
                        "description": "Optional parameters to substitute in the recipe",
                        "additionalProperties": {
                            "type": "string"
                        }
                    },
                    "session_id": {
                        "type": "string",
                        "description": "Optional session ID for browser operations"
                    },
                    "continue_on_error": {
                        "type": "boolean",
                        "description": "Whether to continue execution when individual steps fail (default: false)"
                    }
                },
                "required": ["name"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn delete_recipe_tool() -> Tool {
        Tool {
            name: "delete_recipe".into(),
            description: Some("Delete a browser automation recipe".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Name of the recipe to delete"
                    }
                },
                "required": ["name"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }

    fn create_recipe_template_tool() -> Tool {
        Tool {
            name: "create_recipe_template".into(),
            description: Some("Create a recipe from a predefined template".into()),
            input_schema: Arc::new(json!({
                "type": "object",
                "properties": {
                    "template": {
                        "type": "string",
                        "enum": ["login_and_screenshot", "multi_browser_screenshot", "responsive_test"],
                        "description": "Template type to create"
                    },
                    "base_url": {
                        "type": "string",
                        "description": "Base URL for the template (required for login_and_screenshot)"
                    },
                    "username": {
                        "type": "string",
                        "description": "Username for login templates"
                    },
                    "password": {
                        "type": "string",
                        "description": "Password for login templates"
                    },
                    "url": {
                        "type": "string",
                        "description": "URL for multi_browser_screenshot and responsive_test templates"
                    },
                    "browsers": {
                        "type": "array",
                        "items": {
                            "type": "string",
                            "enum": ["chrome", "firefox", "edge", "auto"]
                        },
                        "description": "List of browsers to use (optional, defaults to ['auto'])"
                    },
                    "resolutions": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "width": { "type": "number" },
                                "height": { "type": "number" }
                            }
                        },
                        "description": "Screen resolutions for responsive_test template"
                    }
                },
                "required": ["template"]
            }).as_object().unwrap().clone()),
            annotations: None,
        }
    }
}
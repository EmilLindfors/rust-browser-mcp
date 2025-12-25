//! Recipe management handlers
//!
//! Handles automation recipe operations:
//! - Creating and deleting recipes
//! - Listing and loading recipes
//! - Executing recipes with parameters
//! - Creating recipes from templates

use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde_json::{Map, Value};

use crate::{
    Recipe,
    recipes::{RecipeManager, RecipeTemplate, RecipeExecutor, ExecutionContext},
    tools::{error_response, success_response},
    WebDriverServer,
};

/// Create a new recipe from JSON
pub async fn handle_create_recipe(
    recipe_manager: &RecipeManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let recipe_json = arguments
        .as_ref()
        .and_then(|args| args.get("recipe_json"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("recipe_json parameter required", None))?;

    match Recipe::from_json(recipe_json) {
        Ok(recipe) => {
            match recipe.validate() {
                Ok(_) => {
                    match recipe_manager.save_recipe(&recipe).await {
                        Ok(file_path) => Ok(success_response(format!(
                            "Recipe '{}' created successfully at {}",
                            recipe.name,
                            file_path.display()
                        ))),
                        Err(e) => Ok(error_response(format!("Failed to save recipe: {}", e))),
                    }
                }
                Err(e) => Ok(error_response(format!("Recipe validation failed: {}", e))),
            }
        }
        Err(e) => Ok(error_response(format!("Invalid recipe JSON: {}", e))),
    }
}

/// List all available recipes
pub async fn handle_list_recipes(
    recipe_manager: &RecipeManager,
    _arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    match recipe_manager.list_recipes().await {
        Ok(recipes) => {
            if recipes.is_empty() {
                Ok(success_response("No recipes found".to_string()))
            } else {
                let mut result = String::from("Available recipes:\n");
                for recipe in recipes {
                    result.push_str(&format!("  {} (v{})", recipe.name, recipe.version));
                    if let Some(desc) = &recipe.description {
                        result.push_str(&format!(" - {}", desc));
                    }
                    result.push_str(&format!(" - {} steps\n", recipe.step_count));
                }
                Ok(success_response(result))
            }
        }
        Err(e) => Ok(error_response(format!("Failed to list recipes: {}", e))),
    }
}

/// Get a recipe by name (returns JSON)
pub async fn handle_get_recipe(
    recipe_manager: &RecipeManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let name = arguments
        .as_ref()
        .and_then(|args| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

    match recipe_manager.load_recipe(name).await {
        Ok(recipe) => {
            match recipe.to_json() {
                Ok(json) => Ok(success_response(json)),
                Err(e) => Ok(error_response(format!("Failed to serialize recipe: {}", e))),
            }
        }
        Err(e) => Ok(error_response(format!("Failed to load recipe '{}': {}", name, e))),
    }
}

/// Execute a recipe with optional parameters
/// Note: This handler requires the full WebDriverServer for recipe execution
pub async fn handle_execute_recipe(
    server: &WebDriverServer,
    recipe_manager: &RecipeManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let name = arguments
        .as_ref()
        .and_then(|args| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

    let parameters: Option<std::collections::HashMap<String, String>> = arguments
        .as_ref()
        .and_then(|args| args.get("parameters"))
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        });

    let session_id = arguments
        .as_ref()
        .and_then(|args| args.get("session_id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let continue_on_error = arguments
        .as_ref()
        .and_then(|args| args.get("continue_on_error"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Load the recipe
    let recipe = match recipe_manager.load_recipe(name).await {
        Ok(recipe) => recipe,
        Err(e) => return Ok(error_response(format!("Failed to load recipe '{}': {}", name, e))),
    };

    // Create execution context
    let context = ExecutionContext {
        session_id,
        variables: std::collections::HashMap::new(),
        continue_on_error,
    };

    // Execute the recipe
    let executor = RecipeExecutor::new(server);
    match executor.execute_recipe(&recipe, parameters, context).await {
        Ok(result) => {
            if result.success {
                Ok(success_response(result.to_summary_string()))
            } else {
                Ok(error_response(result.to_detailed_string()))
            }
        }
        Err(e) => Ok(error_response(format!("Recipe execution failed: {}", e))),
    }
}

/// Delete a recipe by name
pub async fn handle_delete_recipe(
    recipe_manager: &RecipeManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let name = arguments
        .as_ref()
        .and_then(|args| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("name parameter required", None))?;

    match recipe_manager.delete_recipe(name).await {
        Ok(_) => Ok(success_response(format!("Recipe '{}' deleted successfully", name))),
        Err(e) => Ok(error_response(format!("Failed to delete recipe '{}': {}", name, e))),
    }
}

/// Create a recipe from a predefined template
pub async fn handle_create_recipe_template(
    recipe_manager: &RecipeManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let template_type = arguments
        .as_ref()
        .and_then(|args| args.get("template"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| McpError::invalid_params("template parameter required", None))?;

    // Helper function to parse browsers array
    let parse_browsers = |args: &Option<Map<String, Value>>| -> Vec<String> {
        args.as_ref()
            .and_then(|args| args.get("browsers"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .filter(|browsers: &Vec<String>| !browsers.is_empty())
            .unwrap_or_else(|| vec!["auto".to_string()])
    };

    let template = match template_type {
        "login_and_screenshot" => {
            let base_url = arguments
                .as_ref()
                .and_then(|args| args.get("base_url"))
                .and_then(|v| v.as_str())
                .unwrap_or("http://localhost:3000")
                .to_string();

            let username = arguments
                .as_ref()
                .and_then(|args| args.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("user")
                .to_string();

            let password = arguments
                .as_ref()
                .and_then(|args| args.get("password"))
                .and_then(|v| v.as_str())
                .unwrap_or("password")
                .to_string();

            let browsers = Some(parse_browsers(arguments));

            RecipeTemplate::LoginAndScreenshot {
                base_url,
                username,
                password,
                browsers,
            }
        }
        "multi_browser_screenshot" => {
            let url = arguments
                .as_ref()
                .and_then(|args| args.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("https://example.com")
                .to_string();
            let browsers = parse_browsers(arguments);
            RecipeTemplate::MultiBrowserScreenshot { url, browsers }
        }
        "responsive_test" => {
            let url = arguments
                .as_ref()
                .and_then(|args| args.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("https://example.com")
                .to_string();
            let browsers = parse_browsers(arguments);
            let resolutions = arguments
                .as_ref()
                .and_then(|args| args.get("resolutions"))
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| {
                            v.as_object().and_then(|obj| {
                                let width = obj.get("width")?.as_f64()? as u32;
                                let height = obj.get("height")?.as_f64()? as u32;
                                Some((width, height))
                            })
                        })
                        .collect()
                })
                .unwrap_or_else(|| vec![(1920, 1080), (768, 1024), (375, 667)]);
            RecipeTemplate::ResponsiveTest { url, browsers, resolutions }
        }
        _ => return Ok(error_response(format!("Unknown template type: {}", template_type))),
    };

    match recipe_manager.create_recipe_from_template(template).await {
        Ok(recipe) => {
            match recipe_manager.save_recipe(&recipe).await {
                Ok(file_path) => Ok(success_response(format!(
                    "Recipe '{}' created from template at {}",
                    recipe.name,
                    file_path.display()
                ))),
                Err(e) => Ok(error_response(format!("Failed to save recipe: {}", e))),
            }
        }
        Err(e) => Ok(error_response(format!("Failed to create recipe from template: {}", e))),
    }
}

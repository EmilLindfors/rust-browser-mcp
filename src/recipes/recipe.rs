use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;

fn default_browsers() -> Vec<String> {
    vec!["auto".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub parameters: Option<HashMap<String, ParameterDefinition>>,
    #[serde(default = "default_browsers")]
    pub browsers: Vec<String>,
    pub steps: Vec<RecipeStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDefinition {
    pub description: Option<String>,
    pub default_value: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub name: Option<String>,
    pub description: Option<String>,
    pub action: String,
    pub arguments: Map<String, Value>,
    pub continue_on_error: Option<bool>,
    pub retry_count: Option<u32>,
    pub retry_delay_ms: Option<u64>,
    pub condition: Option<String>,
    pub session_id: Option<String>,
    pub browser: Option<String>,
}

impl Recipe {
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn substitute_parameters(&self, parameters: &HashMap<String, String>) -> Result<Recipe, String> {
        let mut recipe = self.clone();
        
        // Substitute parameters in steps
        for step in &mut recipe.steps {
            // Substitute in arguments
            let arguments_str = serde_json::to_string(&step.arguments)
                .map_err(|e| format!("Failed to serialize arguments: {}", e))?;
            
            let substituted_str = substitute_variables(&arguments_str, parameters);
            
            step.arguments = serde_json::from_str(&substituted_str)
                .map_err(|e| format!("Failed to deserialize substituted arguments: {}", e))?;
            
            // Substitute in condition if present
            if let Some(condition) = &step.condition {
                step.condition = Some(substitute_variables(condition, parameters));
            }
        }
        
        Ok(recipe)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Recipe name cannot be empty".to_string());
        }

        if self.steps.is_empty() {
            return Err("Recipe must have at least one step".to_string());
        }

        // Validate browsers
        for browser in &self.browsers {
            match browser.as_str() {
                "auto" | "chrome" | "firefox" | "edge" => {},
                _ => return Err(format!("Unsupported browser: {}", browser)),
            }
        }

        // Validate each step
        for (i, step) in self.steps.iter().enumerate() {
            if step.action.is_empty() {
                return Err(format!("Step {} has empty action", i + 1));
            }

            // Validate step-level browser if specified
            if let Some(browser) = &step.browser {
                match browser.as_str() {
                    "chrome" | "firefox" | "edge" => {},
                    _ => return Err(format!("Step {} has unsupported browser: {}", i + 1, browser)),
                }
            }

            // Validate known actions
            match step.action.as_str() {
                "navigate" => {
                    if !step.arguments.contains_key("url") {
                        return Err(format!("Step {} (navigate) missing required 'url' argument", i + 1));
                    }
                }
                "click" | "wait_for_element" | "get_text" | "hover" | 
                "scroll_to_element" => {
                    if !step.arguments.contains_key("selector") {
                        return Err(format!("Step {} ({}) missing required 'selector' argument", i + 1, step.action));
                    }
                }
                "send_keys" => {
                    if !step.arguments.contains_key("selector") {
                        return Err(format!("Step {} (send_keys) missing required 'selector' argument", i + 1));
                    }
                    if !step.arguments.contains_key("text") {
                        return Err(format!("Step {} (send_keys) missing required 'text' argument", i + 1));
                    }
                }
                "execute_script" => {
                    if !step.arguments.contains_key("script") {
                        return Err(format!("Step {} (execute_script) missing required 'script' argument", i + 1));
                    }
                }
                "wait_for_condition" => {
                    if !step.arguments.contains_key("condition") {
                        return Err(format!("Step {} (wait_for_condition) missing required 'condition' argument", i + 1));
                    }
                }
                "get_attribute" => {
                    if !step.arguments.contains_key("attribute") {
                        return Err(format!("Step {} (get_attribute) missing required 'attribute' argument", i + 1));
                    }
                }
                "get_property" => {
                    if !step.arguments.contains_key("property") {
                        return Err(format!("Step {} (get_property) missing required 'property' argument", i + 1));
                    }
                }
                "fill_and_submit_form" => {
                    if !step.arguments.contains_key("fields") || !step.arguments.contains_key("submit_selector") {
                        return Err(format!("Step {} (fill_and_submit_form) missing required arguments", i + 1));
                    }
                }
                "login_form" => {
                    if !step.arguments.contains_key("username") || !step.arguments.contains_key("password") {
                        return Err(format!("Step {} (login_form) missing required 'username' or 'password' argument", i + 1));
                    }
                }
                // Allow any action - some might be custom or new
                _ => {}
            }
        }

        Ok(())
    }
}

fn substitute_variables(text: &str, parameters: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in parameters {
        let placeholder = format!("${{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_validation() {
        let mut recipe = Recipe {
            name: "Test Recipe".to_string(),
            description: Some("A test recipe".to_string()),
            version: "1.0.0".to_string(),
            author: None,
            created_at: None,
            parameters: None,
            browsers: vec!["auto".to_string()],
            steps: vec![
                RecipeStep {
                    name: Some("Navigate to page".to_string()),
                    description: None,
                    action: "navigate".to_string(),
                    arguments: {
                        let mut map = Map::new();
                        map.insert("url".to_string(), Value::String("https://example.com".to_string()));
                        map
                    },
                    continue_on_error: None,
                    retry_count: None,
                    retry_delay_ms: None,
                    condition: None,
                    session_id: None,
                    browser: None,
                }
            ],
        };

        assert!(recipe.validate().is_ok());

        // Test empty name
        recipe.name = "".to_string();
        assert!(recipe.validate().is_err());
    }

    #[test]
    fn test_parameter_substitution() {
        let recipe = Recipe {
            name: "Test Recipe".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            author: None,
            created_at: None,
            parameters: None,
            browsers: vec!["auto".to_string()],
            steps: vec![
                RecipeStep {
                    name: None,
                    description: None,
                    action: "navigate".to_string(),
                    arguments: {
                        let mut map = Map::new();
                        map.insert("url".to_string(), Value::String("${base_url}/login".to_string()));
                        map
                    },
                    continue_on_error: None,
                    retry_count: None,
                    retry_delay_ms: None,
                    condition: None,
                    session_id: None,
                    browser: None,
                }
            ],
        };

        let mut parameters = HashMap::new();
        parameters.insert("base_url".to_string(), "https://example.com".to_string());

        let substituted = recipe.substitute_parameters(&parameters).unwrap();
        let url = substituted.steps[0].arguments.get("url").unwrap().as_str().unwrap();
        assert_eq!(url, "https://example.com/login");
    }
}
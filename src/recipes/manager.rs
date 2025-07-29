use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde_json::{Map, Value};
use tokio::fs as async_fs;

use crate::recipes::recipe::{Recipe, RecipeStep, ParameterDefinition};
use crate::error::WebDriverError;

#[derive(Clone)]
pub struct RecipeManager {
    recipes_dir: PathBuf,
}

impl RecipeManager {
    pub fn new(recipes_dir: Option<PathBuf>) -> Self {
        let recipes_dir = recipes_dir.unwrap_or_else(|| {
            let mut current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            current_dir.push("recipes");
            current_dir
        });

        Self { recipes_dir }
    }

    pub async fn ensure_recipes_directory(&self) -> Result<(), WebDriverError> {
        if !self.recipes_dir.exists() {
            async_fs::create_dir_all(&self.recipes_dir).await
                .map_err(|e| WebDriverError::FileSystem(format!("Failed to create recipes directory: {}", e)))?;
        }
        Ok(())
    }

    pub async fn save_recipe(&self, recipe: &Recipe) -> Result<PathBuf, WebDriverError> {
        self.ensure_recipes_directory().await?;

        let filename = format!("{}.json", sanitize_filename(&recipe.name));
        let file_path = self.recipes_dir.join(filename);

        let recipe_json = recipe.to_json()
            .map_err(|e| WebDriverError::Serialization(format!("Failed to serialize recipe: {}", e)))?;

        async_fs::write(&file_path, recipe_json).await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to write recipe file: {}", e)))?;

        Ok(file_path)
    }

    pub async fn load_recipe(&self, name: &str) -> Result<Recipe, WebDriverError> {
        let filename = format!("{}.json", sanitize_filename(name));
        let file_path = self.recipes_dir.join(filename);

        if !file_path.exists() {
            return Err(WebDriverError::NotFound(format!("Recipe '{}' not found", name)));
        }

        let recipe_json = async_fs::read_to_string(&file_path).await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to read recipe file: {}", e)))?;

        let recipe = Recipe::from_json(&recipe_json)
            .map_err(|e| WebDriverError::Serialization(format!("Failed to parse recipe JSON: {}", e)))?;

        recipe.validate()
            .map_err(|e| WebDriverError::InvalidRecipe(format!("Recipe validation failed: {}", e)))?;

        Ok(recipe)
    }

    pub async fn list_recipes(&self) -> Result<Vec<RecipeInfo>, WebDriverError> {
        if !self.recipes_dir.exists() {
            return Ok(Vec::new());
        }

        let mut recipes = Vec::new();
        let mut entries = async_fs::read_dir(&self.recipes_dir).await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to read recipes directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to read directory entry: {}", e)))? {
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match self.load_recipe_info(&path).await {
                    Ok(info) => recipes.push(info),
                    Err(e) => {
                        tracing::warn!("Failed to load recipe info from {}: {}", path.display(), e);
                    }
                }
            }
        }

        recipes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(recipes)
    }

    pub async fn delete_recipe(&self, name: &str) -> Result<(), WebDriverError> {
        let filename = format!("{}.json", sanitize_filename(name));
        let file_path = self.recipes_dir.join(filename);

        if !file_path.exists() {
            return Err(WebDriverError::NotFound(format!("Recipe '{}' not found", name)));
        }

        async_fs::remove_file(&file_path).await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to delete recipe file: {}", e)))?;

        Ok(())
    }

    pub async fn create_recipe_from_template(&self, template: RecipeTemplate) -> Result<Recipe, WebDriverError> {
        let recipe = match template {
            RecipeTemplate::LoginAndScreenshot { base_url, username, password, browsers } => {
                Recipe {
                    name: "login_and_screenshot".to_string(),
                    description: Some("Login to a website and take a screenshot".to_string()),
                    version: "1.0.0".to_string(),
                    author: None,
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                    browsers: browsers.unwrap_or_else(|| vec!["auto".to_string()]),
                    parameters: Some({
                        let mut params = HashMap::new();
                        params.insert("base_url".to_string(), ParameterDefinition {
                            description: Some("Base URL of the website".to_string()),
                            default_value: Some(base_url),
                            required: true,
                        });
                        params.insert("username".to_string(), ParameterDefinition {
                            description: Some("Username for login".to_string()),
                            default_value: Some(username),
                            required: true,
                        });
                        params.insert("password".to_string(), ParameterDefinition {
                            description: Some("Password for login".to_string()),
                            default_value: Some(password),
                            required: true,
                        });
                        params
                    }),
                    steps: vec![
                        RecipeStep {
                            name: Some("Navigate to login page".to_string()),
                            description: Some("Navigate to the login page".to_string()),
                            action: "navigate".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("url".to_string(), Value::String("${base_url}".to_string()));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: Some(3),
                            retry_delay_ms: Some(1000),
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Wait for page to load".to_string()),
                            description: Some("Wait for the page to finish loading".to_string()),
                            action: "wait_for_condition".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("condition".to_string(), Value::String("document.readyState === 'complete'".to_string()));
                                map.insert("timeout_seconds".to_string(), Value::Number(serde_json::Number::from(10)));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: None,
                            retry_delay_ms: None,
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Login with credentials".to_string()),
                            description: Some("Fill and submit the login form".to_string()),
                            action: "login_form".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("username".to_string(), Value::String("${username}".to_string()));
                                map.insert("password".to_string(), Value::String("${password}".to_string()));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: Some(2),
                            retry_delay_ms: Some(2000),
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Wait after login".to_string()),
                            description: Some("Wait for login to complete".to_string()),
                            action: "wait_for_condition".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("condition".to_string(), Value::String("document.readyState === 'complete'".to_string()));
                                map.insert("timeout_seconds".to_string(), Value::Number(serde_json::Number::from(5)));
                                map
                            },
                            continue_on_error: Some(true),
                            retry_count: None,
                            retry_delay_ms: None,
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Take screenshot".to_string()),
                            description: Some("Take a screenshot of the current page".to_string()),
                            action: "screenshot".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("save_path".to_string(), Value::String("{{browser}}_login_screenshot.png".to_string()));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: None,
                            retry_delay_ms: None,
                            condition: None,
                            session_id: None,
                            browser: None,
                        }
                    ],
                }
            }
            RecipeTemplate::MultiBrowserScreenshot { url, browsers } => {
                Recipe {
                    name: "multi_browser_screenshot".to_string(),
                    description: Some("Take screenshots across multiple browsers for comparison".to_string()),
                    version: "1.0.0".to_string(),
                    author: None,
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                    browsers,
                    parameters: Some({
                        let mut params = HashMap::new();
                        params.insert("url".to_string(), ParameterDefinition {
                            description: Some("URL to take screenshots of".to_string()),
                            default_value: Some(url),
                            required: true,
                        });
                        params
                    }),
                    steps: vec![
                        RecipeStep {
                            name: Some("Navigate to URL".to_string()),
                            description: Some("Navigate to the target URL".to_string()),
                            action: "navigate".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("url".to_string(), Value::String("${url}".to_string()));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: Some(2),
                            retry_delay_ms: Some(1000),
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Wait for page load".to_string()),
                            description: Some("Wait for the page to fully load".to_string()),
                            action: "wait_for_condition".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("condition".to_string(), Value::String("document.readyState === 'complete'".to_string()));
                                map.insert("timeout_seconds".to_string(), Value::Number(serde_json::Number::from(10)));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: None,
                            retry_delay_ms: None,
                            condition: None,
                            session_id: None,
                            browser: None,
                        },
                        RecipeStep {
                            name: Some("Take browser-specific screenshot".to_string()),
                            description: Some("Take a screenshot with browser name in filename".to_string()),
                            action: "screenshot".to_string(),
                            arguments: {
                                let mut map = Map::new();
                                map.insert("save_path".to_string(), Value::String("{{browser}}_screenshot.png".to_string()));
                                map
                            },
                            continue_on_error: Some(false),
                            retry_count: None,
                            retry_delay_ms: None,
                            condition: None,
                            session_id: None,
                            browser: None,
                        }
                    ],
                }
            }
            RecipeTemplate::ResponsiveTest { url, browsers, resolutions } => {
                let mut steps = vec![
                    RecipeStep {
                        name: Some("Navigate to URL".to_string()),
                        description: Some("Navigate to the target URL".to_string()),
                        action: "navigate".to_string(),
                        arguments: {
                            let mut map = Map::new();
                            map.insert("url".to_string(), Value::String("${url}".to_string()));
                            map
                        },
                        continue_on_error: Some(false),
                        retry_count: Some(2),
                        retry_delay_ms: Some(1000),
                        condition: None,
                        session_id: None,
                        browser: None,
                    },
                    RecipeStep {
                        name: Some("Wait for page load".to_string()),
                        description: Some("Wait for the page to fully load".to_string()),
                        action: "wait_for_condition".to_string(),
                        arguments: {
                            let mut map = Map::new();
                            map.insert("condition".to_string(), Value::String("document.readyState === 'complete'".to_string()));
                            map.insert("timeout_seconds".to_string(), Value::Number(serde_json::Number::from(10)));
                            map
                        },
                        continue_on_error: Some(false),
                        retry_count: None,
                        retry_delay_ms: None,
                        condition: None,
                        session_id: None,
                        browser: None,
                    }
                ];

                // Add steps for each resolution
                for (width, height) in resolutions {
                    let resolution_name = format!("{}x{}", width, height);
                    steps.push(RecipeStep {
                        name: Some(format!("Set {} resolution", resolution_name)),
                        description: Some(format!("Set browser to {} resolution", resolution_name)),
                        action: "resize_window".to_string(),
                        arguments: {
                            let mut map = Map::new();
                            map.insert("width".to_string(), Value::Number(serde_json::Number::from(width)));
                            map.insert("height".to_string(), Value::Number(serde_json::Number::from(height)));
                            map
                        },
                        continue_on_error: Some(false),
                        retry_count: None,
                        retry_delay_ms: None,
                        condition: None,
                        session_id: None,
                        browser: None,
                    });
                    steps.push(RecipeStep {
                        name: Some(format!("{} screenshot", resolution_name)),
                        description: Some(format!("Take screenshot at {} resolution", resolution_name)),
                        action: "screenshot".to_string(),
                        arguments: {
                            let mut map = Map::new();
                            map.insert("save_path".to_string(), Value::String(format!("{{{{browser}}}}_{}_{}.png", width, height)));
                            map
                        },
                        continue_on_error: Some(false),
                        retry_count: None,
                        retry_delay_ms: None,
                        condition: None,
                        session_id: None,
                        browser: None,
                    });
                }

                Recipe {
                    name: "responsive_test".to_string(),
                    description: Some("Test responsive design across different browsers and screen sizes".to_string()),
                    version: "1.0.0".to_string(),
                    author: None,
                    created_at: Some(chrono::Utc::now().to_rfc3339()),
                    browsers,
                    parameters: Some({
                        let mut params = HashMap::new();
                        params.insert("url".to_string(), ParameterDefinition {
                            description: Some("URL to test for responsive design".to_string()),
                            default_value: Some(url),
                            required: true,
                        });
                        params
                    }),
                    steps,
                }
            }
        };

        recipe.validate()
            .map_err(|e| WebDriverError::InvalidRecipe(format!("Generated recipe validation failed: {}", e)))?;

        Ok(recipe)
    }

    async fn load_recipe_info(&self, path: &Path) -> Result<RecipeInfo, WebDriverError> {
        let recipe_json = async_fs::read_to_string(path).await
            .map_err(|e| WebDriverError::FileSystem(format!("Failed to read recipe file: {}", e)))?;

        let recipe: Recipe = serde_json::from_str(&recipe_json)
            .map_err(|e| WebDriverError::Serialization(format!("Failed to parse recipe JSON: {}", e)))?;

        Ok(RecipeInfo {
            name: recipe.name,
            description: recipe.description,
            version: recipe.version,
            author: recipe.author,
            created_at: recipe.created_at,
            step_count: recipe.steps.len(),
            file_path: path.to_path_buf(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct RecipeInfo {
    pub name: String,
    pub description: Option<String>,
    pub version: String,
    pub author: Option<String>,
    pub created_at: Option<String>,
    pub step_count: usize,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum RecipeTemplate {
    LoginAndScreenshot {
        base_url: String,
        username: String,
        password: String,
        browsers: Option<Vec<String>>,
    },
    MultiBrowserScreenshot {
        url: String,
        browsers: Vec<String>,
    },
    ResponsiveTest {
        url: String,
        browsers: Vec<String>,
        resolutions: Vec<(u32, u32)>,
    },
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
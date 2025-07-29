use std::collections::HashMap;
use std::time::Duration;
use serde_json::Value;
// base64 imports removed - recipe execution now handles PNG data directly

use crate::recipes::recipe::{Recipe, RecipeStep};
use crate::error::WebDriverError;
use crate::server::WebDriverServer;
// Remove unused imports

pub struct RecipeExecutor<'a> {
    server: &'a WebDriverServer,
}

#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub session_id: Option<String>,
    pub variables: HashMap<String, String>,
    pub continue_on_error: bool,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub success: bool,
    pub total_steps: usize,
    pub executed_steps: usize,
    pub failed_steps: usize,
    pub step_results: Vec<StepResult>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
    pub browser_results: HashMap<String, BrowserExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct BrowserExecutionResult {
    pub browser: String,
    pub success: bool,
    pub executed_steps: usize,
    pub failed_steps: usize,
    pub step_results: Vec<StepResult>,
    pub execution_time_ms: u64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_index: usize,
    pub step_name: Option<String>,
    pub action: String,
    pub success: bool,
    pub execution_time_ms: u64,
    pub attempts: u32,
    pub result: Option<String>,
    pub error_message: Option<String>,
    pub skipped: bool,
    pub skipped_reason: Option<String>,
    pub browser: Option<String>,
}

impl<'a> RecipeExecutor<'a> {
    pub fn new(server: &'a WebDriverServer) -> Self {
        Self { server }
    }

    fn resolve_browsers(&self, browsers: &[String]) -> Result<Vec<String>, WebDriverError> {
        let mut resolved_browsers = Vec::new();
        
        for browser in browsers {
            match browser.as_str() {
                "auto" => {
                    // Auto-selection priority: Chrome → Firefox → Edge
                    if self.is_browser_available("chrome") {
                        resolved_browsers.push("chrome".to_string());
                    } else if self.is_browser_available("firefox") {
                        resolved_browsers.push("firefox".to_string());
                    } else if self.is_browser_available("edge") {
                        resolved_browsers.push("edge".to_string());
                    } else {
                        return Err(WebDriverError::Execution("No supported browsers available".to_string()));
                    }
                }
                "chrome" | "firefox" | "edge" => {
                    if self.is_browser_available(browser) {
                        resolved_browsers.push(browser.clone());
                    } else {
                        return Err(WebDriverError::Execution(format!("Browser {} is not available", browser)));
                    }
                }
                _ => return Err(WebDriverError::Execution(format!("Unsupported browser: {}", browser))),
            }
        }
        
        Ok(resolved_browsers)
    }

    fn is_browser_available(&self, _browser: &str) -> bool {
        // For now, assume Chrome is always available
        // In a real implementation, this would check system availability
        true
    }

    pub async fn execute_recipe(
        &self,
        recipe: &Recipe,
        parameters: Option<HashMap<String, String>>,
        context: ExecutionContext,
    ) -> Result<ExecutionResult, WebDriverError> {
        let start_time = std::time::Instant::now();
        
        // Substitute parameters if provided
        let final_recipe = if let Some(params) = parameters {
            recipe.substitute_parameters(&params)
                .map_err(|e| WebDriverError::InvalidRecipe(format!("Parameter substitution failed: {}", e)))?
        } else {
            recipe.clone()
        };

        // Validate the final recipe
        final_recipe.validate()
            .map_err(|e| WebDriverError::InvalidRecipe(format!("Recipe validation failed: {}", e)))?;

        // Resolve browsers for execution
        let browsers = self.resolve_browsers(&final_recipe.browsers)?;
        
        // CRITICAL FIX: Refresh driver health before recipe execution
        // This ensures that running drivers are properly registered in healthy_endpoints
        tracing::debug!("🔄 Refreshing driver health before recipe execution");
        let driver_manager = self.server.get_client_manager().get_driver_manager();
        if let Err(e) = driver_manager.refresh_driver_health().await {
            tracing::warn!("Failed to refresh driver health: {}", e);
        } else {
            let healthy_count = driver_manager.get_healthy_endpoints().len();
            tracing::info!("✅ Health check completed: {} healthy endpoints found", healthy_count);
        }
        
        let mut browser_results = HashMap::new();
        let mut all_step_results = Vec::new();
        let mut total_executed_steps = 0;
        let mut total_failed_steps = 0;
        let mut overall_success = true;
        let mut global_error_message = None;

        // Execute recipe for each browser sequentially
        for browser in browsers {
            tracing::info!("🌐 Executing recipe for browser: {}", browser);
            
            let browser_result = self.execute_recipe_for_browser(
                &final_recipe, 
                &browser, 
                &context
            ).await;

            match browser_result {
                Ok(result) => {
                    total_executed_steps += result.executed_steps;
                    total_failed_steps += result.failed_steps;
                    
                    if !result.success {
                        overall_success = false;
                        if global_error_message.is_none() {
                            global_error_message = result.error_message.clone();
                        }
                    }

                    // Add browser info to step results
                    let mut browser_step_results = result.step_results.clone();
                    for step_result in &mut browser_step_results {
                        step_result.browser = Some(browser.clone());
                    }
                    all_step_results.extend(browser_step_results);

                    browser_results.insert(browser.clone(), BrowserExecutionResult {
                        browser: browser.clone(),
                        success: result.success,
                        executed_steps: result.executed_steps,
                        failed_steps: result.failed_steps,
                        step_results: result.step_results,
                        execution_time_ms: result.execution_time_ms,
                        error_message: result.error_message,
                    });
                }
                Err(e) => {
                    overall_success = false;
                    let error_msg = format!("Browser {} failed: {}", browser, e);
                    if global_error_message.is_none() {
                        global_error_message = Some(error_msg.clone());
                    }
                    
                    browser_results.insert(browser.clone(), BrowserExecutionResult {
                        browser: browser.clone(),
                        success: false,
                        executed_steps: 0,
                        failed_steps: final_recipe.steps.len(),
                        step_results: Vec::new(),
                        execution_time_ms: 0,
                        error_message: Some(error_msg),
                    });
                }
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;

        Ok(ExecutionResult {
            success: overall_success,
            total_steps: final_recipe.steps.len(),
            executed_steps: total_executed_steps,
            failed_steps: total_failed_steps,
            step_results: all_step_results,
            execution_time_ms: total_time,
            error_message: global_error_message,
            browser_results,
        })
    }

    async fn execute_recipe_for_browser(
        &self,
        recipe: &Recipe,
        browser: &str,
        context: &ExecutionContext,
    ) -> Result<ExecutionResult, WebDriverError> {
        let start_time = std::time::Instant::now();
        
        // Create browser-specific session
        let browser_session_id = format!("{}_recipe_session", browser);
        let mut browser_context = context.clone();
        browser_context.session_id = Some(browser_session_id);

        let mut step_results = Vec::new();
        let mut executed_steps = 0;
        let mut failed_steps = 0;
        let mut execution_failed = false;
        let mut error_message = None;

        // Execute each step
        for (index, step) in recipe.steps.iter().enumerate() {
            let step_start_time = std::time::Instant::now();
            
            // Determine which browser to use for this step
            let step_browser = step.browser.as_ref().map_or(browser, |s| s.as_str());
            if step_browser != browser {
                // Skip this step if it's for a different browser
                step_results.push(StepResult {
                    step_index: index,
                    step_name: step.name.clone(),
                    action: step.action.clone(),
                    success: true,
                    execution_time_ms: 0,
                    attempts: 0,
                    result: None,
                    error_message: None,
                    skipped: true,
                    skipped_reason: Some(format!("Step for different browser: {}", step_browser)),
                    browser: Some(browser.to_string()),
                });
                continue;
            }
            
            // Check if step should be skipped based on condition
            if let Some(condition) = &step.condition {
                match self.evaluate_condition(condition, &browser_context).await {
                    Ok(should_execute) => {
                        if !should_execute {
                            step_results.push(StepResult {
                                step_index: index,
                                step_name: step.name.clone(),
                                action: step.action.clone(),
                                success: true,
                                execution_time_ms: 0,
                                attempts: 0,
                                result: None,
                                error_message: None,
                                skipped: true,
                                skipped_reason: Some(format!("Condition not met: {}", condition)),
                                browser: Some(browser.to_string()),
                            });
                            continue;
                        }
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to evaluate condition '{}': {}", condition, e);
                        step_results.push(StepResult {
                            step_index: index,
                            step_name: step.name.clone(),
                            action: step.action.clone(),
                            success: false,
                            execution_time_ms: step_start_time.elapsed().as_millis() as u64,
                            attempts: 1,
                            result: None,
                            error_message: Some(error_msg.clone()),
                            skipped: false,
                            skipped_reason: None,
                            browser: Some(browser.to_string()),
                        });
                        
                        if !step.continue_on_error.unwrap_or(false) && !context.continue_on_error {
                            execution_failed = true;
                            error_message = Some(error_msg);
                            break;
                        }
                        failed_steps += 1;
                        continue;
                    }
                }
            }

            // Execute the step with retries
            let step_result = self.execute_step_with_retries(step, &browser_context, index).await;
            let step_duration = step_start_time.elapsed().as_millis() as u64;
            
            executed_steps += 1;
            
            if step_result.success {
                step_results.push(StepResult {
                    step_index: index,
                    step_name: step.name.clone(),
                    action: step.action.clone(),
                    success: true,
                    execution_time_ms: step_duration,
                    attempts: step_result.attempts,
                    result: step_result.result,
                    error_message: None,
                    skipped: false,
                    skipped_reason: None,
                    browser: Some(browser.to_string()),
                });
            } else {
                failed_steps += 1;
                step_results.push(StepResult {
                    step_index: index,
                    step_name: step.name.clone(),
                    action: step.action.clone(),
                    success: false,
                    execution_time_ms: step_duration,
                    attempts: step_result.attempts,
                    result: None,
                    error_message: step_result.error_message.clone(),
                    skipped: false,
                    skipped_reason: None,
                    browser: Some(browser.to_string()),
                });

                // Check if we should continue after this failure
                if !step.continue_on_error.unwrap_or(false) && !context.continue_on_error {
                    execution_failed = true;
                    error_message = step_result.error_message;
                    break;
                }
            }
        }

        let total_time = start_time.elapsed().as_millis() as u64;
        let overall_success = !execution_failed && failed_steps == 0;

        Ok(ExecutionResult {
            success: overall_success,
            total_steps: recipe.steps.len(),
            executed_steps,
            failed_steps,
            step_results,
            execution_time_ms: total_time,
            error_message,
            browser_results: HashMap::new(), // Empty for single browser execution
        })
    }

    async fn execute_step_with_retries(
        &self,
        step: &RecipeStep,
        context: &ExecutionContext,
        step_index: usize,
    ) -> StepExecutionResult {
        let max_retries = step.retry_count.unwrap_or(0);
        let retry_delay = Duration::from_millis(step.retry_delay_ms.unwrap_or(1000));
        
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            if attempt > 0 {
                tracing::debug!("Retrying step {} (attempt {}/{})", step_index + 1, attempt + 1, max_retries + 1);
                tokio::time::sleep(retry_delay).await;
            }

            match self.execute_single_step(step, context).await {
                Ok(result) => {
                    return StepExecutionResult {
                        success: true,
                        attempts: attempt + 1,
                        result: Some(result),
                        error_message: None,
                    };
                }
                Err(e) => {
                    last_error = Some(e);
                    tracing::warn!("Step {} failed on attempt {}: {}", step_index + 1, attempt + 1, last_error.as_ref().unwrap());
                }
            }
        }

        StepExecutionResult {
            success: false,
            attempts: max_retries + 1,
            result: None,
            error_message: last_error.map(|e| e.to_string()),
        }
    }

    async fn execute_single_step(
        &self,
        step: &RecipeStep,
        context: &ExecutionContext,
    ) -> Result<String, WebDriverError> {
        tracing::debug!("🔍 Executing step: {} with action: {}", 
            step.name.as_deref().unwrap_or("unnamed"), step.action);
        
        // Prepare arguments with session_id override if needed
        let mut arguments = step.arguments.clone();
        
        // Use step-level session_id, then context session_id, then default
        let session_id = step.session_id.as_ref()
            .or(context.session_id.as_ref())
            .cloned();
            
        if let Some(sid) = &session_id {
            arguments.insert("session_id".to_string(), Value::String(sid.clone()));
        }

        // Substitute browser placeholder in arguments
        if let Some(session_id) = &context.session_id {
            if let Some(browser) = session_id.strip_suffix("_recipe_session") {
                // Replace {{browser}} placeholder in arguments
                let args_str = serde_json::to_string(&arguments)
                    .map_err(|e| WebDriverError::Execution(format!("Failed to serialize arguments: {}", e)))?;
                let substituted_str = args_str.replace("{{browser}}", browser);
                arguments = serde_json::from_str(&substituted_str)
                    .map_err(|e| WebDriverError::Execution(format!("Failed to deserialize substituted arguments: {}", e)))?;
            }
        }

        tracing::debug!("📋 Step arguments: {:?}", arguments);
        tracing::debug!("🆔 Using session_id: {:?}", session_id);

        // Execute the actual WebDriver tool based on the action
        let result = match step.action.as_str() {
            "navigate" => self.execute_navigate(&arguments).await,
            "click" => self.execute_click(&arguments).await,
            "send_keys" => self.execute_send_keys(&arguments).await,
            "screenshot" => self.execute_screenshot(&arguments).await,
            "get_title" => self.execute_get_title(&arguments).await,
            "get_text" => self.execute_get_text(&arguments).await,
            "wait_for_element" => self.execute_wait_for_element(&arguments).await,
            "wait_for_condition" => self.execute_wait_for_condition(&arguments).await,
            "login_form" => self.execute_login_form(&arguments).await,
            "back" => self.execute_back(&arguments).await,
            "forward" => self.execute_forward(&arguments).await,
            "refresh" => self.execute_refresh(&arguments).await,
            "execute_script" => self.execute_script(&arguments).await,
            "resize_window" => self.execute_resize_window(&arguments).await,
            "get_current_url" => self.execute_get_current_url(&arguments).await,
            "find_element" => self.execute_find_element(&arguments).await,
            "hover" => self.execute_hover(&arguments).await,
            "scroll_to_element" => self.execute_scroll_to_element(&arguments).await,
            "get_attribute" => self.execute_get_attribute(&arguments).await,
            "get_property" => self.execute_get_property(&arguments).await,
            "fill_and_submit_form" => self.execute_fill_and_submit_form(&arguments).await,
            _ => Err(WebDriverError::Execution(format!("Unknown action: {}", step.action))),
        };

        match &result {
            Ok(success_msg) => {
                tracing::debug!("✅ Step completed successfully: {}", success_msg);
            }
            Err(error) => {
                tracing::error!("❌ Step failed with error: {}", error);
            }
        }

        result
    }

    async fn evaluate_condition(
        &self,
        _condition: &str,
        _context: &ExecutionContext,
    ) -> Result<bool, WebDriverError> {
        // For now, implement basic condition evaluation
        // In a full implementation, this could be extended to support:
        // - JavaScript evaluation in the browser
        // - Variable comparisons
        // - Complex logical expressions
        
        // Simple implementation: treat empty or "true" as true
        Ok(_condition.is_empty() || _condition.trim().to_lowercase() == "true")
    }
}

#[derive(Debug)]
struct StepExecutionResult {
    success: bool,
    attempts: u32,
    result: Option<String>,
    error_message: Option<String>,
}

impl ExecutionResult {
    pub fn to_summary_string(&self) -> String {
        if self.success {
            format!(
                "Recipe executed successfully! {} steps completed in {}ms",
                self.executed_steps,
                self.execution_time_ms
            )
        } else {
            format!(
                "Recipe execution failed. {}/{} steps completed, {} failures in {}ms. Error: {}",
                self.executed_steps,
                self.total_steps,
                self.failed_steps,
                self.execution_time_ms,
                self.error_message.as_deref().unwrap_or("Unknown error")
            )
        }
    }

    pub fn to_detailed_string(&self) -> String {
        let mut result = self.to_summary_string();
        result.push_str("\n\nStep Results:\n");
        
        for step_result in &self.step_results {
            let status = if step_result.skipped {
                "SKIPPED"
            } else if step_result.success {
                "SUCCESS"
            } else {
                "FAILED"
            };
            
            let default_name = format!("Step {}", step_result.step_index + 1);
            let step_name = step_result.step_name.as_deref()
                .unwrap_or(&default_name);
            
            result.push_str(&format!(
                "  {} - {} ({}): {}ms",
                step_result.step_index + 1,
                step_name,
                status,
                step_result.execution_time_ms
            ));
            
            if let Some(error) = &step_result.error_message {
                result.push_str(&format!(" - Error: {}", error));
            }
            
            if let Some(reason) = &step_result.skipped_reason {
                result.push_str(&format!(" - {}", reason));
            }
            
            result.push('\n');
        }
        
        result
    }
}

impl<'a> RecipeExecutor<'a> {
    // Individual tool execution methods
    async fn execute_navigate(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        let url = arguments.get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WebDriverError::Execution("Missing 'url' parameter for navigate".to_string()))?;
        
        let session_id = arguments.get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        tracing::debug!("🌐 Navigating to URL: {} with session_id: {}", url, session_id);

        let client_manager = self.server.get_client_manager();
        tracing::debug!("📞 Getting WebDriver client from client manager");
        
        let (_session, client) = client_manager.get_or_create_client(Some(session_id.to_string())).await
            .map_err(|e| {
                tracing::error!("❌ Failed to get WebDriver client for navigation session '{}': {}", session_id, e);
                WebDriverError::Execution(format!("Failed to get client for navigation session '{}': {}", session_id, e))
            })?;

        tracing::debug!("🚀 Calling client.goto() with URL: {}", url);
        client.goto(url).await
            .map_err(|e| {
                tracing::error!("❌ Navigation to '{}' failed for session '{}': {}", url, session_id, e);
                WebDriverError::Execution(format!("Navigation to '{}' failed for session '{}': {}", url, session_id, e))
            })?;

        tracing::debug!("✅ Navigation completed successfully");
        Ok(format!("Successfully navigated to {}", url))
    }

    async fn execute_screenshot(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        let save_path = arguments.get("save_path")
            .and_then(|v| v.as_str());
            
        let session_id = arguments.get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        tracing::debug!("📸 Taking screenshot with session_id: {}, save_path: {:?}", session_id, save_path);

        let client_manager = self.server.get_client_manager();
        tracing::debug!("📞 Getting WebDriver client from client manager");
        
        let (_session, client) = client_manager.get_or_create_client(Some(session_id.to_string())).await
            .map_err(|e| {
                tracing::error!("❌ Failed to get WebDriver client for screenshot session '{}': {}", session_id, e);
                WebDriverError::Execution(format!("Failed to get client for screenshot session '{}': {}", session_id, e))
            })?;

        tracing::debug!("📷 Calling client.screenshot()");
        let screenshot_data = client.screenshot().await
            .map_err(|e| {
                tracing::error!("❌ Screenshot capture failed for session '{}': {}", session_id, e);
                WebDriverError::Execution(format!("Screenshot capture failed for session '{}': {}", session_id, e))
            })?;

        tracing::debug!("📊 Screenshot data received: {} bytes", screenshot_data.len());

        if let Some(path) = save_path {
            // screenshot_data is already PNG binary data, no need to decode from base64
            tracing::debug!("💾 Saving screenshot to: {}", path);
            std::fs::write(path, &screenshot_data)
                .map_err(|e| WebDriverError::Execution(format!("Failed to save screenshot: {}", e)))?;
            
            Ok(format!("Screenshot saved to: {} ({} bytes)", path, screenshot_data.len()))
        } else {
            Ok(format!("Screenshot captured ({} bytes)", screenshot_data.len()))
        }
    }

    async fn execute_wait_for_condition(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        let condition = arguments.get("condition")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WebDriverError::Execution("Missing 'condition' parameter".to_string()))?;
            
        let timeout_seconds = arguments.get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(10);
            
        let session_id = arguments.get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let client_manager = self.server.get_client_manager();
        let (_session, client) = client_manager.get_or_create_client(Some(session_id.to_string())).await
            .map_err(|e| WebDriverError::Execution(format!("Failed to get client: {}", e)))?;

        // For document.readyState === 'complete', use a direct approach
        if condition == "document.readyState === 'complete'" {
            let start_time = std::time::Instant::now();
            let timeout_duration = Duration::from_secs(timeout_seconds);
            
            loop {
                if start_time.elapsed() > timeout_duration {
                    return Err(WebDriverError::Execution("Timeout waiting for page to load".to_string()));
                }
                
                let ready_state = client.execute("return document.readyState", vec![]).await
                    .map_err(|e| WebDriverError::Execution(format!("Failed to check readyState: {}", e)))?;
                
                if let Some(state) = ready_state.as_str() {
                    if state == "complete" {
                        return Ok("Page loaded successfully".to_string());
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        } else if condition == "true" {
            // Simple delay for "true" condition
            tokio::time::sleep(Duration::from_secs(timeout_seconds)).await;
            Ok(format!("Waited {} seconds", timeout_seconds))
        } else {
            // For other conditions, try to evaluate the JavaScript
            let result = client.execute(condition, vec![]).await
                .map_err(|e| WebDriverError::Execution(format!("Failed to evaluate condition: {}", e)))?;
            
            Ok(format!("Condition '{}' evaluated to: {:?}", condition, result))
        }
    }

    async fn execute_login_form(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        let username = arguments.get("username")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WebDriverError::Execution("Missing 'username' parameter".to_string()))?;
            
        let password = arguments.get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| WebDriverError::Execution("Missing 'password' parameter".to_string()))?;
            
        let session_id = arguments.get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        let client_manager = self.server.get_client_manager();
        let (_session, client) = client_manager.get_or_create_client(Some(session_id.to_string())).await
            .map_err(|e| WebDriverError::Execution(format!("Failed to get client: {}", e)))?;

        // Try common username field selectors
        let username_selectors = vec![
            "input[type='email']",
            "input[name='email']", 
            "input[name='username']",
            "input[name='user']",
            "#email",
            "#username",
            "#user"
        ];

        let mut username_element = None;
        for selector in username_selectors {
            if let Ok(element) = client.find(fantoccini::Locator::Css(selector)).await {
                username_element = Some(element);
                break;
            }
        }

        let username_el = username_element
            .ok_or_else(|| WebDriverError::Execution("Could not find username/email field".to_string()))?;

        // Try common password field selectors
        let password_selectors = vec![
            "input[type='password']",
            "input[name='password']",
            "#password"
        ];

        let mut password_element = None;
        for selector in password_selectors {
            if let Ok(element) = client.find(fantoccini::Locator::Css(selector)).await {
                password_element = Some(element);
                break;
            }
        }

        let password_el = password_element
            .ok_or_else(|| WebDriverError::Execution("Could not find password field".to_string()))?;

        // Fill in the form
        username_el.clear().await
            .map_err(|e| WebDriverError::Execution(format!("Failed to clear username field: {}", e)))?;
        username_el.send_keys(username).await
            .map_err(|e| WebDriverError::Execution(format!("Failed to enter username: {}", e)))?;

        password_el.clear().await
            .map_err(|e| WebDriverError::Execution(format!("Failed to clear password field: {}", e)))?;
        password_el.send_keys(password).await
            .map_err(|e| WebDriverError::Execution(format!("Failed to enter password: {}", e)))?;

        // Try to find and click submit button
        let submit_selectors = vec![
            "button[type='submit']",
            "input[type='submit']",
            "button:contains('Sign in')",
            "button:contains('Login')",
            "button:contains('Log in')",
            ".btn-primary",
            ".submit-btn"
        ];

        let mut submitted = false;
        for selector in submit_selectors {
            if let Ok(submit_btn) = client.find(fantoccini::Locator::Css(selector)).await {
                submit_btn.click().await
                    .map_err(|e| WebDriverError::Execution(format!("Failed to click submit: {}", e)))?;
                submitted = true;
                break;
            }
        }

        if !submitted {
            // Try pressing Enter on the password field as fallback
            password_el.send_keys("\n").await
                .map_err(|e| WebDriverError::Execution(format!("Failed to submit form with Enter: {}", e)))?;
        }

        Ok("Login form submitted successfully".to_string())
    }

    // Placeholder implementations for other tools
    async fn execute_click(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Click executed (placeholder)".to_string())
    }

    async fn execute_send_keys(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Send keys executed (placeholder)".to_string())
    }

    async fn execute_get_title(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Get title executed (placeholder)".to_string())
    }

    async fn execute_get_text(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Get text executed (placeholder)".to_string())
    }

    async fn execute_wait_for_element(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Wait for element executed (placeholder)".to_string())
    }

    async fn execute_back(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Back executed (placeholder)".to_string())
    }

    async fn execute_forward(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Forward executed (placeholder)".to_string())
    }

    async fn execute_refresh(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Refresh executed (placeholder)".to_string())
    }

    async fn execute_script(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Execute script executed (placeholder)".to_string())
    }

    async fn execute_resize_window(&self, arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        let width = arguments
            .get("width")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| WebDriverError::Execution("width parameter required".to_string()))?;
            
        let height = arguments
            .get("height")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| WebDriverError::Execution("height parameter required".to_string()))?;

        // Validate dimensions
        if width <= 0.0 || height <= 0.0 {
            return Err(WebDriverError::Execution("Width and height must be positive numbers".to_string()));
        }
        
        if width > 10000.0 || height > 10000.0 {
            return Err(WebDriverError::Execution("Width and height must be less than 10000 pixels".to_string()));
        }

        // Get the client from the server's client manager
        match self.server.get_client_manager().get_or_create_client(None).await {
            Ok((_session, client)) => {
                match client.set_window_size(width as u32, height as u32).await {
                    Ok(_) => {
                        // Verify the resize by getting the current size
                        match client.get_window_size().await {
                            Ok((actual_width, actual_height)) => Ok(format!(
                                "Window resized to {}x{} pixels", 
                                actual_width, actual_height
                            )),
                            Err(_) => Ok(format!(
                                "Window resize command sent ({}x{})",
                                width, height
                            )),
                        }
                    }
                    Err(e) => Err(WebDriverError::Execution(format!("Failed to resize window: {}", e))),
                }
            }
            Err(e) => Err(WebDriverError::Execution(format!("Failed to get client: {}", e))),
        }
    }

    async fn execute_get_current_url(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Get current URL executed (placeholder)".to_string())
    }

    async fn execute_find_element(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Find element executed (placeholder)".to_string())
    }

    async fn execute_hover(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Hover executed (placeholder)".to_string())
    }

    async fn execute_scroll_to_element(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Scroll to element executed (placeholder)".to_string())
    }

    async fn execute_get_attribute(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Get attribute executed (placeholder)".to_string())
    }

    async fn execute_get_property(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Get property executed (placeholder)".to_string())
    }

    async fn execute_fill_and_submit_form(&self, _arguments: &serde_json::Map<String, Value>) -> Result<String, WebDriverError> {
        Ok("Fill and submit form executed (placeholder)".to_string())
    }
}
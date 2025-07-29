# WebDriver MCP Recipe System

The WebDriver MCP server now includes a powerful recipe system that allows you to define, save, and execute complex browser automation workflows using JSON configuration files.

## What are Recipes?

Recipes are JSON files that define a series of browser automation steps that can be executed as a single unit. They support:

- **Parameterization**: Use variables that can be substituted at runtime
- **Error Handling**: Configure retry logic and error continuation behavior
- **Step Dependencies**: Steps execute in sequence with optional conditions
- **Comprehensive Actions**: Support for all WebDriver MCP tools (navigate, click, fill forms, screenshots, etc.)

## Recipe Structure

```json
{
  "name": "recipe_name",
  "description": "Description of what this recipe does",
  "version": "1.0.0",
  "author": "Recipe Author",
  "created_at": "2025-01-25T12:00:00Z",
  "parameters": {
    "param_name": {
      "description": "Parameter description",
      "default_value": "default_value",
      "required": true
    }
  },
  "steps": [
    {
      "name": "Step name",
      "description": "Step description", 
      "action": "navigate",
      "arguments": {
        "url": "${param_name}"
      },
      "continue_on_error": false,
      "retry_count": 3,
      "retry_delay_ms": 1000,
      "condition": "optional_condition"
    }
  ]
}
```

## Available MCP Tools for Recipe Management

### 1. `create_recipe`
Create a new recipe from JSON.

**Parameters:**
- `recipe_json` (required): JSON string containing the recipe definition

**Example:**
```json
{
  "name": "create_recipe",
  "arguments": {
    "recipe_json": "{\"name\":\"test\",\"version\":\"1.0.0\",\"steps\":[...]}"
  }
}
```

### 2. `list_recipes`
List all available recipes.

**Parameters:** None

### 3. `get_recipe`
Get the JSON definition of a specific recipe.

**Parameters:**
- `name` (required): Name of the recipe to retrieve

### 4. `execute_recipe`
Execute a recipe with optional parameters.

**Parameters:**
- `name` (required): Name of the recipe to execute
- `parameters` (optional): Object with parameter values to substitute
- `session_id` (optional): Browser session ID to use
- `continue_on_error` (optional): Whether to continue on step failures

**Example:**
```json
{
  "name": "execute_recipe",
  "arguments": {
    "name": "example_navigation_and_screenshot",
    "parameters": {
      "url": "https://github.com",
      "screenshot_path": "./github_screenshot.png"
    },
    "session_id": "my_session"
  }
}
```

### 5. `delete_recipe`
Delete a recipe.

**Parameters:**
- `name` (required): Name of the recipe to delete

### 6. `create_recipe_template`
Create a recipe from a predefined template.

**Parameters:**
- `template` (required): Template type (currently supports "login_and_screenshot")
- `base_url` (optional): Base URL for the template
- `username` (optional): Username for login templates  
- `password` (optional): Password for login templates

## Supported Actions

Recipes can use any of the existing WebDriver MCP tools as actions:

- **Navigation**: `navigate`, `back`, `forward`, `refresh`
- **Element Interaction**: `click`, `send_keys`, `hover`, `scroll_to_element`
- **Element Finding**: `find_element`, `find_elements`, `wait_for_element`
- **Information Retrieval**: `get_text`, `get_title`, `get_current_url`, `get_attribute`, `get_property`
- **Form Handling**: `fill_and_submit_form`, `login_form`
- **Screenshots**: `screenshot`
- **JavaScript**: `execute_script`, `wait_for_condition`
- **Performance**: `get_performance_metrics`, `monitor_memory_usage`
- **And more...**

## Parameter Substitution

Use `${parameter_name}` syntax in any string value within step arguments to substitute parameters at runtime.

Example:
```json
{
  "arguments": {
    "url": "${base_url}/login",
    "username": "${user_credentials}"
  }
}
```

## Error Handling

Each step can specify:
- `continue_on_error`: Whether to continue if this step fails
- `retry_count`: Number of times to retry the step on failure
- `retry_delay_ms`: Delay between retry attempts
- `condition`: Optional condition that must be true for the step to execute

## File Storage

Recipes are stored as JSON files in the `recipes/` directory (created automatically). Recipe files are named using a sanitized version of the recipe name with `.json` extension.

## Example Usage Workflow

1. **Create a template recipe:**
   ```json
   {
     "name": "create_recipe_template",
     "arguments": {
       "template": "login_and_screenshot",
       "base_url": "http://localhost:3000",
       "username": "testuser", 
       "password": "testpass"
     }
   }
   ```

2. **List available recipes:**
   ```json
   {
     "name": "list_recipes",
     "arguments": {}
   }
   ```

3. **Execute the recipe:**
   ```json
   {
     "name": "execute_recipe",
     "arguments": {
       "name": "login_and_screenshot",
       "parameters": {
         "base_url": "http://localhost:3000"
       }
     }
   }
   ```

This recipe system makes it easy to create reusable browser automation workflows that can be shared, version controlled, and executed with different parameters as needed.
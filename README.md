# Rust Browser MCP

Browser automation for Claude via the Model Context Protocol (MCP).

## Quick Start

1. **Install WebDriver**:
   ```bash
   # Chrome (recommended)
   brew install chromedriver              # macOS
   sudo apt install chromium-chromedriver # Ubuntu
   choco install chromedriver            # Windows
   
   # Firefox
   brew install geckodriver               # macOS
   sudo apt install firefox-geckodriver   # Ubuntu
   choco install geckodriver             # Windows
   ```

2. **Build the server**:
   ```bash
   git clone <repository-url>
   cd rust-browser-mcp
   cargo build --release
   ```

3. **Configure Claude Desktop** (`claude_desktop_config.json`):
   ```json
   {
     "mcpServers": {
       "webdriver": {
         "command": "/path/to/rust-browser-mcp",
         "args": ["--transport", "stdio"],
         "env": {
           "WEBDRIVER_PREFERRED_DRIVER": "chrome",
           "WEBDRIVER_HEADLESS": "true"
         }
       }
     }
   }
   ```

4. **Use with Claude**:
   - "Take a screenshot of example.com"
   - "Fill out the contact form on this website"
   - "Click the submit button"

## Available Tools

- **Navigation**: `navigate`, `back`, `forward`, `refresh`
- **Element Interaction**: `click`, `send_keys`, `hover`, `scroll_to_element`
- **Information**: `find_element`, `get_text`, `get_attribute`, `screenshot`
- **Forms**: `fill_and_submit_form`
- **Advanced**: `execute_script`

## Configuration

Environment variables:
```bash
export WEBDRIVER_PREFERRED_DRIVER="chrome"  # chrome, firefox, edge
export WEBDRIVER_HEADLESS="true"            # run browser headless
```

For Firefox specifically:
```json
{
  "mcpServers": {
    "webdriver": {
      "command": "/path/to/webdriver-mcp",
      "args": ["--transport", "stdio"],
      "env": {
        "WEBDRIVER_PREFERRED_DRIVER": "firefox",
        "WEBDRIVER_HEADLESS": "true"
      }
    }
  }
}
```

## HTTP Mode (Optional)

For remote access:
```bash
cargo run -- --transport http --bind 0.0.0.0:8080
```
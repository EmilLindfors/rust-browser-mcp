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

2. **Download or build the server**:
   
   **Option A: Download pre-built binary**
   ```bash
   # Download for your platform from:
   # https://github.com/EmilLindfors/rust-browser-mcp/releases/latest
   
   # Linux
   wget https://github.com/EmilLindfors/rust-browser-mcp/releases/latest/download/rust-browser-mcp-x86_64-unknown-linux-gnu.tar.gz
   tar xzf rust-browser-mcp-x86_64-unknown-linux-gnu.tar.gz
   
   # macOS (Intel)
   wget https://github.com/EmilLindfors/rust-browser-mcp/releases/latest/download/rust-browser-mcp-x86_64-apple-darwin.tar.gz
   tar xzf rust-browser-mcp-x86_64-apple-darwin.tar.gz
   
   # macOS (Apple Silicon)
   wget https://github.com/EmilLindfors/rust-browser-mcp/releases/latest/download/rust-browser-mcp-aarch64-apple-darwin.tar.gz
   tar xzf rust-browser-mcp-aarch64-apple-darwin.tar.gz
   
   # Windows: Download rust-browser-mcp-x86_64-pc-windows-msvc.zip and extract
   ```
   
   **Option B: Build from source**
   ```bash
   git clone https://github.com/EmilLindfors/rust-browser-mcp.git
   cd rust-browser-mcp
   cargo build --release
   ```

3. **Configure Claude Desktop** (`claude_desktop_config.json`):
   ```json
   {
     "mcpServers": {
       "webdriver": {
         "command": "/path/to/rust-browser-mcp",
         "args": ["--browser", "chrome", "--transport", "stdio"],
         "env": {
           "WEBDRIVER_HEADLESS": "true"
         }
       }
     }
   }
   ```
   
   **Or for Claude Code**:
   ```bash
   claude mcp add webdriver -e WEBDRIVER_HEADLESS=true -- /path/to/rust-browser-mcp --browser firefox --transport stdio
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

### Command Line Options

```bash
# Specify browser driver
rust-browser-mcp --browser firefox --transport stdio
rust-browser-mcp --browser chrome --transport stdio
rust-browser-mcp --browser edge --transport stdio

# HTTP mode
rust-browser-mcp --transport http --bind 0.0.0.0:8080 --browser firefox
```

Options:
- `--browser, -b`: Browser driver to use (chrome, firefox, edge) - defaults to chrome
- `--transport, -t`: Server transport mode (stdio, http) - defaults to stdio
- `--bind`: HTTP server bind address (only used with --transport=http) - defaults to 127.0.0.1:8080
- `--no-auth`: Disable OAuth authentication for HTTP server

### Environment Variables

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
# Default Chrome
cargo run -- --transport http --bind 0.0.0.0:8080

# With Firefox
cargo run -- --browser firefox --transport http --bind 0.0.0.0:8080
```
# WebDriver MCP Server

Professional browser automation for Claude and Claude Code via the Model Context Protocol (MCP). Features enterprise-grade performance monitoring, multi-session support, and zero-latency driver management.

## 🚀 Quick Start with Claude

### Prerequisites

Install WebDriver for your preferred browser:

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

### Installation

**Option A: Download Pre-built Binary**
```bash
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

**Option B: Build from Source**
```bash
git clone https://github.com/EmilLindfors/rust-browser-mcp.git
cd rust-browser-mcp
cargo build --release
# Binary will be at target/release/rust-browser-mcp
```

## 🎯 Using with Claude Desktop

### Stdio Transport (Recommended)

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "webdriver": {
      "command": "/path/to/rust-browser-mcp",
      "args": ["--transport", "stdio", "--browser", "chrome"],
      "env": {
        "WEBDRIVER_HEADLESS": "true"
      }
    }
  }
}
```

**Configuration Options:**
```json
{
  "mcpServers": {
    "webdriver": {
      "command": "/path/to/rust-browser-mcp", 
      "args": ["--transport", "stdio", "--browser", "firefox"],
      "env": {
        "WEBDRIVER_HEADLESS": "false",
        "WEBDRIVER_CONCURRENT_DRIVERS": "firefox,chrome",
        "WEBDRIVER_ENABLE_PERFORMANCE_MEMORY": "true"
      }
    }
  }
}
```

### HTTP Transport (Remote Access)

1. **Start the HTTP server:**
```bash
./rust-browser-mcp --transport http --bind 0.0.0.0:8080 --no-auth
```

2. **Configure Claude Desktop:**
```json
{
  "mcpServers": {
    "webdriver": {
      "command": "npx",
      "args": ["@modelcontextprotocol/server-http", "http://localhost:8080/mcp"]
    }
  }
}
```

## 💻 Using with Claude Code

### Stdio Mode
```bash
# Add WebDriver MCP server
claude mcp add webdriver --env WEBDRIVER_HEADLESS=true -- /path/to/rust-browser-mcp --transport stdio --browser chrome

# With multiple browsers
claude mcp add webdriver \
  --env WEBDRIVER_CONCURRENT_DRIVERS=firefox,chrome \
  --env WEBDRIVER_HEADLESS=true \
  -- /path/to/rust-browser-mcp --transport stdio
```

### HTTP Mode  
```bash
# Start the server
./rust-browser-mcp --transport http --bind 127.0.0.1:8080 --no-auth

# Add to Claude Code
claude mcp add webdriver --transport http http://localhost:8080/mcp
```

## 🌟 Example Usage with Claude

Once configured, you can ask Claude to:

### Basic Automation
- **"Take a screenshot of example.com"**
- **"Navigate to github.com and search for 'rust mcp'"**  
- **"Click the first search result"**
- **"Fill out the contact form on this website with my information"**

### Advanced Testing
- **"Run a performance test on this e-commerce site's checkout flow"**
- **"Monitor the memory usage while navigating through this single-page app"**
- **"Check the console logs for any JavaScript errors on this page"**
- **"Test how long this page takes to load and capture timing metrics"**

### Multi-Session Workflows
- **"Open two browser sessions - one for testing and one for reference"**
- **"Use session 'firefox_test' to test the mobile view of this site"**
- **"Compare loading times between Chrome and Firefox"**

## 🛠️ Available Tools

### Navigation & Interaction
- `navigate` - Go to URL
- `back`, `forward`, `refresh` - Browser navigation
- `click`, `send_keys`, `hover` - Element interaction
- `find_element`, `find_elements` - Element location
- `get_title`, `get_text`, `get_attribute` - Information extraction
- `screenshot` - Capture page images
- `execute_script` - Run JavaScript

### Advanced Features
- `fill_and_submit_form` - Automated form handling
- `login_form` - Smart login automation
- `wait_for_element` - Wait for dynamic content
- `scroll_to_element` - Smooth scrolling

### Performance & Monitoring
- `get_performance_metrics` - Page load and resource timing
- `monitor_memory_usage` - JavaScript heap monitoring
- `run_performance_test` - Automated performance testing
- `monitor_resource_usage` - Network, FPS, CPU monitoring
- `get_console_logs` - JavaScript error detection

### Session Management
- `list_managed_drivers` - View active browsers
- `get_healthy_endpoints` - Check driver health
- `start_driver`, `stop_driver` - Manual lifecycle control
- `refresh_driver_health` - Health check refresh

## ⚙️ Configuration

### Command Line Options

```bash
# Basic usage
rust-browser-mcp --transport stdio --browser chrome

# HTTP server mode
rust-browser-mcp --transport http --bind 0.0.0.0:8080 --browser firefox

# Enable advanced features
rust-browser-mcp --browser chrome --enable-performance-memory --transport stdio
```

**Options:**
- `--transport, -t`: Transport mode (`stdio` or `http`)
- `--browser, -b`: Browser driver (`chrome`, `firefox`, `edge`)
- `--bind`: HTTP server address (default: `127.0.0.1:8080`)
- `--no-auth`: Disable OAuth for HTTP mode
- `--enable-performance-memory`: Enable Chrome memory APIs

### Environment Variables

```bash
# Browser configuration
export WEBDRIVER_PREFERRED_DRIVER="chrome"              # Default browser
export WEBDRIVER_HEADLESS="true"                        # Headless mode
export WEBDRIVER_CONCURRENT_DRIVERS="firefox,chrome"    # Auto-start multiple browsers

# Performance settings  
export WEBDRIVER_STARTUP_TIMEOUT_MS="15000"             # Driver startup timeout
export WEBDRIVER_ENABLE_PERFORMANCE_MEMORY="true"       # Chrome memory APIs
export WEBDRIVER_AUTO_START="true"                      # Auto-start drivers (default)
```

### Browser-Specific Configuration

**Chrome (with performance monitoring):**
```json
{
  "env": {
    "WEBDRIVER_PREFERRED_DRIVER": "chrome",
    "WEBDRIVER_HEADLESS": "true", 
    "WEBDRIVER_ENABLE_PERFORMANCE_MEMORY": "true"
  }
}
```

**Firefox (with multiple drivers):**
```json
{
  "env": {
    "WEBDRIVER_PREFERRED_DRIVER": "firefox",
    "WEBDRIVER_CONCURRENT_DRIVERS": "firefox,chrome",
    "WEBDRIVER_HEADLESS": "false"
  }
}
```

## 🔧 Advanced Features

### Multi-Session Support
- **Concurrent Sessions**: Run multiple browser instances simultaneously
- **Session Isolation**: Each session maintains separate cookies, localStorage
- **Browser Preference**: Use session IDs like `firefox_session1`, `chrome_work`
- **Session Persistence**: Sessions survive across multiple tool calls

### Performance Monitoring
- **Real-time Metrics**: Page load times, resource loading, JavaScript execution
- **Memory Tracking**: Heap usage, garbage collection, memory leaks detection  
- **Network Analysis**: Request timing, response sizes, failed requests
- **User Experience**: Frame rates, input responsiveness, paint timing

### Health Management  
- **Proactive Monitoring**: Automatic health checks for all drivers
- **Self-Healing**: Failed drivers automatically restart
- **Load Balancing**: Requests route to healthiest available driver
- **Diagnostics**: Detailed status reporting for troubleshooting

## 🚀 Enterprise Features

### OAuth Authentication (HTTP Mode)
For production deployments:

```bash
# Start OAuth-protected server
./rust-browser-mcp --transport http --bind 0.0.0.0:3000

# Visit http://localhost:3000/oauth/authorize for setup
```

### Docker Deployment
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y chromium-driver firefox-esr geckodriver
COPY --from=builder /app/target/release/rust-browser-mcp /usr/local/bin/
EXPOSE 8080
CMD ["rust-browser-mcp", "--transport", "http", "--bind", "0.0.0.0:8080"]
```

### Monitoring & Logging
```bash
# Enable debug logging
RUST_LOG=debug ./rust-browser-mcp --transport stdio

# Performance monitoring
export WEBDRIVER_ENABLE_PERFORMANCE_MEMORY="true"
export RUST_LOG="rust_browser_mcp::performance=debug"
```

## 🤝 Examples & Use Cases

### Web Testing
- **"Test the checkout flow on this e-commerce site and measure performance"**
- **"Verify all links on this page work correctly"**  
- **"Check if this form validates input properly"**

### Content Analysis
- **"Extract all product prices from this category page"**
- **"Take screenshots of this page in different browser sizes"**
- **"Get the page load time for this news article"**

### Automation Workflows  
- **"Log into this admin panel and generate a report"**
- **"Monitor this dashboard for changes every 5 minutes"**
- **"Compare how this page renders in Chrome vs Firefox"**

## 📚 API Reference

See [examples/](examples/) directory for complete working examples:
- `stdio_client.rs` - Basic stdio usage
- `http_client.rs` - HTTP transport example  
- `oauth_demo_server.rs` - OAuth-protected server
- `advanced_monitoring.rs` - Performance testing showcase

## 🔍 Troubleshooting

### Common Issues

**Driver not found:**
```bash
# Install the WebDriver for your browser
brew install chromedriver  # macOS
sudo apt install chromium-chromedriver  # Ubuntu
```

**Permission denied:**
```bash
# Make binary executable
chmod +x rust-browser-mcp
```

**Port already in use:**
```bash
# Change the port
./rust-browser-mcp --transport http --bind 127.0.0.1:8081
```

**Browser won't start:**
```bash
# Check WebDriver installation
chromedriver --version
geckodriver --version

# Enable debug logging
RUST_LOG=debug ./rust-browser-mcp --transport stdio
```

### Getting Help

- **Check logs**: Enable `RUST_LOG=debug` for detailed output
- **Verify setup**: Use example configurations from this README
- **Test manually**: Run `cargo run --example stdio_client` to test
- **Check drivers**: Ensure WebDriver binaries are in PATH and executable

## 📄 License

MIT License - see [LICENSE](LICENSE) for details.
# Multi-Browser Recipe Execution Issue

## ✅ **RESOLVED - Issue Fixed Successfully**

## Problem Summary (Historical)
Chrome WebDriver sessions failed with "No matching capabilities found" error when executed within multi-browser recipes, while individual Chrome sessions worked perfectly. Firefox showed connection issues in individual sessions.

## Final Status - ALL WORKING ✅
- ✅ Individual Chrome session works
- ✅ Individual Firefox session works  
- ✅ Recipe Firefox execution works  
- ✅ Recipe Chrome execution works
- ✅ Multi-browser recipes create both screenshots successfully

## Resolution Summary
**Root Cause**: The `create_configured_client` method wasn't using session ID to determine browser capabilities, causing Chrome sessions to get wrong capabilities when multiple drivers were running.

**Solution Applied**: 
1. Modified `create_configured_client` to accept session ID parameter
2. Added session-aware browser type detection using `extract_browser_preference_from_session`
3. Enhanced logging for better debugging of capability assignment

**Test Results**: 
- Chrome creates chrome_test.png (21,972 bytes) ✅
- Firefox creates firefox_test.png (44,957 bytes) ✅  
- Recipe execution time: ~3.5s initially, ~129ms subsequent runs ✅

---

## Historical Information (For Reference)

### Reproduction Steps (Historical)
1. Start both Chrome and Firefox drivers:
   ```
   mcp__browser__start_driver("chrome")  
   mcp__browser__start_driver("firefox")
   ```

2. Test individual sessions:
   ```
   # Chrome works
   mcp__browser__navigate("https://example.com", "chrome_test")
   
   # Firefox fails with connection error
   mcp__browser__navigate("https://example.com", "firefox_test") 
   ```

3. Execute multi-browser recipe:
   ```
   mcp__browser__execute_recipe("test_multi_browser")
   ```
   Result: Chrome fails, Firefox succeeds

### Error Messages (Historical)

#### Recipe Chrome Failure (Historical)
```
Execution error: Failed to get client for navigation session 'chrome_recipe_session': Generic error: WebDriver session creation error: webdriver did not create session: session not created: No matching capabilities found
```

#### Individual Firefox Failure (Historical) 
```
Failed to create webdriver client: Generic error: WebDriver session creation error: webdriver server did not respond (legacy client): client error (Connect)
```

### Analysis (Historical)

### Root Cause Investigation
The issue appears to be in the session isolation and capability assignment logic during multi-browser recipe execution. Key findings:

1. **Concurrency Issue**: Chrome capabilities work in isolation but fail when Firefox driver is also active
2. **Session ID Logic**: Recipe creates `chrome_recipe_session` and `firefox_recipe_session` but Chrome session gets wrong capabilities
3. **Non-deterministic Fallback**: Fixed the HashMap iteration issue in endpoint resolution, but core problem persists

### Code Areas Investigated
- `src/client.rs:103-151` - `create_configured_client()` browser detection and capabilities
- `src/client.rs:195-241` - `resolve_webdriver_endpoint_for_session()` endpoint selection  
- `src/recipes/execution.rs` - Multi-browser recipe execution flow

### Fixes Applied
1. ✅ **Deterministic Endpoint Selection**: Replaced non-deterministic HashMap iteration with priority-based selection
2. ✅ **Enhanced Logging**: Added debug logging for session browser preference detection
3. ✅ **Capabilities Format**: Restored `browserName` field and essential Chrome arguments

## Current Hypothesis
The issue likely stems from either:
1. **Capability Contamination**: Chrome sessions getting Firefox capabilities or vice versa
2. **Session Timing**: Race condition in session creation when multiple browsers are involved  
3. **Driver State Conflict**: Chrome and Firefox drivers interfering with each other's session creation

## Technical Details

### Working Chrome Capabilities (Individual)
```json
{
  "browserName": "chrome",
  "goog:chromeOptions": {
    "args": ["--no-sandbox", "--disable-dev-shm-usage", "--disable-gpu", "--remote-debugging-port=0", "--headless"]
  }
}
```

### Firefox Capabilities  
```json
{
  "browserName": "firefox", 
  "moz:firefoxOptions": {
    "args": ["--headless"]
  }
}
```

### Session ID Logic
- Individual: User-provided session ID (e.g., `chrome_test`)
- Recipe: Auto-generated `{browser}_recipe_session` (e.g., `chrome_recipe_session`)

## Next Steps
1. **Debug Recipe Session Creation**: Add detailed logging to recipe execution to trace exact capabilities being sent
2. **Test Sequential vs Concurrent**: Try executing browsers sequentially instead of concurrently in recipes
3. **Isolate Capability Assignment**: Verify that each session gets the correct browser-specific capabilities
4. **Driver Communication**: Check if Chrome and Firefox drivers are conflicting at the WebDriver protocol level

## Environment
- ChromeDriver: 138.0.7204.157
- Firefox/GeckoDriver: 6565 (snap)
- fantoccini: 0.22.0
- OS: Linux WSL2

## Impact
- Multi-browser testing recipes cannot execute reliably
- Chrome-Firefox comparison screenshots fail
- Cross-browser testing workflows are blocked

---
*Issue identified through systematic investigation of WebDriver session creation and multi-browser concurrency handling.*
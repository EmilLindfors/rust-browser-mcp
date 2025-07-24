# WebDriver MCP Architecture Refactor

## Overview
Refactor the webdriver session handling from reactive (client-triggered) to proactive (server-spawned) architecture for better performance, multi-session support, and cleaner separation of concerns.

## Current Issues
- ❌ Webdrivers start reactively on first tool call (startup latency)
- ❌ Client actions coupled to driver lifecycle management  
- ❌ Delayed initial responses due to lazy driver startup
- ❌ No true multi-session efficiency (each session may trigger new drivers)

## Target Architecture
- ✅ Server proactively spawns Firefox + Chrome webdrivers at startup
- ✅ Clients connect to existing webdriver endpoints and get sessions
- ✅ Multiple sessions efficiently share the same webdriver process
- ✅ Fast tool call responses (no startup delays)
- ✅ Clean separation: DriverManager handles processes, ClientManager handles sessions

## Implementation Plan

### Phase 1: Core Architecture Changes

#### 1. Modify WebDriverServer Initialization
**File**: `src/server.rs`
- [ ] Add proactive webdriver startup in `WebDriverServer::new()` and `with_config()`
- [ ] Start both Firefox (port 4444) and Chrome (port 9515) drivers by default
- [ ] Handle startup failures gracefully (continue with available drivers)
- [ ] Store available endpoints in server state

#### 2. Update ClientManager Session Handling  
**File**: `src/client.rs`
- [ ] Remove reactive driver startup from `resolve_webdriver_endpoint()`
- [ ] Replace `auto_start_for_endpoint()` calls with endpoint selection logic
- [ ] Add `get_available_endpoints()` method to query server state
- [ ] Update `get_or_create_client()` to select from available endpoints

#### 3. Configuration Updates
**File**: `src/config.rs` 
- [ ] Add `concurrent_drivers: Vec<String>` (default: ["firefox", "chrome"])
- [ ] Add `driver_startup_timeout_ms: u64` for initialization timeout
- [ ] Update `validate()` method for new config options
- [ ] Update setup guidance documentation

### Phase 2: Session Management Enhancements

#### 4. Endpoint Selection Logic
**File**: `src/client.rs`
- [ ] Add browser preference handling (`session_id` → browser mapping)
- [ ] Implement round-robin or load balancing for endpoint selection
- [ ] Support explicit browser selection via session naming (e.g., `firefox_session_1`)
- [ ] Fallback logic when preferred browser unavailable

#### 5. Health Checks and Failover
**File**: `src/driver.rs`
- [ ] Add periodic health checks for spawned webdrivers
- [ ] Implement auto-restart logic for crashed drivers  
- [ ] Add endpoint status reporting (`/status` endpoint monitoring)
- [ ] Graceful degradation when drivers fail

#### 6. Enhanced Driver Management
**File**: `src/driver.rs`
- [ ] Add `start_concurrent_drivers()` method for multi-driver startup
- [ ] Implement driver readiness waiting with timeout
- [ ] Add `get_healthy_endpoints()` method for ClientManager
- [ ] Better error handling and logging for startup failures

### Phase 3: Testing and Validation

#### 7. Multi-Session Testing
**Files**: `examples/` or `tests/`
- [ ] Create test for concurrent sessions on same webdriver
- [ ] Test session isolation (separate browser windows/tabs)
- [ ] Load testing with multiple simultaneous clients
- [ ] Failover testing (kill driver process during active sessions)

#### 8. Performance Validation
- [ ] Benchmark first tool call latency (before/after)
- [ ] Memory usage comparison with concurrent sessions
- [ ] Driver startup time vs. on-demand startup comparison

### Phase 4: Documentation and Polish

#### 9. Documentation Updates
- [ ] Update README.md with new architecture explanation
- [ ] Document new configuration options
- [ ] Add multi-session usage examples
- [ ] Update setup guidance for concurrent drivers

#### 10. Tool Updates
**File**: `src/tools.rs`
- [ ] Add tools for querying available endpoints
- [ ] Add tools for checking driver health status
- [ ] Update existing tool descriptions for session handling

## Implementation Notes

### Key Design Decisions
1. **Default Concurrent Drivers**: Start both Firefox and Chrome by default for maximum compatibility
2. **Session Naming**: Use session_id format like `firefox_default`, `chrome_session1` for explicit browser selection
3. **Graceful Degradation**: Continue server startup even if some drivers fail
4. **Health Monitoring**: Periodic checks to restart failed drivers automatically

### Breaking Changes
- `WEBDRIVER_ENDPOINT="auto"` behavior changes (now selects from pre-started drivers)
- Session creation slightly faster but server startup slightly slower
- New configuration options may require documentation updates

### Migration Path
1. Implement changes with feature flag or config option
2. Maintain backward compatibility during transition
3. Update examples and documentation
4. Remove old reactive startup code after validation

## File Modification Summary

| File | Changes | Priority |
|------|---------|----------|
| `src/server.rs` | Add proactive driver startup | High |
| `src/client.rs` | Remove reactive startup, add endpoint selection | High |
| `src/config.rs` | Add concurrent driver config options | Medium |
| `src/driver.rs` | Add concurrent startup, health checks | Medium |
| `src/tools.rs` | Add endpoint management tools | Low |
| `examples/` | Add multi-session examples | Low |

## ✅ Implementation Status - MOSTLY COMPLETE!

### ✅ Completed Successfully:
- [x] **TODO.md Planning Document** - Comprehensive implementation roadmap
- [x] **Proactive WebDriver Startup** - Server spawns webdrivers at initialization  
- [x] **Smart Endpoint Selection** - Session routing based on session ID prefixes
- [x] **Enhanced Configuration** - Support for concurrent drivers and timeouts
- [x] **Health Check Framework** - Periodic monitoring and endpoint tracking
- [x] **Integration Tests** - Comprehensive test suite validates architecture
- [x] **Driver Lifecycle Management** - Proper cleanup and process management

### ⚠️ Known Issues to Fix:
- [ ] **Healthy Endpoints Sync Issue**: `healthy_endpoints` not properly maintained after concurrent startup
  - Root cause: Process tracking vs endpoint tracking mismatch
  - Impact: System falls back to reactive startup instead of using pre-started drivers
  - Fix needed: Ensure `healthy_endpoints` updated after successful concurrent startup

### 🎯 Current Performance Results:
- **Driver Startup**: ~1s (done once at server start) ✅  
- **Session Creation**: Currently ~2s (falls back to reactive due to sync issue) ⚠️
- **Multi-Session Support**: Working correctly ✅
- **Browser Selection**: Working correctly ✅
- **Health Monitoring**: Framework exists, needs sync fix ⚠️

## Success Criteria - Status
- [x] Server starts with multiple webdrivers running
- [ ] First tool call has no startup latency (blocked by sync issue)
- [x] Multiple sessions can use same webdriver process  
- [x] Sessions are properly isolated (separate browser contexts)
- [x] Health monitoring framework implemented
- [x] Backward compatibility maintained for existing clients

## 🚀 Next Steps
1. **Fix healthy endpoints synchronization** (main blocker)
2. **Re-run tests to validate performance improvements**
3. **Architecture refactor complete!**
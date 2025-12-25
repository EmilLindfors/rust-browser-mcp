//! Performance monitoring handlers
//!
//! Handles browser performance monitoring:
//! - Console log collection
//! - Performance metrics (navigation, resources, paint)
//! - Memory usage monitoring
//! - CPU and FPS monitoring
//! - Performance testing with actions

use base64::{Engine as _, engine::general_purpose};
use fantoccini::Locator;
use rmcp::{ErrorData as McpError, model::CallToolResult};
use serde_json::{Map, Value};

use crate::{
    ClientManager,
    tools::{error_response, success_response},
};
use super::extract_session_id;

/// Get console logs from the browser
pub async fn handle_get_console_logs(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let level_filter = arguments
        .as_ref()
        .and_then(|args| args.get("level"))
        .and_then(|v| v.as_str())
        .unwrap_or("all");

    let since_timestamp = arguments
        .as_ref()
        .and_then(|args| args.get("since_timestamp"))
        .and_then(|v| v.as_f64());

    let wait_timeout = arguments
        .as_ref()
        .and_then(|args| args.get("wait_timeout"))
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);

    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            // Wait for JavaScript execution to complete before capturing logs
            if wait_timeout > 0.0 {
                tokio::time::sleep(std::time::Duration::from_secs_f64(wait_timeout)).await;
            }

            // Simple script to retrieve stored console logs
            let retrieve_script = r#"
                try {
                    return window.__mcpConsoleLogs || [];
                } catch (e) {
                    return [];
                }
            "#;

            match client.execute(retrieve_script, vec![]).await {
                Ok(result) => {
                    // Try to parse the result as JSON array of log entries
                    let formatted_logs = if let Ok(logs) = serde_json::from_value::<Vec<serde_json::Value>>(result.clone()) {
                        if logs.is_empty() {
                            "No console logs found.".to_string()
                        } else {
                            logs.into_iter()
                                .filter(|log| {
                                    // Filter by level
                                    if level_filter != "all" {
                                        let log_level = log.get("level").and_then(|v| v.as_str()).unwrap_or("");
                                        if log_level != level_filter {
                                            return false;
                                        }
                                    }

                                    // Filter by timestamp
                                    if let Some(since) = since_timestamp {
                                        let log_timestamp = log.get("timestamp").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        if log_timestamp < since {
                                            return false;
                                        }
                                    }

                                    true
                                })
                                .map(|log| {
                                    let level = log.get("level").and_then(|v| v.as_str()).unwrap_or("unknown");
                                    let message = log.get("message").and_then(|v| v.as_str()).unwrap_or("");
                                    let timestamp = log.get("timestamp").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let _url = log.get("url").and_then(|v| v.as_str()).unwrap_or("");

                                    let time_str = if timestamp > 0 {
                                        format!("[{}ms] ", timestamp)
                                    } else {
                                        "".to_string()
                                    };

                                    format!("{time_str}{level}: {message}")
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        }
                    } else {
                        // Fallback if parsing fails
                        format!("Raw result: {result:?}")
                    };

                    Ok(success_response(format!(
                        "Console logs (session: {session}):\n{formatted_logs}"
                    )))
                }
                Err(e) => Ok(error_response(format!("Failed to retrieve console logs: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!(
            "Failed to create webdriver client: {e}"
        ))),
    }
}

/// Get performance metrics from the browser
pub async fn handle_get_performance_metrics(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let include_resources = arguments
        .as_ref()
        .and_then(|args| args.get("include_resources"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_navigation = arguments
        .as_ref()
        .and_then(|args| args.get("include_navigation"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_paint = arguments
        .as_ref()
        .and_then(|args| args.get("include_paint"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let performance_script = format!(r#"
                const metrics = {{}};

                // Basic timing info
                if (performance.timing) {{
                    metrics.timing = {{
                        navigationStart: performance.timing.navigationStart,
                        loadEventEnd: performance.timing.loadEventEnd,
                        domContentLoadedEventEnd: performance.timing.domContentLoadedEventEnd,
                        responseEnd: performance.timing.responseEnd,
                        domComplete: performance.timing.domComplete
                    }};

                    metrics.calculated = {{
                        pageLoadTime: performance.timing.loadEventEnd - performance.timing.navigationStart,
                        domContentLoadedTime: performance.timing.domContentLoadedEventEnd - performance.timing.navigationStart,
                        responseTime: performance.timing.responseEnd - performance.timing.navigationStart
                    }};
                }}

                // Navigation timing (newer API)
                if ({include_navigation} && performance.getEntriesByType) {{
                    const nav = performance.getEntriesByType('navigation')[0];
                    if (nav) {{
                        metrics.navigation = {{
                            type: nav.type,
                            redirectCount: nav.redirectCount,
                            transferSize: nav.transferSize,
                            encodedBodySize: nav.encodedBodySize,
                            decodedBodySize: nav.decodedBodySize,
                            duration: nav.duration,
                            domContentLoadedEventStart: nav.domContentLoadedEventStart,
                            domContentLoadedEventEnd: nav.domContentLoadedEventEnd,
                            loadEventStart: nav.loadEventStart,
                            loadEventEnd: nav.loadEventEnd
                        }};
                    }}
                }}

                // Resource timing
                if ({include_resources} && performance.getEntriesByType) {{
                    const resources = performance.getEntriesByType('resource');
                    metrics.resources = resources.map(r => ({{
                        name: r.name,
                        duration: r.duration,
                        transferSize: r.transferSize,
                        encodedBodySize: r.encodedBodySize,
                        decodedBodySize: r.decodedBodySize,
                        initiatorType: r.initiatorType
                    }})).slice(0, 50); // Limit to first 50 resources
                }}

                // Paint timing
                if ({include_paint} && performance.getEntriesByType) {{
                    const paintEntries = performance.getEntriesByType('paint');
                    metrics.paint = {{}};
                    paintEntries.forEach(entry => {{
                        metrics.paint[entry.name] = entry.startTime;
                    }});
                }}

                // Memory info if available
                if (performance.memory) {{
                    metrics.memory = {{
                        usedJSHeapSize: performance.memory.usedJSHeapSize,
                        totalJSHeapSize: performance.memory.totalJSHeapSize,
                        jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                    }};
                }}

                return metrics;
            "#);

            match client.execute(&performance_script, vec![]).await {
                Ok(result) => Ok(success_response(format!(
                    "Performance metrics collected (session: {session}):\n{result:#?}"
                ))),
                Err(e) => Ok(error_response(format!("Failed to collect performance metrics: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
    }
}

/// Monitor memory usage over time
pub async fn handle_monitor_memory_usage(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let duration_seconds = arguments
        .as_ref()
        .and_then(|args| args.get("duration_seconds"))
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0);
    let interval_ms = arguments
        .as_ref()
        .and_then(|args| args.get("interval_ms"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1000.0);
    let include_gc_info = arguments
        .as_ref()
        .and_then(|args| args.get("include_gc_info"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let memory_script = format!(r#"
                return new Promise((resolve) => {{
                    const samples = [];
                    const startTime = Date.now();
                    const duration = {duration_seconds} * 1000;
                    const interval = {interval_ms};

                    function collectSample() {{
                        const sample = {{
                            timestamp: Date.now() - startTime,
                            url: window.location.href
                        }};

                        if (performance.memory) {{
                            sample.memory = {{
                                usedJSHeapSize: performance.memory.usedJSHeapSize,
                                totalJSHeapSize: performance.memory.totalJSHeapSize,
                                jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                            }};
                        }}

                        // Try to get GC info if available
                        if ({include_gc_info} && performance.measureUserAgentSpecificMemory) {{
                            performance.measureUserAgentSpecificMemory().then(result => {{
                                sample.detailedMemory = result;
                            }}).catch(() => {{
                                // GC info not available
                            }});
                        }}

                        samples.push(sample);

                        if (Date.now() - startTime < duration) {{
                            setTimeout(collectSample, interval);
                        }} else {{
                            // Calculate memory leak indicators
                            const analysis = {{}};
                            if (samples.length > 1) {{
                                const first = samples[0];
                                const last = samples[samples.length - 1];

                                if (first.memory && last.memory) {{
                                    analysis.memoryGrowth = {{
                                        usedHeapGrowth: last.memory.usedJSHeapSize - first.memory.usedJSHeapSize,
                                        totalHeapGrowth: last.memory.totalJSHeapSize - first.memory.totalJSHeapSize,
                                        growthRate: (last.memory.usedJSHeapSize - first.memory.usedJSHeapSize) / (duration / 1000)
                                    }};

                                    analysis.leakIndicators = {{
                                        steadyGrowth: analysis.memoryGrowth.usedHeapGrowth > 1024 * 1024, // 1MB growth
                                        highGrowthRate: analysis.memoryGrowth.growthRate > 512 * 1024 // 512KB/sec
                                    }};
                                }}
                            }}

                            resolve({{
                                samples: samples,
                                analysis: analysis,
                                summary: {{
                                    duration: duration,
                                    sampleCount: samples.length,
                                    interval: interval
                                }}
                            }});
                        }}
                    }}

                    collectSample();
                }});
            "#);

            match client.execute(&memory_script, vec![]).await {
                Ok(result) => Ok(success_response(format!(
                    "Memory monitoring completed (session: {session}):\n{result:#?}"
                ))),
                Err(e) => Ok(error_response(format!("Failed to monitor memory usage: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
    }
}

/// Run a performance test with a sequence of actions
pub async fn handle_run_performance_test(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let test_actions = arguments
        .as_ref()
        .and_then(|args| args.get("test_actions"))
        .and_then(|v| v.as_array())
        .ok_or_else(|| McpError::invalid_params("test_actions array is required", None))?;
    let iterations = arguments
        .as_ref()
        .and_then(|args| args.get("iterations"))
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0) as usize;
    let collect_screenshots = arguments
        .as_ref()
        .and_then(|args| args.get("collect_screenshots"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let mut results = Vec::new();

            for iteration in 0..iterations {
                let mut iteration_results = Vec::new();

                // Start performance monitoring
                let start_script = r#"
                    window.__perfTestStart = performance.now();
                    window.__perfTestMarks = [];
                    return "Performance test started";
                "#;
                client.execute(start_script, vec![]).await.ok();

                // Execute test actions
                for (action_idx, action) in test_actions.iter().enumerate() {
                    let action_obj = action.as_object().ok_or_else(|| {
                        McpError::invalid_params("Each test action must be an object", None)
                    })?;

                    let action_type = action_obj.get("type")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| McpError::invalid_params("Action type is required", None))?;

                    let mark_script = format!(r#"
                        window.__perfTestMarks.push({{
                            action: "{action_type}",
                            index: {action_idx},
                            timestamp: performance.now() - window.__perfTestStart
                        }});
                    "#);
                    client.execute(&mark_script, vec![]).await.ok();

                    match action_type {
                        "click" => {
                            if let Some(selector) = action_obj.get("selector").and_then(|v| v.as_str()) {
                                if let Ok(element) = client.find(Locator::Css(selector)).await {
                                    element.click().await.ok();
                                }
                            }
                        }
                        "scroll" => {
                            if let Some(selector) = action_obj.get("selector").and_then(|v| v.as_str()) {
                                let scroll_script = format!("document.querySelector('{selector}')?.scrollIntoView();");
                                client.execute(&scroll_script, vec![]).await.ok();
                            }
                        }
                        "wait" => {
                            if let Some(duration_ms) = action_obj.get("duration_ms").and_then(|v| v.as_f64()) {
                                tokio::time::sleep(std::time::Duration::from_millis(duration_ms as u64)).await;
                            }
                        }
                        "navigate" => {
                            if let Some(url) = action_obj.get("url").and_then(|v| v.as_str()) {
                                client.goto(url).await.ok();
                            }
                        }
                        _ => {
                            // Unknown action type, skip
                        }
                    }

                    // Small delay between actions
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                // Collect final metrics
                let end_script = r#"
                    const endTime = performance.now();
                    const testDuration = endTime - window.__perfTestStart;

                    const result = {
                        testDuration: testDuration,
                        marks: window.__perfTestMarks,
                        finalMetrics: {}
                    };

                    // Collect performance metrics
                    if (performance.memory) {
                        result.finalMetrics.memory = {
                            usedJSHeapSize: performance.memory.usedJSHeapSize,
                            totalJSHeapSize: performance.memory.totalJSHeapSize,
                            jsHeapSizeLimit: performance.memory.jsHeapSizeLimit
                        };
                    }

                    // Collect paint metrics
                    const paintEntries = performance.getEntriesByType('paint');
                    result.finalMetrics.paint = {};
                    paintEntries.forEach(entry => {
                        result.finalMetrics.paint[entry.name] = entry.startTime;
                    });

                    return result;
                "#;

                match client.execute(end_script, vec![]).await {
                    Ok(iteration_result) => {
                        iteration_results.push(iteration_result);

                        if collect_screenshots {
                            if let Ok(screenshot) = client.screenshot().await {
                                // Convert screenshot to base64
                                let screenshot_b64 = general_purpose::STANDARD.encode(&screenshot);
                                iteration_results.push(serde_json::json!({
                                    "screenshot": format!("data:image/png;base64,{}", screenshot_b64)
                                }));
                            }
                        }
                    }
                    Err(e) => {
                        iteration_results.push(serde_json::json!({
                            "error": format!("Failed to collect metrics: {}", e)
                        }));
                    }
                }

                results.push(serde_json::json!({
                    "iteration": iteration,
                    "results": iteration_results
                }));
            }

            Ok(success_response(format!(
                "Performance test completed (session: {session}):\n{results:#?}"
            )))
        }
        Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
    }
}

/// Monitor resource usage (network, FPS, CPU)
pub async fn handle_monitor_resource_usage(
    client_manager: &ClientManager,
    arguments: &Option<Map<String, Value>>,
) -> Result<CallToolResult, McpError> {
    let duration_seconds = arguments
        .as_ref()
        .and_then(|args| args.get("duration_seconds"))
        .and_then(|v| v.as_f64())
        .unwrap_or(30.0);
    let include_network = arguments
        .as_ref()
        .and_then(|args| args.get("include_network"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_cpu = arguments
        .as_ref()
        .and_then(|args| args.get("include_cpu"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_fps = arguments
        .as_ref()
        .and_then(|args| args.get("include_fps"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let network_filter = arguments
        .as_ref()
        .and_then(|args| args.get("network_filter"))
        .and_then(|v| v.as_str())
        .unwrap_or(".*");
    let session_id = extract_session_id(arguments);

    match client_manager.get_or_create_client(session_id).await {
        Ok((session, client)) => {
            let resource_script = format!(r#"
                return new Promise((resolve) => {{
                    const results = {{
                        network: [],
                        fps: [],
                        cpu: [],
                        summary: {{}}
                    }};

                    const startTime = performance.now();
                    const duration = {duration_seconds} * 1000;
                    const networkFilter = new RegExp('{network_filter}');

                    // Network monitoring
                    if ({include_network}) {{
                        const observer = new PerformanceObserver((list) => {{
                            for (const entry of list.getEntries()) {{
                                if (entry.entryType === 'resource' && networkFilter.test(entry.name)) {{
                                    results.network.push({{
                                        name: entry.name,
                                        type: entry.initiatorType,
                                        duration: entry.duration,
                                        transferSize: entry.transferSize,
                                        encodedBodySize: entry.encodedBodySize,
                                        startTime: entry.startTime,
                                        responseEnd: entry.responseEnd
                                    }});
                                }}
                            }}
                        }});
                        observer.observe({{entryTypes: ['resource']}});
                    }}

                    // FPS monitoring
                    if ({include_fps}) {{
                        let frameCount = 0;
                        let lastTime = performance.now();

                        function countFrame() {{
                            frameCount++;
                            const currentTime = performance.now();

                            if (currentTime - lastTime >= 1000) {{
                                results.fps.push({{
                                    timestamp: currentTime - startTime,
                                    fps: frameCount
                                }});
                                frameCount = 0;
                                lastTime = currentTime;
                            }}

                            if (currentTime - startTime < duration) {{
                                requestAnimationFrame(countFrame);
                            }}
                        }}
                        requestAnimationFrame(countFrame);
                    }}

                    // CPU monitoring (approximation using timing)
                    if ({include_cpu}) {{
                        let cpuSamples = [];

                        function sampleCPU() {{
                            const start = performance.now();

                            // Perform a small CPU-intensive task to measure responsiveness
                            let sum = 0;
                            for (let i = 0; i < 10000; i++) {{
                                sum += Math.random();
                            }}

                            const end = performance.now();
                            const cpuTime = end - start;

                            cpuSamples.push({{
                                timestamp: start - startTime,
                                taskTime: cpuTime,
                                responsiveness: cpuTime < 5 ? 'good' : cpuTime < 15 ? 'fair' : 'poor'
                            }});

                            if (end - startTime < duration) {{
                                setTimeout(sampleCPU, 1000);
                            }}
                        }}
                        setTimeout(sampleCPU, 100);
                    }}

                    // Final collection
                    setTimeout(() => {{
                        results.summary = {{
                            duration: duration,
                            networkRequests: results.network.length,
                            averageFPS: results.fps.length > 0 ?
                                results.fps.reduce((a, b) => a + b.fps, 0) / results.fps.length : 0,
                            totalTransferSize: results.network.reduce((a, b) => a + (b.transferSize || 0), 0),
                            slowRequests: results.network.filter(r => r.duration > 1000).length
                        }};

                        resolve(results);
                    }}, duration + 100);
                }});
            "#);

            match client.execute(&resource_script, vec![]).await {
                Ok(result) => Ok(success_response(format!(
                    "Resource usage monitoring completed (session: {session}):\n{result:#?}"
                ))),
                Err(e) => Ok(error_response(format!("Failed to monitor resource usage: {e}"))),
            }
        }
        Err(e) => Ok(error_response(format!("Failed to create webdriver client: {e}"))),
    }
}

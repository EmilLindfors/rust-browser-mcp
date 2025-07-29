use std::time::{Duration, Instant};
use rmcp::{
    ServiceExt,
    model::CallToolRequestParam,
    object,
    transport::{ConfigureCommandExt, TokioChildProcess},
    RoleClient,
    service::RunningService,
};
use tokio::process::Command;

pub struct TestClient {
    pub client: RunningService<RoleClient, ()>,
}

impl TestClient {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let client = ()
            .serve(TokioChildProcess::new(Command::new("cargo").configure(
                |cmd| {
                    cmd.arg("run").arg("--bin").arg("rust-browser-mcp");
                },
            ))?)
            .await?;

        Ok(TestClient { client })
    }

    pub async fn start_driver(&self, driver_type: &str) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        self.client
            .call_tool(CallToolRequestParam {
                name: "start_driver".into(),
                arguments: Some(object!({ "driver_type": driver_type })),
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn start_driver_with_session(&self, driver_type: &str, session_id: &str) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        self.client
            .call_tool(CallToolRequestParam {
                name: "start_driver".into(),
                arguments: Some(object!({ 
                    "driver_type": driver_type,
                    "session_id": session_id
                })),
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn navigate(&self, url: &str, session_id: Option<&str>) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        let args = if let Some(sid) = session_id {
            object!({ "url": url, "session_id": sid })
        } else {
            object!({ "url": url })
        };

        self.client
            .call_tool(CallToolRequestParam {
                name: "navigate".into(),
                arguments: Some(args),
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn get_title(&self, session_id: Option<&str>) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        let args = if let Some(sid) = session_id {
            Some(object!({ "session_id": sid }))
        } else {
            None
        };

        self.client
            .call_tool(CallToolRequestParam {
                name: "get_title".into(),
                arguments: args,
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn stop_all_drivers(&self) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        self.client
            .call_tool(CallToolRequestParam {
                name: "stop_all_drivers".into(),
                arguments: None,
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn list_tools(&self) -> Result<Vec<rmcp::model::Tool>, Box<dyn std::error::Error>> {
        self.client.list_all_tools().await.map_err(|e| e.into())
    }

    pub fn server_info(&self) -> Option<&rmcp::model::InitializeResult> {
        self.client.peer_info()
    }

    pub async fn execute_recipe(&self, recipe_name: &str) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        self.client
            .call_tool(CallToolRequestParam {
                name: "execute_recipe".into(),
                arguments: Some(object!({ "name": recipe_name })),
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn force_cleanup_orphaned_processes(&self) -> Result<rmcp::model::CallToolResult, Box<dyn std::error::Error>> {
        self.client
            .call_tool(CallToolRequestParam {
                name: "force_cleanup_orphaned_processes".into(),
                arguments: Some(object!({})),
            })
            .await
            .map_err(|e| e.into())
    }

    pub async fn cleanup(self) -> Result<(), Box<dyn std::error::Error>> {
        let _ = self.stop_all_drivers().await;
        let _ = self.client.cancel().await;
        Ok(())
    }
}

pub struct TestTimer {
    start: Instant,
}

impl TestTimer {
    pub fn new() -> Self {
        TestTimer {
            start: Instant::now(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_ms(&self) -> f64 {
        self.elapsed().as_secs_f64() * 1000.0
    }
}

pub fn check_tool_result_success(result: &rmcp::model::CallToolResult) -> bool {
    if let Some(content) = result.content.first() {
        let content_str = format!("{:?}", content.raw);
        !content_str.contains("\"isError\":true") && !content_str.contains("isError: true")
    } else {
        false
    }
}



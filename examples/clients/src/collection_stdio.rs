/// This example shows how to store multiple clients in a map and call tools on them.
/// into_dyn() is used to convert the service to a dynamic service.
/// This example creates multiple MCP client connections to local counter servers
/// and demonstrates calling tools on each of them concurrently.
use std::collections::HashMap;

use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    service::ServiceExt,
    transport::{ConfigureCommandExt, TokioChildProcess},
};
use tokio::process::Command;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("info,{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let mut clients_map = HashMap::new();
    for idx in 0..3 {  // Reduced from 10 to 3 for testing
        let client = ()
            .into_dyn()
            .serve(TokioChildProcess::new(Command::new("cargo").configure(
                |cmd| {
                    cmd.arg("run")
                        .arg("--example")
                        .arg("servers_counter_stdio");
                },
            ))?)
            .await?;
        clients_map.insert(idx, client);
    }

    for (_, client) in clients_map.iter() {
        // Initialize
        let _server_info = client.peer_info();

        // List tools
        let _tools = client.list_tools(Default::default()).await?;

        // Call tool 'get_value' (no arguments needed)
        let _tool_result = client
            .call_tool(CallToolRequestParam {
                name: "get_value".into(),
                arguments: None,
            })
            .await?;
    }
    for (_, service) in clients_map {
        service.cancel().await?;
    }
    Ok(())
}
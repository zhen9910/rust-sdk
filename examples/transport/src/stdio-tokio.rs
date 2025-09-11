//! # Stdio Tokio Transport Example
//!
//! This example demonstrates using stdio-like transport within the same process
//! using Tokio's duplex streams. Both the MCP server and client run as separate
//! Tokio tasks within the same process, communicating through in-memory duplex streams.
//!
//! Unlike the regular `stdio.rs` example which uses subprocess communication,
//! this example keeps everything in-process for simpler debugging and testing.
//!
//! ## Usage
//!
//! Run the demo:
//! ```bash
//! cargo run --example stdio-tokio
//! ```
//!
//! ## How it works
//!
//! 1. Creates two duplex streams using `tokio::io::duplex()`
//! 2. Spawns an MCP server task using one end of the duplex
//! 3. Creates an MCP client using the other end of the duplex
//! 4. Demonstrates typical MCP operations (list tools, call tools)
//! 5. Both tasks communicate through the in-memory duplex streams

mod common;

use common::calculator::Calculator;
use rmcp::{model::CallToolRequestParam, serve_client, serve_server};
use std::time::Duration;
use tokio::time::timeout;

fn create_tool_request(tool_name: &str, a: i32, b: i32) -> CallToolRequestParam {
    let mut args = serde_json::Map::new();
    args.insert("a".to_string(), serde_json::json!(a));
    args.insert("b".to_string(), serde_json::json!(b));

    CallToolRequestParam {
        name: tool_name.to_string().into(),
        arguments: Some(args),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for better debugging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    println!("🚀 Starting stdio-tokio transport demo...");
    println!("📡 Creating in-process duplex communication channels...");

    // Create duplex streams for bidirectional communication
    // Each duplex() call returns a pair of (DuplexStream, DuplexStream)
    // where data written to one end can be read from the other end
    let (server_stream, client_stream) = tokio::io::duplex(8192);

    println!("🖥️  Spawning MCP server task...");

    // Spawn the server task
    let server_handle = tokio::spawn(async move {
        match serve_server(Calculator::new(), server_stream).await {
            Ok(server) => {
                tracing::info!("✅ MCP server initialized successfully");
                
                // Wait for the server to handle requests
                if let Err(e) = server.waiting().await {
                    tracing::error!("❌ Server error while waiting: {}", e);
                } else {
                    tracing::info!("✅ MCP server completed successfully");
                }
            }
            Err(e) => {
                tracing::error!("❌ Failed to initialize MCP server: {}", e);
            }
        }
    });

    // Give the server a moment to initialize
    tokio::time::sleep(Duration::from_millis(100)).await;

    println!("🖥️  Creating MCP client...");

    // Create the client
    let client = serve_client((), client_stream).await?;

    println!("✅ Client and server connected!");

    // Demonstrate MCP operations
    println!("\n📋 Getting server information...");
    if let Some(server_info) = client.peer().peer_info() {
        println!("📄 Server info:");
        println!("{}", serde_json::to_string_pretty(server_info)?);
    } else {
        println!("⚠️  Server info not available yet");
    }

    println!("\n🔧 Listing available tools...");
    let tools = client.peer().list_tools(Default::default()).await?;
    println!("🛠️  Available tools:");
    println!("{}", serde_json::to_string_pretty(&tools)?);

    println!("\n🧮 Testing calculator tools...");

    // Test the sum tool
    println!("➕ Calling sum(5, 3)...");
    let sum_request = create_tool_request("sum", 5, 3);
    let sum_result = client.peer().call_tool(sum_request).await?;
    println!("📊 Sum result:");
    println!("{}", serde_json::to_string_pretty(&sum_result)?);

    // Test the sub tool
    println!("➖ Calling sub(10, 4)...");
    let sub_request = create_tool_request("sub", 10, 4);
    let sub_result = client.peer().call_tool(sub_request).await?;
    println!("📊 Subtraction result:");
    println!("{}", serde_json::to_string_pretty(&sub_result)?);

    // Test with larger numbers
    println!("🔢 Calling sum(1000, 2000)...");
    let large_sum_request = create_tool_request("sum", 1000, 2000);
    let large_sum_result = client.peer().call_tool(large_sum_request).await?;
    println!("📊 Large sum result:");
    println!("{}", serde_json::to_string_pretty(&large_sum_result)?);

    println!("\n🏁 Closing client connection...");
    
    // Close the client connection gracefully
    drop(client);

    println!("⏳ Waiting for server to finish...");

    // Wait for the server task to complete with a timeout
    match timeout(Duration::from_secs(5), server_handle).await {
        Ok(Ok(())) => {
            println!("✅ Server task completed successfully");
        }
        Ok(Err(e)) => {
            println!("❌ Server task failed: {}", e);
        }
        Err(_) => {
            println!("⚠️  Server task timed out");
        }
    }

    println!("\n🎉 stdio-tokio transport demo completed!");
    println!("💡 This demo showed in-process MCP communication using Tokio duplex streams");

    Ok(())
}

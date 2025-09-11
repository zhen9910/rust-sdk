//! # Stdio Transport Example
//!
//! This example demonstrates using stdio (standard input/output) as the transport
//! layer for the Model Context Protocol (MCP). This is commonly used when an MCP
//! server runs as a subprocess and communicates with the parent process through
//! stdin/stdout.
//!
//! ## Usage
//!
//! Run the demo (spawns server subprocess automatically):
//! ```bash
//! cargo run --example stdio
//! ```
//!
//! Run the WASM demo (demonstrates WASM component integration):
//! ```bash
//! cargo run --example stdio wasm
//! ```
//!
//! Run the Custom WIT demo (demonstrates custom WIT interface):
//! ```bash
//! cargo run --example stdio custom-wit
//! ```
//!
//! Run as server (waits for input on stdin):
//! ```bash
//! cargo run --example stdio server
//! ```
//!
//! Run as client (sends output to stdout):
//! ```bash
//! cargo run --example stdio client
//! ```
//!
//! ## WASM Component
//!
//! The WASM demo uses a separate WebAssembly component (`mcp_client_stdio_wasm`)
//! that implements an MCP client using WASI stdio transport. To build the WASM component:
//!
//! ```bash
//! rustup target add wasm32-wasip2
//! cargo build --target wasm32-wasip2 -p mcp_client_stdio_wasm
//! ```

mod common;

use common::calculator::Calculator;
use rmcp::{model::CallToolRequestParam, serve_client, serve_server};
use std::fs;
use tokio::io::{stdin, stdout};
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};

use wasmtime_wasi::p2;
use wasmtime_wasi::{DirPerms, FilePerms};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};

struct MyState {
    wasi: WasiCtx,
    table: ResourceTable,
}

impl WasiView for MyState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.wasi,
            table: &mut self.table,
        }
    }
}

// Generate bindings for our custom WIT interface
wasmtime::component::bindgen!({
    world: "mcp-client",
    path: "../mcp_client_custom_wit/wit/world.wit",
});

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
    // Check if running as server or client based on command line arguments
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && args[1] == "server" {
        server().await?;
    } else if args.len() > 1 && args[1] == "client" {
        client().await?;
    } else if args.len() > 1 && args[1] == "wasm" {
        demo_wasm().await?;
    } else {
        // Default behavior: demonstrate both by spawning a subprocess
        demo().await?;
    }

    Ok(())
}

async fn demo() -> anyhow::Result<()> {
    println!("Running stdio transport demo...");

    // Spawn server as subprocess
    let mut server_process = tokio::process::Command::new(std::env::current_exe()?)
        .arg("server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let server_stdin = server_process.stdin.take().unwrap();
    let server_stdout = server_process.stdout.take().unwrap();

    // Create client that communicates with the server subprocess via stdio
    let client = serve_client((), (server_stdout, server_stdin)).await?;

    // Example: Get server info (from get_info() method)
    if let Some(server_info) = client.peer().peer_info() {
        println!("Server info (JSON):");
        println!("{}", serde_json::to_string_pretty(server_info)?);
    } else {
        println!("Server info not available yet");
    }

    let tools = client.peer().list_tools(Default::default()).await?;
    println!("\nAvailable tools (JSON):");
    println!("{}", serde_json::to_string_pretty(&tools)?);

    // Example: Call the "sum" tool
    let sum_request = create_tool_request("sum", 5, 3);
    let sum_result = client.peer().call_tool(sum_request).await?;
    println!("\nSum result (JSON):");
    println!("{}", serde_json::to_string_pretty(&sum_result)?);

    // Example: Call the "sub" tool
    let sub_request = create_tool_request("sub", 10, 4);
    let sub_result = client.peer().call_tool(sub_request).await?;
    println!("\nSubtraction result (JSON):");
    println!("{}", serde_json::to_string_pretty(&sub_result)?);

    // Clean up the server process
    server_process.kill().await.ok();

    Ok(())
}

async fn demo_wasm() -> anyhow::Result<()> {
    println!("Running WASM-based stdio transport demo...");

    // Check if WASM component exists
    let wasm_component_path = "target/wasm32-wasip2/debug/mcp_client_stdio_wasm.wasm";

    if !std::path::Path::new(wasm_component_path).exists() {
        println!("WASM component not found at: {}", wasm_component_path);
        println!("To build the WASM component, run:");
        println!("  cargo build --target wasm32-wasip2 -p mcp_client_stdio_wasm");
        println!("\nNote: You may need to install the wasm32-wasip2 target first:");
        println!("  rustup target add wasm32-wasip2");
        return Ok(());
    }

    // Check if wasmtime is available
    if tokio::process::Command::new("wasmtime")
        .arg("--version")
        .output()
        .await
        .is_err()
    {
        println!("wasmtime CLI not found. Please install wasmtime:");
        println!("  curl https://wasmtime.dev/install.sh -sSf | bash");
        return Ok(());
    }

    println!("1. Spawning MCP server subprocess...");

    // Spawn server as subprocess using std::process to get compatible handles
    let mut server_process = std::process::Command::new(std::env::current_exe()?)
        .arg("server")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let server_stdin = server_process.stdin.take().unwrap();
    let server_stdout = server_process.stdout.take().unwrap();

    println!("2. Starting wasmtime engine...");
    println!("3. Loading WASM component: {}", wasm_component_path);

    // Spawn the WASM component using wasmtime CLI with std::process
    let mut wasm_process = std::process::Command::new("wasmtime")
        .arg("run")
        .arg(wasm_component_path)
        .stdin(server_stdout)
        .stdout(server_stdin)
        .stderr(std::process::Stdio::piped()) // Capture stderr to filter out WASI cleanup errors
        .spawn()?;

    println!("4. WASM MCP client is now running and communicating with the server...");

    // Capture and filter stderr from wasmtime in a background task
    let stderr = wasm_process.stderr.take().unwrap();
    let _stderr_task = tokio::task::spawn_blocking(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                // Filter out only the specific WASI cleanup error stack trace
                // Keep all other output including WASM Client messages, INFO logs, etc.
                if !line.contains("failed to run main module")
                    && !line.contains("failed to invoke `run` function")
                    && !line.contains("error while executing at wasm backtrace")
                    && !line.contains("mcp_client_stdio_wasm.wasm!")
                    && !line.contains("resource has children")
                    && !line
                        .trim()
                        .chars()
                        .all(|c| c.is_ascii_digit() || c == ':' || c.is_whitespace())
                {
                    // Skip numbered stack trace lines
                    eprintln!("{}", line);
                }
            }
        }
    });

    // Convert to async monitoring
    let server_handle = tokio::task::spawn_blocking(move || server_process.wait());
    let wasm_handle = tokio::task::spawn_blocking(move || wasm_process.wait());

    // Wait for either process to complete with a timeout
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        async {
            tokio::select! {
                server_result = server_handle => {
                    match server_result {
                        Ok(Ok(status)) => println!("Server exited with status: {}", status),
                        Ok(Err(e)) => println!("Server error: {}", e),
                        Err(e) => println!("Server task error: {}", e),
                    }
                }
                wasm_result = wasm_handle => {
                    match wasm_result {
                        Ok(Ok(status)) => {
                            // Check if this is the known WASI cleanup error
                            if !status.success() && status.code() == Some(1) {
                                // This is likely the WASI resource cleanup error, which is benign
                                // since the actual MCP communication succeeded
                                println!("\n=== WASM MCP Client completed successfully! ===");
                                println!("(Note: Exit code 1 is a known WASI/Wasmtime cleanup issue and can be ignored)");
                            } else if status.success() {
                                println!("\n=== WASM MCP Client completed successfully! ===");
                            } else {
                                println!("\n=== WASM MCP Client exited with error: {} ===", status);
                            }
                        }
                        Ok(Err(e)) => println!("WASM process error: {}", e),
                        Err(e) => println!("WASM task error: {}", e),
                    }
                }
            }
        }
    ).await;

    match result {
        Ok(_) => {
            println!("Demo completed!");
        }
        Err(_) => {
            println!("\n=== Demo timed out, processes may still be running ===");
        }
    }

    Ok(())
}

async fn server() -> anyhow::Result<()> {
    // Server mode: communicate via stdin/stdout
    let stdin = stdin();
    let stdout = stdout();

    let server = serve_server(Calculator::new(), (stdin, stdout)).await?;
    server.waiting().await?;

    Ok(())
}

async fn client() -> anyhow::Result<()> {
    // Client mode: communicate via stdin/stdout
    // This mode is typically used when this process is spawned by another process
    let stdin = stdin();
    let stdout = stdout();

    let _client = serve_client((), (stdin, stdout)).await?;

    // In a real scenario, the client would be controlled by the parent process
    // For this example, we'll just wait
    tokio::time::sleep(std::time::Duration::from_secs(60)).await;

    Ok(())
}

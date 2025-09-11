//! # Custom WIT-based MCP Client WASM Component
//!
//! This is a WebAssembly component that defines a custom WIT interface
//! for MCP client operations, allowing the host to have more control
//! over the component lifecycle and error handling.

use rmcp::{model::CallToolRequestParam, serve_client};
use std::task::{Poll, Waker};
use tokio::io::{AsyncRead, AsyncWrite};
use tracing_subscriber::EnvFilter;
use wasi::{
    cli::{
        stdin::{InputStream, get_stdin},
        stdout::{OutputStream, get_stdout},
    },
    io::streams::Pollable,
};

// Generate WIT bindings
wit_bindgen::generate!({
    world: "mcp-client",
    path: "wit",
});

use exports::example::mcp_client::mcp::Guest;

// Export the functions directly
export!(McpClient);

/// Implementation of the exported functions
struct McpClient;

impl Guest for McpClient {
    fn run() -> bool {
        // Initialize tracing
        if tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
            )
            .with_writer(std::io::stderr)
            .with_ansi(false)
            .try_init()
            .is_err()
        {
            // Tracing already initialized, continue
        }

        println!("USE println! in WASM component for logging    (to be captured by host)");
        // Create Tokio runtime
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => {
                tracing::error!("Failed to create Tokio runtime: {}", e);
                return false;
            }
        };

        // Run the MCP client
        rt.block_on(async move {
            match run_mcp_client().await {
                Ok(()) => {
                    tracing::info!("Custom WIT MCP Client completed successfully!");
                    true
                }
                Err(e) => {
                    tracing::error!("MCP client failed: {}", e);
                    false
                }
            }
        })
    }

    fn version() -> String {
        "0.6.1-custom-wit".to_string()
    }
}

// // Also provide a CLI runner for compatibility with wasmtime run without --invoke
// struct CliRunner;

// impl wasi::exports::cli::run::Guest for CliRunner {
//     fn run() -> Result<(), ()> {
//         // Just call our custom run function
//         let success = McpClient::run();
//         if success {
//             Ok(())
//         } else {
//             Err(())
//         }
//     }
// }

// // Export both interfaces
// wasi::cli::command::export!(CliRunner);

pub fn wasi_io() -> (AsyncInputStream, AsyncOutputStream) {
    let input = AsyncInputStream { inner: get_stdin() };
    let output = AsyncOutputStream {
        inner: get_stdout(),
    };
    (input, output)
}

pub struct AsyncInputStream {
    inner: InputStream,
}

impl AsyncRead for AsyncInputStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let bytes = self
            .inner
            .read(buf.remaining() as u64)
            .map_err(std::io::Error::other)?;
        if bytes.is_empty() {
            let pollable = self.inner.subscribe();
            let waker = cx.waker().clone();
            runtime_poll(waker, pollable);
            return Poll::Pending;
        }
        buf.put_slice(&bytes);
        std::task::Poll::Ready(Ok(()))
    }
}

pub struct AsyncOutputStream {
    inner: OutputStream,
}

fn runtime_poll(waker: Waker, pollable: Pollable) {
    tokio::task::spawn(async move {
        loop {
            if pollable.ready() {
                waker.wake();
                break;
            } else {
                tokio::task::yield_now().await;
            }
        }
    });
}

impl AsyncWrite for AsyncOutputStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        let writable_len = self.inner.check_write().map_err(std::io::Error::other)?;
        if writable_len == 0 {
            let pollable = self.inner.subscribe();
            let waker = cx.waker().clone();
            runtime_poll(waker, pollable);
            return Poll::Pending;
        }
        let bytes_to_write = buf.len().min(writable_len as usize);
        self.inner
            .write(&buf[0..bytes_to_write])
            .map_err(std::io::Error::other)?;
        Poll::Ready(Ok(bytes_to_write))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.inner.flush().map_err(std::io::Error::other)?;
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        self.poll_flush(cx)
    }
}

fn create_tool_request(tool_name: &str, a: i32, b: i32) -> CallToolRequestParam {
    let mut args = serde_json::Map::new();
    args.insert("a".to_string(), serde_json::json!(a));
    args.insert("b".to_string(), serde_json::json!(b));

    CallToolRequestParam {
        name: tool_name.to_string().into(),
        arguments: Some(args),
    }
}

async fn run_mcp_client() -> anyhow::Result<()> {
    tracing::info!("Custom WIT MCP Client starting...");

    // Get WASI stdin/stdout for MCP communication
    let (stdin, stdout) = wasi_io();

    // Create MCP client using WASI stdio
    let client = serve_client((), (stdin, stdout)).await?;

    tracing::info!("Custom WIT Client: Connected to MCP server via WASI stdio");

    // Get server info
    if let Some(server_info) = client.peer().peer_info() {
        tracing::info!("Custom WIT Client: Server info received");
        tracing::info!(
            "Server info (JSON): {}",
            serde_json::to_string_pretty(server_info)?
        );
    } else {
        tracing::warn!("Custom WIT Client: Server info not available yet");
    }

    // List available tools
    tracing::info!("Custom WIT Client: Requesting available tools");
    let tools = client.peer().list_tools(Default::default()).await?;
    tracing::info!(
        "Custom WIT Client: Available tools: {}",
        serde_json::to_string_pretty(&tools)?
    );

    // Call the "sum" tool if available
    if tools.tools.iter().any(|t| t.name == "sum") {
        tracing::info!("Custom WIT Client: Calling sum tool");
        let sum_request = create_tool_request("sum", 12, 8);
        let sum_result = client.peer().call_tool(sum_request).await?;
        tracing::info!(
            "Custom WIT Client: Sum result: {}",
            serde_json::to_string_pretty(&sum_result)?
        );
    }

    // Call the "sub" tool if available
    if tools.tools.iter().any(|t| t.name == "sub") {
        tracing::info!("Custom WIT Client: Calling sub tool");
        let sub_request = create_tool_request("sub", 20, 5);
        let sub_result = client.peer().call_tool(sub_request).await?;
        tracing::info!(
            "Custom WIT Client: Subtraction result: {}",
            serde_json::to_string_pretty(&sub_result)?
        );
    }

    tracing::info!("Custom WIT Client: Demo completed successfully!");

    Ok(())
}

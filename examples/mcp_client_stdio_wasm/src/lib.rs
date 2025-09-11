//! # WASM MCP Client with WASI Stdio Transport
//!
//! This is a WebAssembly component that acts as an MCP client using WASI stdio
//! for communication. It follows the same pattern as the existing WASI server example
//! but implements a client instead.

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
    tracing::info!("WASM MCP Client starting...");

    // Get WASI stdin/stdout for MCP communication
    let (stdin, stdout) = wasi_io();

    // Create MCP client using WASI stdio
    let client = serve_client((), (stdin, stdout)).await?;

    tracing::info!("WASM Client: Connected to MCP server via WASI stdio");

    // Get server info
    if let Some(server_info) = client.peer().peer_info() {
        tracing::info!("WASM Client: Server info received");
        tracing::info!(
            "Server info (JSON): {}",
            serde_json::to_string_pretty(server_info)?
        );
    } else {
        tracing::warn!("WASM Client: Server info not available yet");
    }

    // List available tools
    tracing::info!("WASM Client: Requesting available tools");
    let tools = client.peer().list_tools(Default::default()).await?;
    tracing::info!(
        "WASM Client: Available tools: {}",
        serde_json::to_string_pretty(&tools)?
    );

    // Call the "sum" tool if available
    if tools.tools.iter().any(|t| t.name == "sum") {
        tracing::info!("WASM Client: Calling sum tool");
        let sum_request = create_tool_request("sum", 7, 3);
        let sum_result = client.peer().call_tool(sum_request).await?;
        tracing::info!(
            "WASM Client: Sum result: {}",
            serde_json::to_string_pretty(&sum_result)?
        );
    }

    // Call the "sub" tool if available
    if tools.tools.iter().any(|t| t.name == "sub") {
        tracing::info!("WASM Client: Calling sub tool");
        let sub_request = create_tool_request("sub", 15, 8);
        let sub_result = client.peer().call_tool(sub_request).await?;
        tracing::info!(
            "WASM Client: Subtraction result: {}",
            serde_json::to_string_pretty(&sub_result)?
        );
    }

    println!("[println!]WASM Client: Demo completed successfully!");
    tracing::info!("WASM Client: Demo completed successfully!");

    Ok(())
}

struct TokioCliRunner;

impl wasi::exports::cli::run::Guest for TokioCliRunner {
    fn run() -> Result<(), ()> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            tracing_subscriber::fmt()
                .with_env_filter(
                    EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()),
                )
                .with_writer(std::io::stderr)
                .with_ansi(false)
                .init();

            if let Err(e) = run_mcp_client().await {
                tracing::error!("MCP client error: {}", e);
                return;
            }
        });
        Ok(())
    }
}

wasi::cli::command::export!(TokioCliRunner);

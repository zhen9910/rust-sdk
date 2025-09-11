mod common;

use common::calculator::Calculator;
use rmcp::serve_server;
use std::fs;
use wasmtime::{
    Config, Engine, Store,
    component::{Component, Linker},
};
use wasmtime_wasi::p2;
use wasmtime_wasi::{
    cli::{AsyncStdinStream, AsyncStdoutStream},
};
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    demo_custom_wit().await
}

async fn demo_custom_wit() -> anyhow::Result<()> {
    println!("Running Custom WIT-based WASM stdio transport demo with Rust API...");

    // Path to the custom WIT WASM component
    let wasm_component_path = "target/wasm32-wasip2/debug/mcp_client_custom_wit.wasm";

    // Check if WASM component exists
    if !std::path::Path::new(wasm_component_path).exists() {
        println!("WASM component not found at: {}", wasm_component_path);
        println!("Please build it first:");
        println!("  cargo build --target wasm32-wasip2 --package mcp_client_custom_wit");
        return Ok(());
    }

    println!("1. Creating communication channels...");

    // Create bidirectional communication channels
    // Channel 1: Host -> Server (server's stdin)
    let (host_to_server_tx, host_to_server_rx) = tokio::io::duplex(8192);
    
    // Channel 2: Server -> Host (server's stdout) 
    let (server_to_host_tx, server_to_host_rx) = tokio::io::duplex(8192);

    println!("2. Starting MCP server as tokio task...");
    
    // Start the server as a tokio task with the channels
    let _server_handle = tokio::spawn(async move {
        if let Err(e) = server(host_to_server_rx, server_to_host_tx).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Now we have:
    // - host_to_server_tx: Write to this to send data TO the server
    // - server_to_host_rx: Read from this to get data FROM the server
    
    println!("3. Creating WASI streams connected to server channels...");
    
    // Create WASI streams that connect to our server
    // WASM client will read from server's output and write to server's input
    let wasi_stdin = AsyncStdinStream::new(server_to_host_rx);   // WASM reads from server
    let wasi_stdout = AsyncStdoutStream::new(4096, host_to_server_tx); // WASM writes to server
    let wasi_stderr = wasmtime_wasi::cli::stderr(); // Use default stderr

    println!("4. Loading component with Wasmtime Rust API...");

    // Load component bytes following the requested pattern
    let component_bytes = fs::read(wasm_component_path)?;
    println!("📦 Loaded component: {} bytes", component_bytes.len());

    let mut config = Config::new();
    config.wasm_component_model(true);
    config.debug_info(true); // Enable debug info
    config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Enable);
    config.cranelift_debug_verifier(true); // Enable Cranelift verification
    config.cranelift_opt_level(wasmtime::OptLevel::None);
    // config.async_support(true); // Enable async support for WASI streams

    let engine = Engine::new(&config)?;
    println!("🚀 Engine created with debug configuration");

    let component = Component::new(&engine, &component_bytes)?;
    println!("🧩 Component instantiated successfully");

    // let linker = Linker::new(&engine);
    let mut linker = Linker::<MyState>::new(&engine);
    println!("🔗 Linker created");
    // Add the entire WASI API (Preview-2) to the linker (async variant for async streams)
    p2::add_to_linker_sync(&mut linker)?;

    // use wasmtime_wasi::cli::stdin;
    // let i = stdin();
    // Configure WasiCtx with the communication channels
    // The WASM client will communicate with the server via these streams:
    // - wasi_stdin: WASM reads from this (connected to server's output)
    // - wasi_stdout: WASM writes to this (connected to server's input)
    let wasi_ctx = WasiCtxBuilder::new()
        .stdin(wasi_stdin)       // WASM reads from server via this
        .stdout(wasi_stdout)     // WASM writes to server via this
        .stderr(wasi_stderr)     // WASM stderr goes to host stderr
        .inherit_env()
        .preopened_dir(
            ".",              // host dir
            ".",              // guest path
            DirPerms::all(),  // allow all dir operations
            FilePerms::all(), // allow all file operations
        )?
        .build();

    // let mut store = Store::new(&engine, ());
    // Initialize your store with WasiCtx and ResourceTable
    let mut store = Store::new(
        &engine,
        MyState {
            wasi: wasi_ctx,
            table: ResourceTable::new(),
        },
    );

    // let bindings = McpClient::instantiate_async(&mut store, &component, &linker).await?;
    let bindings = McpClient::instantiate(&mut store, &component, &linker)?;
    println!("⚡ Component bindings established");

    println!("3. Calling exported custom WIT functions directly...");

    // Call the version() function from our custom interface
    match bindings.example_mcp_client_mcp().call_version(&mut store) {
        Ok(version) => {
            println!("   📋 Component version: {}", version);
        }
        Err(e) => {
            println!("   ❌ Failed to get version: {}", e);
        }
    }

    // Call the run() function from our custom interface
    println!("   🏃 Calling run() function...");
    let result = bindings.example_mcp_client_mcp().call_run(&mut store)?;
    println!("   🏁 Run result: {}", result);
    // match bindings.call_run(&mut store) {
    //     Ok(success) => {
    //         println!("   ✅ Run completed successfully: {}", success);
    //         if success {
    //             println!("   🎉 Custom WIT function returned true - operations succeeded!");
    //         }
    //     }
    //     Err(e) => {
    //         println!("   ❌ Run failed: {}", e);
    //     }
    // }

    // Clean up: The server task will terminate when the channels are dropped
    // _server_handle.abort(); // Uncomment to forcefully terminate server if needed
    
    println!("🎯 Demo completed - server is running in background tokio task");

    Ok(())
}

async fn server<I, O>(stdin: I, stdout: O) -> anyhow::Result<()> 
where
    I: tokio::io::AsyncRead + Unpin + Send + 'static,
    O: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    // Server mode: communicate via provided stdin/stdout
    let server = serve_server(Calculator::new(), (stdin, stdout)).await?;
    server.waiting().await?;

    Ok(())
}

# WASM MCP Client with WASI Stdio Transport

This is a WebAssembly component that acts as an MCP client using WASI stdio for communication. It's designed to be loaded by a Wasmtime host that provides MCP server communication through stdin/stdout.

## Building

To build the WASM component:

```bash
# Install the wasm32-wasip2 target if not already installed
rustup target add wasm32-wasip2

# Build the WASM component
cargo build --target wasm32-wasip2 -p mcp_client_stdio_wasm
```

The resulting WASM component will be located at:
`target/wasm32-wasip2/debug/mcp_client_stdio_wasm.wasm`

## Usage

This WASM component can be loaded by the stdio transport example:

```bash
cargo run --example stdio wasm
```

## Features

- Connects to an MCP server via WASI stdio
- Lists available tools from the server
- Calls tools (sum and sub) if available
- Demonstrates WASM component integration with MCP protocol

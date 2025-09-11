# Custom WIT-based MCP Client WASM Component

This example demonstrates how to create a WebAssembly component with a custom WIT (WebAssembly Interface Types) interface for MCP client operations.

## Overview

Unlike the standard `mcp_client_stdio_wasm` example that uses `wasi::exports::cli::run()`, this example:

1. **Defines a custom WIT interface** in `wit/world.wit`
2. **Exports custom functions** that can be called by the host
3. **Provides better error handling** with structured return types
4. **Allows more control** over the component lifecycle

## WIT Interface

The component exports two functions:

- `run() -> mcp-result` - Runs the MCP client demo and returns success/error
- `version() -> string` - Returns the client version

## Custom Interface Definition

```wit
interface mcp-client {
    variant mcp-result {
        success,
        error(string),
    }
    
    run: func() -> mcp-result;
    version: func() -> string;
}
```

## Building

```bash
# Build the WASM component
cargo build --target wasm32-wasip2 --package mcp_client_custom_wit
```

## Host Integration

A Wasmtime-based host application can load this component and call the exported functions:

```rust
// Load and instantiate the component
let component = Component::from_file(&engine, "target/wasm32-wasip2/debug/mcp_client_custom_wit.wasm")?;
let instance = linker.instantiate(&mut store, &component)?;

// Get the exported interface
let interface = instance.get_typed_func::<(), (McpResult,)>(&mut store, "run")?;

// Call the run function
let (result,) = interface.call(&mut store, ())?;
match result {
    McpResult::Success => println!("MCP client completed successfully!"),
    McpResult::Error(msg) => println!("MCP client failed: {}", msg),
}
```

## Advantages over Standard CLI Export

1. **Better Error Handling**: Returns structured error information instead of exit codes
2. **Custom Interface**: Define exactly what functions you want to export
3. **Host Control**: Host can handle errors and results programmatically
4. **Version Information**: Export additional metadata like version info
5. **Future Extensibility**: Easy to add new exported functions

## Usage in Host Application

The host application needs to:

1. Set up stdio pipes for MCP communication
2. Load and instantiate the WASM component
3. Call the exported functions
4. Handle the structured return values

This provides much more flexibility than the standard WASI CLI interface for embedded WASM components in larger applications.

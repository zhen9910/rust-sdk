# Transport Examples

This directory contains examples demonstrating different transport methods for the Model Context Protocol (MCP) in Rust.

## Available Examples

### Network Transports

- **[tcp.rs](src/tcp.rs)** - TCP socket transport
  ```bash
  cargo run --example tcp
  ```

- **[unix_socket.rs](src/unix_socket.rs)** - Unix domain socket transport (Unix systems only)
  ```bash
  cargo run --example unix_socket
  ```

- **[websocket.rs](src/websocket.rs)** - WebSocket transport
  ```bash
  cargo run --example websocket
  ```

- **[http_upgrade.rs](src/http_upgrade.rs)** - HTTP upgrade transport
  ```bash
  cargo run --example http_upgrade
  ```

- **[named-pipe.rs](src/named-pipe.rs)** - Named pipe transport (Windows only)
  ```bash
  cargo run --example named-pipe
  ```

### Process Communication

- **[stdio.rs](src/stdio.rs)** - Standard input/output transport with subprocess
  ```bash
  cargo run --example stdio
  cargo run --example stdio wasm        # WASM component demo
  cargo run --example stdio custom-wit  # Custom WIT demo
  cargo run --example stdio server      # Run as server
  cargo run --example stdio client      # Run as client
  ```

- **[stdio-tokio.rs](src/stdio-tokio.rs)** - ⭐ **NEW!** In-process stdio-like transport using Tokio tasks
  ```bash
  cargo run --example stdio-tokio
  ```

- **[stdio-mcp.rs](src/stdio-mcp.rs)** - MCP-specific stdio transport
  ```bash
  cargo run --example stdio-mcp
  ```

## Key Differences

### stdio.rs vs stdio-tokio.rs

| Feature | stdio.rs | stdio-tokio.rs |
|---------|----------|----------------|
| **Process Model** | Subprocess communication | In-process tasks |
| **Transport** | Real stdin/stdout pipes | Tokio duplex streams |
| **Complexity** | Process management required | Simple task spawning |
| **Debugging** | Cross-process debugging | Single-process debugging |
| **Performance** | Process overhead | In-memory communication |
| **Use Case** | Production MCP servers | Testing, prototyping |

### When to Use Each

- **Use `stdio.rs`** when:
  - Building production MCP servers
  - Need process isolation
  - Interfacing with external systems
  - Following standard MCP deployment patterns

- **Use `stdio-tokio.rs`** when:
  - Testing MCP implementations
  - Prototyping new features
  - Learning MCP concepts
  - Need simpler debugging
  - Building embedded MCP systems

## Common Calculator Service

All examples use a shared `Calculator` service from `src/common/calculator.rs` that provides:

- **sum(a, b)** - Add two numbers
- **sub(a, b)** - Subtract two numbers

This allows you to compare how different transports handle the same MCP operations.

## Running Examples

Each example can be run independently:

```bash
# From the rust-sdk root directory
cargo run --example <example-name>
```

For examples with multiple modes, check the individual file documentation for usage instructions.

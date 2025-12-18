# Rust gRPC C2 System (Educational)

A simple Command & Control (C2) system implementation in Rust using gRPC, designed for educational purposes. This project demonstrates patterns used in enterprise management tools like SCCM, Jamf, Puppet, and remote administration systems.

## 📚 Educational Purpose

This project showcases:
- **gRPC communication** patterns (client-server, bidirectional)
- **Channel-based architecture** using Tokio's mpsc channels
- **Polling vs Push** communication patterns
- **Async/await** Rust programming
- **Protocol Buffers** for service definitions
- **gRPC Reflection** for runtime service discovery

## 🏗️ Architecture

```
┌─────────────┐         ┌──────────────────┐         ┌─────────────┐
│   Admin     │         │      Server      │         │   Implant   │
│   Client    │         │   (Two Ports)    │         │   Client    │
│  (port 9090)│         │                  │         │ (port 4444) │
└──────┬──────┘         └────┬────────┬────┘         └──────┬──────┘
       │                     │        │                     │
       │  RunCommand(cmd)    │        │                     │
       ├────────────────────>│        │                     │
       │                 work_tx -> work_rx                 │
       │                     │        │  FetchCommand()     │
       │                     │        │<────────────────────┤
       │                     │        │  (cmd)              │
       │                     │        ├────────────────────>│
       │                     │        │     [executes]      │
       │                     │        │  SendOutput(result) │
       │                     │        │<────────────────────┤
       │                output_tx <- output_rx              │
       │  (result)           │        │                     │
       │<────────────────────┤        │                     │
       │                     │        │                     │
```

### Components

1. **Server** (`src/bin/server.rs`)
   - Runs two gRPC servers on different ports
   - **Implant Server (4444)**: Handles implant connections
   - **Admin Server (9090)**: Handles admin connections
   - Uses Tokio channels to route commands between admin and implants

2. **Implant** (`src/bin/implant.rs`)
   - Remote agent that polls the server every 3 seconds
   - Executes shell commands and returns output
   - Works behind NAT/firewalls (client-initiated connections)

3. **Admin** (`src/bin/admin.rs`)
   - CLI tool for sending commands to implants
   - Waits for command execution results
   - Simple request-response pattern

## 🚀 Quick Start

### Prerequisites

- Rust 1.75+ (2024 edition)
- `grpcurl` (optional, for CLI testing with reflection)
- `grpcui` (optional, for web-based GUI testing)

```bash
# Install grpcurl (optional, for CLI testing)
# macOS
brew install grpcurl

# Linux
# Download from: https://github.com/fullstorydev/grpcurl/releases

# Install grpcui (optional, for web GUI testing)
# macOS
brew install grpcui

# Linux / macOS (with Go installed)
go install github.com/fullstorydev/grpcui/cmd/grpcui@latest

# Or download from: https://github.com/fullstorydev/grpcui/releases
```

### Build

```bash
# Build all binaries
cargo build --release

# Binaries will be in target/release/
# - server
# - implant
# - admin
```

### Run the System

**Terminal 1 - Start the Server:**
```bash
cargo run --bin server

# Or use the release binary
./target/release/server
```

You should see:
```
Starting gRPC servers...
Implant server listening on 0.0.0.0:4444 (with reflection)
Admin server listening on 0.0.0.0:9090 (with reflection)

Test with:
  grpcurl -plaintext localhost:4444 list
  grpcurl -plaintext localhost:9090 list
```

**Terminal 2 - Start the Implant:**
```bash
cargo run --bin implant

# Or use the release binary
./target/release/implant
```

You should see:
```
Implant starting...
Connecting to server at http://localhost:4444
Connected! Polling for commands every 3 seconds...
```

**Terminal 3 - Send Commands:**
```bash
# Basic commands
cargo run --bin admin "whoami"
cargo run --bin admin "ls -la"
cargo run --bin admin "echo hello world"

# Or use the release binary
./target/release/admin "pwd"
```

Output:
```
Admin client starting...
Connecting to server at http://localhost:9090
Sending command: whoami
Waiting for implant to execute command...

=== Command Result ===
Input: whoami
Output:
username
```

## 🔧 Configuration

### Environment Variables

**Server:**
- Default ports are hardcoded (4444 for implants, 9090 for admin)

**Implant:**
```bash
# Connect to a different server
GRPC_SERVER_URL=http://10.0.0.5:4444 cargo run --bin implant
```

**Admin:**
```bash
# Connect to a different admin server
GRPC_SERVER_URL=http://10.0.0.5:9090 cargo run --bin admin "ls"
```

## 🔍 Testing with gRPC Reflection

The server includes gRPC reflection, allowing you to explore and test the API without proto files.

### List Available Services

```bash
# List services on implant server (port 4444)
grpcurl -plaintext localhost:4444 list
```

Output:
```
grpc.reflection.v1.ServerReflection
implant.Admin
implant.Implant
```

```bash
# List services on admin server (port 9090)
grpcurl -plaintext localhost:9090 list
```

Output:
```
grpc.reflection.v1.ServerReflection
implant.Admin
```

### Describe Services

```bash
# Describe the Implant service
grpcurl -plaintext localhost:4444 describe implant.Implant
```

Output:
```
implant.Implant is a service:
service Implant {
  rpc FetchCommand ( .implant.Empty ) returns ( .implant.Command );
  rpc SendOutput ( .implant.Command ) returns ( .implant.Empty );
}
```

```bash
# Describe the Admin service
grpcurl -plaintext localhost:9090 describe implant.Admin
```

Output:
```
implant.Admin is a service:
service Admin {
  rpc RunCommand ( .implant.Command ) returns ( .implant.Command );
}
```

### Describe Messages

```bash
# Describe the Command message
grpcurl -plaintext localhost:4444 describe implant.Command
```

Output:
```
implant.Command is a message:
message Command {
  string inp = 1;
  string out = 2;
}
```

### Call RPCs Directly

**Fetch Command (as an implant):**
```bash
# Poll for commands (returns empty if no work)
grpcurl -plaintext -d '{}' localhost:4444 implant.Implant/FetchCommand
```

**Send a Command (as admin):**
```bash
# Send a command and wait for result
grpcurl -plaintext -d '{"Inp": "whoami"}' localhost:9090 implant.Admin/RunCommand
```

This will block until an implant polls, executes the command, and returns the result.

**Send Output (as implant):**
```bash
# Send command result back
grpcurl -plaintext -d '{"Inp": "whoami", "Out": "root"}' localhost:4444 implant.Implant/SendOutput
```

## 🖥️ Testing with grpcui (Web GUI)

`grpcui` provides a web-based graphical interface for testing gRPC services. It's perfect for visual exploration and interactive testing.

### Launch grpcui

**For Implant Server (port 4444):**
```bash
grpcui -plaintext localhost:4444
```

Output:
```
gRPC Web UI available at http://127.0.0.1:60551/
```

**For Admin Server (port 9090):**
```bash
grpcui -plaintext localhost:9090
```

Your browser will automatically open to the grpcui interface.

### Using the Web Interface

1. **Select a Service**
   - In the dropdown, you'll see `implant.Implant` or `implant.Admin`
   - Select the service you want to test

2. **Select a Method**
   - Choose from available RPC methods:
     - `FetchCommand` (for implant server)
     - `SendOutput` (for implant server)
     - `RunCommand` (for admin server)

3. **Fill in Request Data**
   - The interface shows a JSON editor for the request
   - For example, to test `RunCommand`:
     ```json
     {
       "Inp": "ls -la",
       "Out": ""
     }
     ```

4. **Click "Invoke"**
   - The request is sent to the server
   - Response appears in the "Response Data" section

### Example Workflows in grpcui

**Test as Admin (port 9090):**

1. Open grpcui: `grpcui -plaintext localhost:9090`
2. Select service: `implant.Admin`
3. Select method: `RunCommand`
4. Enter request:
   ```json
   {
     "Inp": "whoami",
     "Out": ""
   }
   ```
5. Click "Invoke"
6. Wait for implant to execute (this will block until an implant responds)
7. See the response:
   ```json
   {
     "Inp": "whoami",
     "Out": "username\n"
   }
   ```

**Test as Implant (port 4444):**

1. Open grpcui: `grpcui -plaintext localhost:4444`
2. Select service: `implant.Implant`
3. Select method: `FetchCommand`
4. Enter empty request: `{}`
5. Click "Invoke"
6. If no commands queued:
   ```json
   {
     "Inp": "",
     "Out": ""
   }
   ```
7. If command available:
   ```json
   {
     "Inp": "ls -la",
     "Out": ""
   }
   ```

### grpcui Features

- **📋 Request History**: View all previous requests
- **🔄 Auto-complete**: JSON fields auto-complete based on proto
- **📝 Pretty Print**: Responses are formatted nicely
- **🎨 Syntax Highlighting**: JSON syntax highlighting
- **⏱️ Timing Info**: See request duration
- **🔍 Service Discovery**: Automatic via gRPC reflection

### grpcui vs grpcurl

| Feature | grpcurl | grpcui |
|---------|---------|--------|
| Interface | Command-line | Web browser |
| Speed | Very fast | Slightly slower (opens browser) |
| Scripting | Easy to script | Not scriptable |
| Learning | Steeper curve | Very intuitive |
| CI/CD | Perfect for automation | Not suitable |
| Exploration | Good | Excellent |

**Use grpcurl when:**
- Writing automated tests
- Scripting in CI/CD pipelines
- Quick one-off commands
- Working in terminal-only environments

**Use grpcui when:**
- Learning the API
- Manual testing with complex data
- Demonstrating to others
- Exploring service capabilities
- You prefer visual interfaces

## 📖 Protocol Buffer Definition

The API is defined in `proto/implant.proto`:

```protobuf
service Implant {
  rpc FetchCommand (Empty) returns (Command);
  rpc SendOutput (Command) returns (Empty);
}

service Admin {
  rpc RunCommand (Command) returns (Command);
}

message Command {
  string inp = 1;  // Input command
  string out = 2;  // Output result
}

message Empty {}
```

## 🧪 Testing the Full Flow

1. **Start the server** in Terminal 1
2. **Start an implant** in Terminal 2
3. **In Terminal 3**, send a command:
   ```bash
   ./target/release/admin "date"
   ```

Watch the flow:
- **Terminal 3 (Admin)**: Sends command, waits...
- **Terminal 1 (Server)**: Routes command through channels
- **Terminal 2 (Implant)**: Receives command, executes, sends output
- **Terminal 1 (Server)**: Routes output back
- **Terminal 3 (Admin)**: Displays result

## 🔐 Security Notes

⚠️ **This is for educational purposes only!**

In production systems, you should implement:

- **Authentication**: Verify implant and admin identities (mTLS, JWT tokens)
- **Authorization**: Control who can execute which commands (RBAC)
- **Encryption**: Use TLS for all connections (not plaintext HTTP/2)
- **Command Validation**: Whitelist allowed commands, sanitize inputs
- **Audit Logging**: Log all commands and results
- **Rate Limiting**: Prevent DoS attacks
- **Timeouts**: Add timeouts to prevent indefinite waits
- **Command IDs**: Match requests/responses for multiple implants/admins
- **Bounded Channels**: Prevent memory exhaustion

## 📚 Learning Resources

This project demonstrates several advanced Rust concepts:

### Tokio Channels
- **Unbounded channels** for work queue and output queue
- **Arc<Mutex<>>** for sharing receivers across async tasks
- **Clone-able senders** for multiple producers

See: `src/bin/server.rs` lines 51-62

### gRPC with Tonic
- **Service traits** generated from `.proto` files
- **Client and server** implementations
- **Async/await** patterns
- **gRPC reflection** for runtime discovery

See: `src/implant/service.rs` and `src/admin/service.rs`

### Build Scripts
- **Proto compilation** at build time
- **Code generation** from `.proto` to Rust
- **File descriptors** for reflection

See: `build.rs`

## 🛠️ Development

### Project Structure

```
grpc-rs/
├── proto/
│   └── implant.proto          # gRPC service definitions
├── src/
│   ├── admin/
│   │   ├── mod.rs
│   │   └── service.rs         # Admin service implementation
│   ├── implant/
│   │   ├── mod.rs
│   │   └── service.rs         # Implant service implementation
│   ├── bin/
│   │   ├── admin.rs           # Admin CLI client
│   │   ├── implant.rs         # Implant agent
│   │   └── server.rs          # Central C2 server
│   └── lib.rs                 # Library exports
├── build.rs                   # Proto compilation
├── Cargo.toml
└── README.md
```

### Rebuild Proto Files

Proto files are automatically compiled by `build.rs` when you run:

```bash
cargo build
```

To force a rebuild:

```bash
cargo clean
cargo build
```

### Run Tests

```bash
cargo test
```

### Code Documentation

Generate and view code documentation:

```bash
cargo doc --open
```

All code includes comprehensive comments explaining:
- Why architectural decisions were made
- How channels flow data
- When to use blocking vs non-blocking operations
- Production considerations

## 🤝 Contributing

This is an educational project. If you find issues or want to improve the documentation, feel free to:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Submit a pull request

## 📄 License

This project is licensed under the **BSD 3-Clause License** - see the [LICENSE](LICENSE) file for details.

### BSD 3-Clause License

```
BSD 3-Clause License

Copyright (c) 2025, Valentin
```

**What this means:**
- ✅ Free to use for any purpose (commercial or personal)
- ✅ Free to modify and distribute
- ✅ Free to use in proprietary software
- ✅ Can be included in closed-source projects
- ⚠️ Must include the copyright notice and license in redistributions
- ⚠️ Cannot use the author's name to endorse derived products without permission
- ⚠️ No warranty or liability from the author

For the full license text, see the [LICENSE](LICENSE) file.

## 🙏 Acknowledgments

This project was inspired by:
- Enterprise management tools (SCCM, Jamf, Puppet)
- gRPC examples from the Tonic documentation
- Command & Control patterns from security research (ethical/defensive use only)

---

**Educational Disclaimer**: This project is designed for learning about distributed systems, gRPC, and Rust async programming. Do not use for malicious purposes. Always obtain proper authorization before deploying on systems you don't own.

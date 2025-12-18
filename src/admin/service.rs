// AdminService - Server-side handler for admin client connections
//
// ARCHITECTURE OVERVIEW:
// This service allows admin clients to send commands to implants and receive results.
// It acts as a coordinator between the admin client and implant by using two channels.
//
// CHANNEL FLOW:
// Admin (this service) -> work_tx -> work_rx -> ImplantService
// ImplantService -> output_tx -> output_rx -> Admin (this service)
//
// This creates a full request-response cycle:
// 1. Admin sends command via work_tx
// 2. Implant fetches command from work_rx (polling every 3 seconds)
// 3. Implant executes command and sends result to output_tx
// 4. Admin receives result from output_rx

use crate::proto::admin_server::Admin;
use crate::proto::Command;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tonic::{Request, Response, Status};

/// AdminService handles gRPC requests from admin clients
///
/// FIELDS:
/// - work_tx: Sends commands to implants (via work channel)
///   - Clone-able sender, so multiple admin requests can send commands
///   - UnboundedSender means we never block when sending commands
///
/// - output_rx: Receives command results from implants (via output channel)
///   - Wrapped in Arc<Mutex<>> because we need to share it across requests
///   - Each RunCommand call locks it while waiting for a response
///   - IMPORTANT: This design assumes one admin at a time. For multiple admins,
///     you'd need a more sophisticated routing system (e.g., command IDs)
#[derive(Debug, Clone)]
pub struct AdminService {
    pub work_tx: mpsc::UnboundedSender<Command>,
    pub output_rx: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
}

// Implement the Admin trait (generated from proto/implant.proto)
// This trait defines the RPC methods that admin clients can call
#[tonic::async_trait]
impl Admin for AdminService {
    /// RunCommand is called by the admin client to execute a command on an implant
    ///
    /// FLOW:
    /// 1. Admin client calls this RPC with a command (e.g., "ls -la")
    /// 2. We send the command to work_tx channel
    /// 3. We BLOCK waiting on output_rx for the result
    /// 4. Implant polls, gets command, executes it, sends result
    /// 5. We receive result and return it to admin client
    ///
    /// WHY BLOCKING (recv)?
    /// - Admin wants to wait for the result before returning
    /// - This creates a synchronous request-response pattern for the admin
    /// - The implant sees an async polling pattern, but admin sees sync RPC
    ///
    /// LIMITATIONS:
    /// - If implant is offline, admin will wait forever (add timeout in production!)
    /// - Only works with one admin at a time (results could go to wrong admin)
    /// - No command routing/matching (first result goes to first waiting admin)
    async fn run_command(&self, request: Request<Command>) -> Result<Response<Command>, Status> {
        let cmd = request.into_inner();

        // Send command to work channel
        // This is non-blocking - the command goes into the queue immediately
        self.work_tx
            .send(cmd)
            .map_err(|_| Status::internal("Failed to send command to implant"))?;

        // Wait for response from output channel
        // This BLOCKS until an implant sends back a result
        let mut rx = self.output_rx.lock().await;

        // recv() waits indefinitely for a message
        // In production, you'd want to add a timeout here
        match rx.recv().await {
            Some(result) => Ok(Response::new(result)),
            None => Err(Status::internal("Output channel closed")),
        }
    }
}

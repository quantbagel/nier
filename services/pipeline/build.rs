//! Build script for compiling protobuf schemas
//!
//! This script compiles the .proto files in src/schemas/ into Rust code
//! when the "proto" feature is enabled.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Only compile protos if the proto feature is enabled
    #[cfg(feature = "proto")]
    {
        let proto_files = [
            "src/schemas/detection_event.proto",
            "src/schemas/frame_metadata.proto",
            "src/schemas/alert.proto",
        ];

        // Tell cargo to rerun if proto files change
        for proto in &proto_files {
            println!("cargo:rerun-if-changed={}", proto);
        }

        prost_build::compile_protos(&proto_files, &["src/schemas/"])?;
    }

    Ok(())
}

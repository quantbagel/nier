use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(false)
        .build_client(true)
        .out_dir("src/")
        .compile(&["proto/ingest.proto"], &["proto/"])?;
    Ok(())
}

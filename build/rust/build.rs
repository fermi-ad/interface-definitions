use std::io::Result;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let mut protos = vec![];

    let proto_dir = Path::new("../../");
    // Recursively find all .proto files within the proto directory.
    find_protos(proto_dir, &mut protos)?;

    // Create the output directory if it does not already exist.
    fs::create_dir_all("./generated")?;

    // Use the builder pattern to configure and compile the proto files.
    // We pass the list of relative file paths and the correct include path.
    tonic_build::configure()
        .out_dir("./generated")
        .compile_protos(
            &protos,
            &[proto_dir],
        )?;

    // Tell Cargo to re-run the build script if any of the proto files change.
    for proto in protos.iter() {
        println!("cargo:rerun-if-changed={}", proto.to_str().unwrap());
    }

    Ok(())
}

// A helper function to recursively find all .proto files within a directory.
fn find_protos(dir: &Path, protos: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Recurse into subdirectories.
                find_protos(&path, protos)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("proto") {
                // If the file has a .proto extension, add it to our list.
                protos.push(path);
            }
        }
    }
    Ok(())
}
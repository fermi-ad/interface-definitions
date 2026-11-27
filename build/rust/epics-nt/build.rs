use std::{env, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../../..");
    let schemas: Vec<_> = ["types", "pvdata", "normative_types"]
        .into_iter()
        .map(|name| root.join(format!("proto/controls/common/v1/epics/{name}.proto")))
        .collect();
    for schema in &schemas {
        println!("cargo:rerun-if-changed={}", schema.display());
    }
    println!("cargo:rerun-if-env-changed=PROTOC");
    let compiler = match env::var_os("PROTOC") {
        Some(path) => PathBuf::from(path),
        None => protoc_bin_vendored::protoc_bin_path()?,
    };
    prost_build::Config::new()
        .protoc_executable(compiler)
        .file_descriptor_set_path(
            PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("epics_nt.bin"),
        )
        .compile_protos(&schemas, &[root])?;
    Ok(())
}

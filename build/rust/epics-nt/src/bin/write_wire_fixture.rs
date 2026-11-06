//! Write a deterministic Rust-produced fixture for the Java compatibility test.
use epics_nt::{
    Validate,
    proto::{NtScalar, Scalar, Time, scalar},
};
use prost::Message;
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("usage: write_wire_fixture <output.pb>")?,
    );
    let value = NtScalar {
        value: Some(Scalar {
            value: Some(scalar::Value::UlongValue(u64::MAX)),
        }),
        descriptor_text: Some(String::new()),
        time_stamp: Some(Time {
            seconds_past_epoch: Some(-1),
            nanoseconds: Some(-5),
            user_tag: Some(42),
        }),
        type_id: Some("epics:nt/NTScalar:1.0".into()),
        ..Default::default()
    };
    value.validate()?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, value.encode_to_vec())?;
    Ok(())
}

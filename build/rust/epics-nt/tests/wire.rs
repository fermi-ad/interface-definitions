mod support;
use epics_nt::{Validate, proto::*};
use prost::Message;
use support::*;

const FIXTURE: &str = "0a0b48ffffffffffffffffff011200221808ffffffffffffffffff0110fbffffffffffffffff01182aa2061565706963733a6e742f4e545363616c61723a312e30";
fn from_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
        .collect()
}

#[test]
fn rust_wire_bytes_match_java_uint64_presence_and_timestamp_fixture() {
    let mut message = scalar_message();
    message.value.as_mut().unwrap().value = Some(scalar::Value::UlongValue(u64::MAX));
    message.descriptor_text = Some(String::new());
    message.time_stamp = Some(time());
    roundtrip(&message);
    assert_eq!(message.encode_to_vec(), from_hex(FIXTURE));
}

#[test]
fn fixture_writer_emits_real_generated_rust_bytes() {
    let path =
        std::env::temp_dir().join(format!("epics-nt-rust-fixture-{}.pb", std::process::id()));
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_write_wire_fixture"))
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let data = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(data, from_hex(FIXTURE));
    NtScalar::decode(data.as_slice())
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn prost_unknown_field_behavior_is_explicit() {
    // Prost intentionally discards unknown fields on decode. Preserve original
    // bytes/descriptor data separately when proxying newer protobuf messages.
    let message = scalar_message();
    let mut bytes = message.encode_to_vec();
    bytes.extend_from_slice(&[0xc0, 0x0c, 0x07]);
    let copy = NtScalar::decode(bytes.as_slice()).unwrap();
    copy.validate().unwrap();
    assert_eq!(copy.encode_to_vec(), message.encode_to_vec());
}

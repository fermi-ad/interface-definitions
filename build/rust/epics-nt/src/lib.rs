//! Repository-local Prost bindings and reference validation for the canonical
//! EPICS Normative Types. This is not a pvData/pvAccess adapter or codec decoder.
//!
//! ```
//! use epics_nt::{Validate, proto::{NtScalar, Scalar, scalar}};
//! use prost::Message;
//! let value = NtScalar {
//!     type_id: Some("epics:nt/NTScalar:1.0".into()),
//!     value: Some(Scalar { value: Some(scalar::Value::UlongValue(u64::MAX)) }),
//!     ..Default::default()
//! };
//! value.validate().unwrap();
//! let restored = NtScalar::decode(value.encode_to_vec().as_slice()).unwrap();
//! restored.validate().unwrap();
//! assert_eq!(value, restored);
//! ```

/// Generated, language-neutral protobuf contract. Existing service APIs are not changed.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/common.epics.v1.rs"));
}

/// Descriptor set for schema/contract tests and consumer introspection.
pub const FILE_DESCRIPTOR_SET: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/epics_nt.bin"));

mod validate;
pub use validate::{Validate, ValidationError};

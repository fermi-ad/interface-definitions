# EPICS Normative Types

## Contract and scope

`proto/controls/common/v1/epics/` provides the `common.epics.v1` protobuf package
for the 13 types in Appendix B of the
[EPICS Normative Types Editors Draft, 16 March 2015](https://docs.epics-controls.org/en/latest/pv-access/Normative-Types-Specification.html).
Rust is the primary implementation language for reference validation and tests.
Legacy Java DPM is an expected consumer; the Java package is
`gov.fnal.controls.common.epics.v1`. No Python runtime or scripts are required.

This is a protobuf application-level mapping, not the pvAccess wire protocol.
It does not change `common.device.Value`, DAQ, or existing alarm interfaces.
NTUnion and NTScalarMultiChannel from Appendix A are not included. No live pvData
adapter, codec implementation, or performance claim is supplied.

| Schema | Contents |
| --- | --- |
| `types.proto` | The 12 scalar types and homogeneous arrays; enum_t, time_t, alarm_t, alarmLimit_t, display_t, control_t |
| `pvdata.proto` | Descriptor-aware dynamic fields, structures, variant and regular unions, nullable array elements |
| `normative_types.proto` | Typed NT messages, URI query and table structures, image metadata |

The `.proto` files are authoritative for field tags. Comments identify pvData
names and constraints. Protobuf does not enforce required-by-contract fields or
cross-field invariants; validate before serialization and before reconstruction.

## Normative Types

Every NT message requires a present, case-sensitive `type_id` equal to
`epics:nt/<MessageName>:1.0`. Tag 100 carries the ID; it is the pvData structure ID,
not an additional pvData field. Protobuf API `v1`, NT ID version `1.0`, and an EPICS
software release are independent version concepts.

| Type | Required protobuf fields | Additional rules |
| --- | --- | --- |
| NTScalar | `value` | Select one Scalar discriminator, including for zero/false/empty string. |
| NTScalarArray | `value` | Select one homogeneous array discriminator, including for an empty array. |
| NTEnum | `value` | Explicit index selects one of the supplied choices. |
| NTMatrix | `value` | Absent dim means vector. Present dim has 1 or 2 positive elements, product equals value length; row-major. |
| NTURI | `scheme`, `path` | Nonempty RFC 3986 scheme. Optional authority precedes path in pvData. Optional query has ordered string/double/int arguments. |
| NTNameValue | `name`, `value` | Lengths match; association is positional, not a map. |
| NTTable | `labels`, `value` | Ordered unique column field names, matching label count, equal row counts. Labels need not equal names. |
| NTAttribute | `name`, `value` | Variant union value; optional tags and metadata. In an image, descriptor_text/source_type/source are required. |
| NTMultiChannel | `value`, `channel_name` | One common union descriptor, matching value/name counts and every present metadata array length. |
| NTNDArray | `value`, `codec`, `compressed_size`, `uncompressed_size`, `dimension`, `unique_id`, `data_time_stamp`, `attribute` | Numeric/boolean arrays only. Stored size matches type width. Raw arrays cover the dimension product. Codec-dependent decoding remains external. |
| NTContinuum | `base`, `value`, `units` | Point-interleaved; values per base point equal units count minus one. |
| NTHistogram | `ranges`, `value` | Signed short/int/long counts; ranges length is counts length plus one. |
| NTAggregate | `value`, `n` | n maps to pvData uppercase N and is nonnegative. Summary fields are independently optional. |

All types except NTURI support optional descriptor_text, alarm, and time_stamp.
The protobuf field descriptor_text maps to pvData descriptor; the suffix avoids
Java protobuf's reserved getDescriptor() accessor. Additional optional fields
are declared directly in the schema. Primitive fields
use proto3 `optional` to retain presence, even when they are required by contract;
message fields already retain presence. A wrapper with an empty repeated field
means a present empty array. An absent wrapper means the field is not present.

## Type fidelity

Scalar and ScalarArray retain boolean, byte, ubyte, short, ushort, int, uint, long,
ulong, float, double, and string as distinct cases. Check 8-bit and 16-bit ranges
before conversion. Do not infer width from magnitude or convert 64-bit integers
through double. Float NaN, infinities, and signed zero are permitted. Binary
round trips are tested; JSON/text fidelity and identical NaN payload bits are not
promised. Protobuf JSON represents 64-bit integers as strings.

Time retains seconds_past_epoch, signed nanoseconds, and the application-defined
user_tag exactly. A google.protobuf.Timestamp alone cannot represent all three.
Alarm codes are the draft's severity 0..4 and status 0..7, not common.alarm.Status
or common.status.Status. Display includes precision and the standard form enum.
AlarmLimit targets alarmLimit_t, not implementation-specific valueAlarm_t variants.

The typed core uses the canonical structure shapes and IDs in the linked draft.
It does not retain arbitrary custom nested metadata IDs, nested extra fields,
noncanonical union declarations, or bounded/fixed arrays inside typed core fields.
An adapter must reject unsupported introspection rather than silently drop it.
Use a full dynamic PVField outside the typed NT API when arbitrary introspection
must be retained.

## Dynamic fields, unions, and extensions

PVFieldType retains a field's kind, scalar discriminator, structure/union ID,
ordered named declarations, and array size policy. IDs are present even when the
ID string is empty. Scalar arrays can be variable, fixed, or bounded; structure
and union arrays are variable-sized. This follows
[pvDataCPP introspection](https://github.com/epics-base/pvDataCPP/blob/master/src/pv/pvIntrospect.h).
PVField pairs the descriptor with its matching value case. Strings in this mapping
are unbounded UTF-8. Implementation-specific bounded-string introspection is not
represented; adapters must reject it instead of silently erasing the bound.

For variant unions, variant is explicitly true, declarations are empty, and a
selected value can be any PVField. No selected_name is stored. Regular unions
have variant explicitly false and one or more unique, ordered alternatives;
a selection must name a declared alternative and have the matching descriptor.
An empty union value means no selection. A nullable array element without a value
means null, which is distinct from an unselected union. Empty structure/union
arrays still retain their element descriptor.

`google.protobuf.Any` is not used as a substitute for pvData `any`: it would need
additional descriptor, selection, and registry conventions to preserve these
semantics.

NT `extra_fields` at tag 101 holds uniquely named PVNamedField entries in order.
Names must not collide with canonical pvData fields, including fields omitted in
this instance. Restore descriptor_text to descriptor and snake_case schema names
to the published names such as timeStamp, channelName, sourceType, uniqueId, and N.
Emit canonical fields in the published order, then stored extensions. Do not emit type_id or extra_fields as
ordinary pvData fields. Protobuf binary field order is not pvData structure order.

### Explicit mapping policies and limits

- NTContinuum with an empty base requires an empty value array and at least two
  units strings, declaring a base unit and one or more component units. The draft
  does not define division by an empty base; this is the mapping's empty-data policy.
- NTAggregate observation counts and image dimension sizes are nonnegative.
- Raw NTNDArray stored arrays may include padding. The draft requires at least the
  dimension product, not exact equality. compressed_size counts the whole stored
  array; uncompressed_size counts the logical dimensions.
- A compressed image's decoded type may differ from its stored array type.
  The reference validator checks stored size and structural fields, but cannot
  validate decoded content/size without the named codec. A zero-dimension image
  still requires uncompressed_size zero.
- Future identifiers are not accepted automatically. Unsupported data must fail
  with a reason, not be silently coerced into this version.

## Rust generation, validation, and tests

Run from the repository root with Rust 1.85 or later:

```bash
cargo test --locked --manifest-path build/rust/epics-nt/Cargo.toml
cargo fmt --manifest-path build/rust/epics-nt/Cargo.toml --check
cargo clippy --locked --manifest-path build/rust/epics-nt/Cargo.toml --all-targets -- -D warnings
```

The repository-local `epics-nt` crate uses Prost 0.14, the protobuf library family
used by the Rust control services. Its build script compiles only the three EPICS
schemas into Cargo's OUT_DIR and produces a descriptor set. A pinned vendored
compiler supplies protoc; set `PROTOC` to select an installed compiler instead.
The Cargo.lock file makes CI dependency resolution repeatable. Generated source
and build outputs are not committed.

The crate is not published. Services may generate the same `.proto` files through
their existing `rust-grpc-lib`/Prost pipeline. The `proto` module contains the
bindings; `Validate` is the reference validation trait implemented for all 13 NTs.
Example, also exercised as a Rust documentation test:

```rust
use epics_nt::{Validate, proto::{NtScalar, Scalar, scalar}};
use prost::Message;

let value = NtScalar {
    type_id: Some("epics:nt/NTScalar:1.0".into()),
    value: Some(Scalar { value: Some(scalar::Value::UlongValue(u64::MAX)) }),
    ..Default::default()
};
value.validate().unwrap();
let restored = NtScalar::decode(value.encode_to_vec().as_slice()).unwrap();
restored.validate().unwrap();
assert_eq!(value, restored);
```

ValidationError exposes a protobuf field path and reason. Validation does not
mutate or normalize the message. Bound untrusted payload byte size before decoding;
`validate()` limits recursive depth to 64. `validate_with_limit()` accepts a positive
explicit limit. A Java consumer must implement the same boundary invariants;
Java code generation alone does not enforce them. No pvData adapter is supplied.

Rust tests cover all NTs, scalar types and limits, optional presence, empty/null
arrays, descriptor matching, unions, metadata, shape constraints, and the schema's
stable field names and tags. Floating-point special values are not normalized.

**Unknown protobuf fields:** Prost discards unknown fields when decoding and
re-encoding. Java's full runtime retains them. This difference is tested and is
not a universal unknown-field preservation guarantee. A transparent Rust proxy
must retain original bytes or use an explicit unknown-field-preserving mechanism.
The NT `extra_fields` representation is ordinary known schema data and is retained.

## Java artifact and DPM compatibility

Keep Java 21 and Maven support for legacy DPM consumers. To test Rust-to-Java binary
interoperability using the actual Rust-generated message, run:

```bash
cargo run --locked --manifest-path build/rust/epics-nt/Cargo.toml \
  --bin write_wire_fixture -- /tmp/epics-nt-scalar.pb
mvn -f build/java/controls-epics/pom.xml \
  -Depics.nt.fixture=/tmp/epics-nt-scalar.pb clean package
```

Maven can also run independently without that property, using the same fixed
reference bytes. CI always supplies the Rust-produced fixture.

Coordinates: `gov.fnal.controls.common.proto:epics-nt-v1`. The local version is
1.0.0-SNAPSHOT. Java protoc/protobuf-java remain at 4.30.2, matching the existing
DAQ/DPM runtime. Rust's generator is independent of Java's runtime requirement;
both use the same protobuf wire contract. Keep Java generated-code/runtime versions
aligned, or explicitly resolve a compatible newer runtime in a consuming application.

Generated Java classes use `gov.fnal.controls.common.epics.v1`, with separate
descriptor outer classes. Java tests cover generated APIs, unknown-field retention, and Rust-produced
bytes for the full uint64 range, timestamp components, and present empty metadata.
The module contains messages, not a gRPC service or a DPM adapter. Existing DPM and
DAQ artifacts are not changed or made to import these structures automatically.

## CI and release boundary

`.github/workflows/epics-nt-validation.yaml` runs Rust formatting, Clippy, contract
tests, and the Rust-to-Java compatibility test/build. The existing generic schema
validation workflow already compiles the new schemas and is unchanged.

The shared DAQ/DPM publishing workflow is unchanged. This PR does not add automatic
EPICS Java or Rust artifact publication, version bumping, or unrelated shared-workflow
fixes. A distribution/release path for DPM is a separate, explicit decision before
adoption. Do not run Maven deploy without release authorization.

## Adoption and compatibility

Import the new schema through repository-rooted paths, for example
`proto/controls/common/v1/epics/normative_types.proto`. Existing Value and DAQ
consumers keep working unchanged. Adding a new NT field/oneof case to a service
requires its own compatibility review, including how old consumers handle unknown
cases. Do not cast repository alarms or device.Value messages into NT equivalents.

Keep tags and names stable; reserve both on removal. Compatible additions remain
within v1; breaking changes need a separate package/version. Unknown-field behavior
depends on the binding runtime as described above. Field-by-field pvData converters
need explicit extension preservation and must reject unsupported structures.

Before introducing a production pvData adapter, test real pvData -> protobuf ->
pvData round trips against a pinned EPICS implementation, comparing introspection,
IDs, field order, presence, scalar types, values, unions, and array declarations.
The tests here prove the protobuf contract only. Benchmark comparable binary
representations before making size, speed, or memory claims.

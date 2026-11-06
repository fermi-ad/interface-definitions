# EPICS Normative Types

## Contract and scope

`proto/controls/common/v1/epics/` provides the `common.epics.v1` protobuf package
for the 13 types in Appendix B of the
[EPICS Normative Types Editors Draft, 16 March 2015](https://docs.epics-controls.org/en/latest/pv-access/Normative-Types-Specification.html).
The Java package is `gov.fnal.controls.common.epics.v1`.

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

## Generate and test

Commands run from the repository root. Use protoc 30.2 with Python protobuf 6.30.2
(the matching versions pinned in CI), or set `PROTOC` to a compatible compiler.
Use a virtual environment rather than modifying the system Python:

```bash
python3 -m venv /tmp/epics-nt-venv
/tmp/epics-nt-venv/bin/python -m pip install -r tools/epics_nt/requirements.txt
/tmp/epics-nt-venv/bin/python tools/epics_nt/test_contract.py
/tmp/epics-nt-venv/bin/python tools/epics_nt/test_workflow.py
```

The runner compiles only the three EPICS schemas, emits Python/Java/C++ source and
a descriptor set in a temporary directory, runs contract tests, and removes the
temporary output. It does not compile arbitrary untracked experiments in proto/.
To generate code for an application, supply the repository root include path:

```bash
mkdir -p /tmp/epics-nt-python
"${PROTOC:-protoc}" -I . --python_out=/tmp/epics-nt-python \
  proto/controls/common/v1/epics/types.proto \
  proto/controls/common/v1/epics/pvdata.proto \
  proto/controls/common/v1/epics/normative_types.proto
```

Validate generated messages with the reference Python validator:

```bash
PYTHONPATH=/tmp/epics-nt-python:tools/epics_nt /tmp/epics-nt-venv/bin/python - <<'PY'
from proto.controls.common.v1.epics import normative_types_pb2 as nt
from proto.controls.common.v1.epics import types_pb2 as types
from validate import validate

message = nt.NTScalar(type_id="epics:nt/NTScalar:1.0",
                      value=types.Scalar(ulong_value=2**64 - 1))
validate(message)
restored = nt.NTScalar.FromString(message.SerializeToString())
validate(restored)
assert restored.value.ulong_value == 2**64 - 1
PY
```

ValidationError reports a protobuf field path and reason. The validator does not
mutate the message. Bound untrusted payload byte size before parsing; validation
limits recursive depth to 64 by default. A service must port the same invariants
if its implementation language is not Python. No validator is generated by protoc.

## Java artifact and CI

The message-only Maven module requires Java 21 and Maven:

```bash
mvn -f build/java/controls-epics/pom.xml clean package
```

Coordinates: `gov.fnal.controls.common.proto:epics-nt-v1`. The local version is
1.0.0-SNAPSHOT. The module pins protoc/protobuf-java 4.30.2, matching protoc 30.2
and the existing DAQ/DPM runtime. A newer generated-code version can fail to load
if Maven resolves an older runtime from a service artifact. Keep these versions
aligned, or explicitly resolve a compatible newer runtime in the consuming application.
Generated classes use `gov.fnal.controls.common.epics.v1`, with separate descriptor
outer classes. Maven runs Java contract tests, including a Python-produced binary
fixture with the full uint64 range, timestamp components, and present empty metadata.
The module does not require a gRPC plugin or add a service API.

The validation workflow compiles all tracked schemas, runs the EPICS contract
suite and publishing-detector regression tests, and packages the Java module. The existing main-merge publishing workflow
includes a separately validated EPICS job, producing versions
`1.0.<GitHub run number>` in GitHub Packages, consistent with the current DAQ/DPM
publisher. No artifact is published merely by adding this configuration or running
package locally. Do not run deploy without delivery authorization.

## Adoption and compatibility

Import the new schema through repository-rooted paths, for example
`proto/controls/common/v1/epics/normative_types.proto`. Existing Value and DAQ
consumers keep working unchanged. Adding a new NT field/oneof case to a service
requires its own compatibility review, including how old consumers handle unknown
cases. Do not cast repository alarms or device.Value messages into NT equivalents.

Keep tags and names stable; reserve both on removal. Compatible additions remain
within v1; breaking changes need a separate package/version. Binary protobuf
unknown-field retention is tested, but field-by-field pvData converters need explicit
extension preservation and must reject unsupported structures.

Before introducing a production pvData adapter, test real pvData -> protobuf ->
pvData round trips against a pinned EPICS implementation, comparing introspection,
IDs, field order, presence, scalar types, values, unions, and array declarations.
The tests here prove the protobuf contract only. Benchmark comparable binary
representations before making size, speed, or memory claims.

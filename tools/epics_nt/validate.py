"""Reference validation for common.epics.v1's canonical NT mapping.

Call validate(message) before serialization or pvData reconstruction. This checks
protobuf-side invariants, not live EPICS introspection or codec-dependent content.
Generated message imports are deliberately left to the consuming application.
"""

import math
import re

from google.protobuf.message import Message


class ValidationError(ValueError):
    """A contract violation, reported with its protobuf field path."""


REQUIRED_FIELDS = {
    "NTScalar": ("value",),
    "NTScalarArray": ("value",),
    "NTEnum": ("value",),
    "NTMatrix": ("value",),
    "NTURI": ("scheme", "path"),
    "NTNameValue": ("name", "value"),
    "NTTable": ("labels", "value"),
    "NTAttribute": ("name", "value"),
    "NTMultiChannel": ("value", "channel_name"),
    "NTNDArray": ("value", "codec", "compressed_size", "uncompressed_size",
                  "dimension", "unique_id", "data_time_stamp", "attribute"),
    "NTContinuum": ("base", "value", "units"),
    "NTHistogram": ("ranges", "value"),
    "NTAggregate": ("value", "n"),
}

SCALAR_TYPES = {name: index for index, name in enumerate((
    "boolean_value", "byte_value", "ubyte_value", "short_value", "ushort_value",
    "int_value", "uint_value", "long_value", "ulong_value", "float_value",
    "double_value", "string_value"), start=1)}
NARROW_RANGES = {
    "byte_value": (-128, 127), "ubyte_value": (0, 255),
    "short_value": (-32768, 32767), "ushort_value": (0, 65535),
}
SCALAR_WIDTHS = {
    "boolean_value": 1, "byte_value": 1, "ubyte_value": 1,
    "short_value": 2, "ushort_value": 2, "int_value": 4, "uint_value": 4,
    "long_value": 8, "ulong_value": 8, "float_value": 4, "double_value": 8,
}
DISPLAY_FORMS = ["Default", "String", "Binary", "Decimal", "Hex", "Exponential", "Engineering"]
PV_NAMES = {
    "descriptor_text": "descriptor", "time_stamp": "timeStamp", "channel_name": "channelName",
    "seconds_past_epoch": "secondsPastEpoch", "user_tag": "userTag",
    "compressed_size": "compressedSize", "uncompressed_size": "uncompressedSize",
    "unique_id": "uniqueId", "data_time_stamp": "dataTimeStamp",
    "source_type": "sourceType", "first_time_stamp": "firstTimeStamp",
    "last_time_stamp": "lastTimeStamp", "n": "N",
}
# PVFieldKind and PVArraySize values are part of pvdata.proto's public contract.
SCALAR, SCALAR_ARRAY, STRUCTURE, STRUCTURE_ARRAY, UNION, UNION_ARRAY = range(1, 7)
VARIABLE, FIXED, BOUNDED = range(1, 4)


def _fail(path, reason):
    raise ValidationError(f"{path}: {reason}")


def _require(message, fields, path):
    for field in fields:
        if not message.HasField(field):
            _fail(f"{path}.{field}", "required field is absent")


def _forbid(message, fields, path):
    for field in fields:
        if message.HasField(field):
            _fail(f"{path}.{field}", "not applicable to this field kind")


def _depth(path, depth, limit):
    if depth > limit:
        _fail(path, f"maximum nesting depth {limit} exceeded")


def _unique_name(entry, seen, path):
    _require(entry, ("name",), path)
    if not entry.name:
        _fail(f"{path}.name", "field name is empty")
    if entry.name in seen:
        _fail(f"{path}.name", f"duplicate field name {entry.name!r}")
    seen.add(entry.name)


def _range(value, low, high, path):
    if not low <= value <= high:
        _fail(path, f"expected {low}..{high}, got {value}")


def _scalar(message, path):
    case = message.WhichOneof("value")
    if case not in SCALAR_TYPES:
        _fail(path, "a scalar type must be selected")
    if case in NARROW_RANGES:
        _range(getattr(message, case), *NARROW_RANGES[case], f"{path}.{case}")
    return case


def _array(message, path):
    case = message.WhichOneof("value")
    if case not in SCALAR_TYPES:
        _fail(path, "an array type must be selected, including for an empty array")
    values = getattr(message, case).value
    if case in NARROW_RANGES:
        for index, value in enumerate(values):
            _range(value, *NARROW_RANGES[case], f"{path}.{case}.value[{index}]")
    return case, values


def _enum(message, path):
    _require(message, ("index", "choices"), path)
    if not 0 <= message.index < len(message.choices.value):
        _fail(f"{path}.index", "must select an entry in choices")


def _alarm(message, path):
    _require(message, ("severity", "status", "message"), path)
    _range(message.severity, 0, 4, f"{path}.severity")
    _range(message.status, 0, 7, f"{path}.status")


def _time(message, path):
    _require(message, ("seconds_past_epoch", "nanoseconds", "user_tag"), path)


def _metadata(message, path):
    if "alarm" in message.DESCRIPTOR.fields_by_name and message.HasField("alarm"):
        _alarm(message.alarm, f"{path}.alarm")
    for field in ("time_stamp", "data_time_stamp", "first_time_stamp", "last_time_stamp"):
        if field in message.DESCRIPTOR.fields_by_name and message.HasField(field):
            _time(getattr(message, field), f"{path}.{field}")
    if "display" in message.DESCRIPTOR.fields_by_name and message.HasField("display"):
        display = message.display
        _require(display, ("limit_low", "limit_high", "description", "units", "precision", "form"),
                 f"{path}.display")
        _enum(display.form, f"{path}.display.form")
        if list(display.form.choices.value) != DISPLAY_FORMS:
            _fail(f"{path}.display.form.choices", "must use the standard ordered display forms")
    if "control" in message.DESCRIPTOR.fields_by_name and message.HasField("control"):
        _require(message.control, ("limit_low", "limit_high", "min_step"), f"{path}.control")


def _field_type(descriptor, path, depth, limit):
    _depth(path, depth, limit)
    kind = descriptor.kind
    if kind not in range(SCALAR, UNION_ARRAY + 1):
        _fail(f"{path}.kind", "unknown or unspecified pvData field kind")
    if kind in (SCALAR, SCALAR_ARRAY):
        _require(descriptor, ("scalar_type",), path)
        if descriptor.scalar_type not in SCALAR_TYPES.values():
            _fail(f"{path}.scalar_type", "unknown or unspecified scalar type")
        _forbid(descriptor, ("type_id", "variant"), path)
        if descriptor.fields:
            _fail(f"{path}.fields", "scalar fields cannot declare members")
    else:
        _require(descriptor, ("type_id",), path)
        _forbid(descriptor, ("scalar_type",), path)
        if kind in (UNION, UNION_ARRAY):
            _require(descriptor, ("variant",), path)
            if descriptor.variant and descriptor.fields:
                _fail(f"{path}.fields", "variant unions do not declare alternatives")
            if not descriptor.variant and not descriptor.fields:
                _fail(f"{path}.fields", "regular unions require at least one alternative")
        else:
            _forbid(descriptor, ("variant",), path)
        seen = set()
        for index, member in enumerate(descriptor.fields):
            member_path = f"{path}.fields[{index}]"
            _unique_name(member, seen, member_path)
            _require(member, ("type",), member_path)
            _field_type(member.type, f"{member_path}.type", depth + 1, limit)
    if kind in (SCALAR_ARRAY, STRUCTURE_ARRAY, UNION_ARRAY):
        _require(descriptor, ("array_size",), path)
        if descriptor.array_size not in (VARIABLE, FIXED, BOUNDED):
            _fail(f"{path}.array_size", "unknown or unspecified array size policy")
        if kind != SCALAR_ARRAY and descriptor.array_size != VARIABLE:
            _fail(f"{path}.array_size", "structure and union arrays are variable-sized")
        if descriptor.array_size == VARIABLE:
            _forbid(descriptor, ("array_capacity",), path)
        else:
            _require(descriptor, ("array_capacity",), path)
    else:
        _forbid(descriptor, ("array_size", "array_capacity"), path)


def _structure_value(value, descriptor, path, depth, limit):
    _depth(path, depth, limit)
    if len(value.fields) != len(descriptor.fields):
        _fail(path, "structure values must match the declaration count and order")
    for index, (field, declaration) in enumerate(zip(value.fields, descriptor.fields)):
        field_path = f"{path}.fields[{index}]"
        if field.type != declaration.type:
            _fail(f"{field_path}.type", f"does not match declaration {declaration.name!r}")
        _field(field, field_path, depth + 1, limit)


def _union_value(value, descriptor, path, depth, limit):
    _depth(path, depth, limit)
    has_name = value.HasField("selected_name")
    has_value = value.HasField("selected_value")
    if descriptor.variant:
        if has_name:
            _fail(f"{path}.selected_name", "variant unions do not select a named alternative")
        if has_value:
            _field(value.selected_value, f"{path}.selected_value", depth + 1, limit)
    else:
        if has_name != has_value:
            _fail(f"{path}.selected_value", "regular union selection requires both name and value")
        if has_value:
            members = {field.name: field.type for field in descriptor.fields}
            if value.selected_name not in members:
                _fail(f"{path}.selected_name", "not a declared regular-union alternative")
            if value.selected_value.type != members[value.selected_name]:
                _fail(f"{path}.selected_value.type", "does not match the selected declaration")
            _field(value.selected_value, f"{path}.selected_value", depth + 1, limit)


def _union_array_value(value, descriptor, path, depth, limit):
    for index, element in enumerate(value.elements):
        if element.HasField("value"):
            _union_value(element.value, descriptor, f"{path}.elements[{index}].value", depth + 1, limit)


def _field(message, path, depth, limit):
    _depth(path, depth, limit)
    _require(message, ("type",), path)
    descriptor = message.type
    _field_type(descriptor, f"{path}.type", depth, limit)
    cases = {SCALAR: "scalar", SCALAR_ARRAY: "scalar_array", STRUCTURE: "structure",
             STRUCTURE_ARRAY: "structure_array", UNION: "union", UNION_ARRAY: "union_array"}
    case = cases[descriptor.kind]
    if message.WhichOneof("value") != case:
        _fail(path, f"expected value case {case!r} for its declared field kind")
    value = getattr(message, case)
    value_path = f"{path}.{case}"
    if descriptor.kind == SCALAR:
        scalar_case = _scalar(value, value_path)
        if SCALAR_TYPES[scalar_case] != descriptor.scalar_type:
            _fail(f"{path}.type.scalar_type", "does not match the selected scalar type")
    elif descriptor.kind == SCALAR_ARRAY:
        scalar_case, values = _array(value, value_path)
        if SCALAR_TYPES[scalar_case] != descriptor.scalar_type:
            _fail(f"{path}.type.scalar_type", "does not match the selected array type")
        if descriptor.array_size == FIXED and len(values) != descriptor.array_capacity:
            _fail(f"{path}.type.array_capacity", "fixed array length must equal its capacity")
        if descriptor.array_size == BOUNDED and len(values) > descriptor.array_capacity:
            _fail(f"{path}.type.array_capacity", "bounded array length exceeds its capacity")
    elif descriptor.kind == STRUCTURE:
        _structure_value(value, descriptor, value_path, depth, limit)
    elif descriptor.kind == STRUCTURE_ARRAY:
        for index, element in enumerate(value.elements):
            if element.HasField("value"):
                _structure_value(element.value, descriptor,
                                 f"{value_path}.elements[{index}].value", depth + 1, limit)
    elif descriptor.kind == UNION:
        _union_value(value, descriptor, value_path, depth, limit)
    else:
        _union_array_value(value, descriptor, value_path, depth, limit)


def _union(message, path, depth, limit, *, array=False, variant=False):
    _require(message, ("type", "value"), path)
    _field_type(message.type, f"{path}.type", depth, limit)
    if message.type.kind != (UNION_ARRAY if array else UNION):
        _fail(f"{path}.type.kind", "expected a union array" if array else "expected a union")
    if variant and not message.type.variant:
        _fail(f"{path}.type.variant", "this NT field requires a variant union")
    if array:
        _union_array_value(message.value, message.type, f"{path}.value", depth, limit)
    else:
        _union_value(message.value, message.type, f"{path}.value", depth, limit)


def _extensions(message, path, depth, limit):
    canonical = {PV_NAMES.get(field.name, field.name) for field in message.DESCRIPTOR.fields
                 if field.name not in ("type_id", "extra_fields")}
    seen = set()
    for index, field in enumerate(message.extra_fields):
        field_path = f"{path}.extra_fields[{index}]"
        _unique_name(field, seen, field_path)
        if field.name in canonical:
            _fail(f"{field_path}.name", "collides with a canonical pvData field name")
        _require(field, ("value",), field_path)
        _field(field.value, f"{field_path}.value", depth + 1, limit)


def _matrix(message, path):
    if message.HasField("dim"):
        dims = message.dim.value
        if len(dims) not in (1, 2) or any(dim <= 0 for dim in dims):
            _fail(f"{path}.dim", "must contain one or two strictly positive dimensions")
        if math.prod(dims) != len(message.value.value):
            _fail(f"{path}.dim", "dimension product must equal value length")


def _uri(message, path):
    if re.fullmatch(r"[A-Za-z][A-Za-z0-9+.-]*", message.scheme) is None:
        _fail(f"{path}.scheme", "must be a nonempty RFC 3986 scheme")
    if message.HasField("query"):
        seen = set()
        for index, argument in enumerate(message.query.arguments):
            argument_path = f"{path}.query.arguments[{index}]"
            _unique_name(argument, seen, argument_path)
            if argument.WhichOneof("value") is None:
                _fail(f"{argument_path}.value", "a string, double, or int argument is required")


def _table(message, path):
    if len(message.labels.value) != len(message.value.columns):
        _fail(f"{path}.labels", "label count must equal column count")
    seen = set()
    rows = None
    for index, column in enumerate(message.value.columns):
        column_path = f"{path}.value.columns[{index}]"
        _unique_name(column, seen, column_path)
        _require(column, ("value",), column_path)
        _, values = _array(column.value, f"{column_path}.value")
        if rows is not None and len(values) != rows:
            _fail(column_path, "all columns must have the same row count")
        rows = len(values)


def _multichannel(message, path, depth, limit):
    _union(message.value, f"{path}.value", depth, limit, array=True)
    count = len(message.value.value.elements)
    if count != len(message.channel_name.value):
        _fail(f"{path}.channel_name", "channel names must match the value count")
    for field in ("severity", "status", "message", "seconds_past_epoch", "nanoseconds", "user_tag"):
        if message.HasField(field):
            values = getattr(message, field).value
            if len(values) != count:
                _fail(f"{path}.{field}", "per-channel metadata must match the value count")
            if field in ("severity", "status"):
                for index, value in enumerate(values):
                    _range(value, 0, 4 if field == "severity" else 7, f"{path}.{field}.value[{index}]")


def _image(message, path, depth, limit):
    case, values = _array(message.value, f"{path}.value")
    width = SCALAR_WIDTHS[case]
    _require(message.codec, ("name", "parameters"), f"{path}.codec")
    _union(message.codec.parameters, f"{path}.codec.parameters", depth, limit, variant=True)
    if message.compressed_size != len(values) * width:
        _fail(f"{path}.compressed_size", "must equal stored element count times scalar byte width")
    if message.uncompressed_size < 0:
        _fail(f"{path}.uncompressed_size", "must be nonnegative")
    dimensions = message.dimension.value
    for index, dimension in enumerate(dimensions):
        dim_path = f"{path}.dimension.value[{index}]"
        _require(dimension, ("size", "offset", "full_size", "binning", "reverse"), dim_path)
        if dimension.size < 0:
            _fail(f"{dim_path}.size", "must be nonnegative")
    count = math.prod(dim.size for dim in dimensions) if dimensions else 0
    if not dimensions and message.uncompressed_size != 0:
        _fail(f"{path}.uncompressed_size", "must be zero when there are no dimensions")
    if not message.codec.name:
        if len(values) < count:
            _fail(f"{path}.value", "raw element count must cover the dimension product")
        if message.uncompressed_size != count * width:
            _fail(f"{path}.uncompressed_size", "must equal dimension product times scalar byte width")
    # A compressed image's decoded type and content require codec-specific checks.
    for index, attribute in enumerate(message.attribute.value):
        attr_path = f"{path}.attribute.value[{index}]"
        _require(attribute, ("descriptor_text", "source_type", "source"), attr_path)
        _validate(attribute, attr_path, depth + 1, limit)


def _continuum(message, path):
    base_count, value_count, units_count = (len(message.base.value), len(message.value.value),
                                           len(message.units.value))
    if not base_count:
        if value_count:
            _fail(f"{path}.base", "an empty base requires an empty value array")
        if units_count < 2:
            _fail(f"{path}.units", "empty data requires base units and at least one component's units")
    else:
        if value_count % base_count or value_count // base_count < 1:
            _fail(f"{path}.value", "must contain one or more complete components for every base point")
        if units_count != value_count // base_count + 1:
            _fail(f"{path}.units", "must contain base units plus one unit per point component")


def _validate(message, path, depth, limit):
    _depth(path, depth, limit)
    name = message.DESCRIPTOR.name
    _require(message, (*REQUIRED_FIELDS[name], "type_id"), path)
    if message.type_id != f"epics:nt/{name}:1.0":
        _fail(f"{path}.type_id", f"expected case-sensitive epics:nt/{name}:1.0")
    _metadata(message, path)
    _extensions(message, path, depth, limit)
    if name == "NTScalar":
        _scalar(message.value, f"{path}.value")
    elif name == "NTScalarArray":
        _array(message.value, f"{path}.value")
    elif name == "NTEnum":
        _enum(message.value, f"{path}.value")
    elif name == "NTMatrix":
        _matrix(message, path)
    elif name == "NTURI":
        _uri(message, path)
    elif name == "NTNameValue":
        _, values = _array(message.value, f"{path}.value")
        if len(message.name.value) != len(values):
            _fail(f"{path}.name", "name and value arrays must have equal lengths")
    elif name == "NTTable":
        _table(message, path)
    elif name == "NTAttribute":
        _union(message.value, f"{path}.value", depth, limit, variant=True)
        if message.HasField("source_type"):
            _range(message.source_type, 0, 3, f"{path}.source_type")
    elif name == "NTMultiChannel":
        _multichannel(message, path, depth, limit)
    elif name == "NTNDArray":
        _image(message, path, depth, limit)
    elif name == "NTContinuum":
        _continuum(message, path)
    elif name == "NTHistogram":
        _, values = _array(message.value, f"{path}.value")
        if len(message.ranges.value) != len(values) + 1:
            _fail(f"{path}.ranges", "range boundary count must equal bin count plus one")
    elif name == "NTAggregate" and message.n < 0:
        _fail(f"{path}.n", "observation count must be nonnegative")


def validate(message, *, max_depth=64):
    """Validate one generated NT message, or raise ValidationError with a path.

    Caller must bound payload byte size before parsing. Nesting is limited here;
    codec-specific decoding and live pvData conversion remain the caller's work.
    Returns None on success; never mutates or normalizes the supplied message.
    """
    if not isinstance(message, Message) or message.DESCRIPTOR.full_name not in {
        f"common.epics.v1.{name}" for name in REQUIRED_FIELDS
    }:
        _fail("message", "unsupported message; expected a common.epics.v1 Normative Type")
    if not isinstance(max_depth, int) or isinstance(max_depth, bool) or max_depth < 1:
        raise ValueError("max_depth must be a positive integer")
    _validate(message, message.DESCRIPTOR.name, 0, max_depth)

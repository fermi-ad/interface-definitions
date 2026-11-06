#!/usr/bin/env python3
"""Generate temporary bindings and test the EPICS NT protobuf contract.

Run from any directory with PROTOC (or protoc on PATH) and a matching Python
protobuf runtime. Generated files never enter the source tree.
"""

import importlib
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
PROTO_DIR = "proto/controls/common/v1/epics"
TEMP = tempfile.TemporaryDirectory(prefix="epics-nt-contract-")
GENERATED = Path(TEMP.name)


def generate_bindings():
    compiler = os.environ.get("PROTOC", "protoc")
    executable = shutil.which(compiler)
    if executable is None:
        raise RuntimeError(f"Protocol Buffer compiler not found: {compiler}")
    includes = ["-I", str(ROOT)]
    # Support distribution-installed protoc and the official release ZIP layout.
    for path in (Path(executable).resolve().parent.parent / "include", Path("/usr/include")):
        if (path / "google/protobuf/descriptor.proto").is_file():
            includes.extend(["-I", str(path)])
    for language in ("python", "java", "cpp"):
        (GENERATED / language).mkdir()
    subprocess.run(
        [executable, *includes,
         f"--python_out={GENERATED / 'python'}",
         f"--java_out={GENERATED / 'java'}",
         f"--cpp_out={GENERATED / 'cpp'}",
         f"--descriptor_set_out={GENERATED / 'epics-nt.pb'}", "--include_imports",
         *(f"{PROTO_DIR}/{name}.proto" for name in ("types", "pvdata", "normative_types"))],
        check=True, cwd=ROOT,
    )
    sys.path.insert(0, str(GENERATED / "python"))
    return tuple(importlib.import_module(f"proto.controls.common.v1.epics.{name}_pb2")
                 for name in ("types", "pvdata", "normative_types"))


def scalar_field(case="int_value", value=0):
    result = pv.PVField()
    result.type.kind = pv.PV_FIELD_KIND_SCALAR
    result.type.scalar_type = getattr(pv, f"PV_SCALAR_TYPE_{case.removesuffix('_value').upper()}")
    setattr(result.scalar, case, value)
    return result


def union(variant=True, selected=None):
    result = pv.PVUnion()
    result.type.kind = pv.PV_FIELD_KIND_UNION
    result.type.type_id = "any" if variant else "union"
    result.type.variant = variant
    if not variant:
        member = result.type.fields.add(name="reading")
        member.type.CopyFrom(scalar_field().type)
    result.value.SetInParent()
    if selected is not None:
        result.value.selected_value.CopyFrom(selected)
        if not variant:
            result.value.selected_name = "reading"
    return result


def time():
    # Preserve components and userTag, including a non-normalized nanoseconds value.
    return types.Time(seconds_past_epoch=-1, nanoseconds=-5, user_tag=42)


def alarm():
    return types.Alarm(severity=0, status=0, message="")


def enum_value():
    return types.EnumValue(index=0, choices=types.StringArray(value=["off", "on"]))


def display():
    return types.Display(
        limit_low=0, limit_high=10, description="value", units="A", precision=3,
        form=types.EnumValue(index=0, choices=types.StringArray(value=[
            "Default", "String", "Binary", "Decimal", "Hex", "Exponential", "Engineering"])),
    )


def minimal_messages():
    scalar = nt.NTScalar(value=types.Scalar(int_value=0))
    array = nt.NTScalarArray(value=types.ScalarArray(double_value=types.DoubleArray()))
    enumeration = nt.NTEnum(value=enum_value())
    matrix = nt.NTMatrix(value=types.DoubleArray(value=[1, 2, 3, 4]),
                         dim=types.IntArray(value=[2, 2]))
    uri = nt.NTURI(scheme="pva", path="channel")
    names = nt.NTNameValue(name=types.StringArray(value=["a"]),
                           value=types.ScalarArray(long_value=types.LongArray(value=[2**63 - 1])))
    table = nt.NTTable(labels=types.StringArray(value=["Heading"]), value=nt.TableValue())
    table.value.columns.add(name="field", value=types.ScalarArray(
        string_value=types.StringArray(value=["row"])))
    attribute = nt.NTAttribute(name="key", value=union(selected=scalar_field()))
    channels = nt.NTMultiChannel(channel_name=types.StringArray(value=["a"]))
    channels.value.type.CopyFrom(union().type)
    channels.value.type.kind = pv.PV_FIELD_KIND_UNION_ARRAY
    channels.value.type.array_size = pv.PV_ARRAY_SIZE_VARIABLE
    channels.value.value.elements.add().value.selected_value.CopyFrom(scalar_field())
    image = nt.NTNDArray(
        value=nt.NDArrayValue(ubyte_value=types.UByteArray(value=[0, 255, 1, 2])),
        codec=nt.Codec(name="", parameters=union()), compressed_size=4, uncompressed_size=4,
        unique_id=0, data_time_stamp=time(), attribute=nt.AttributeArray(),
    )
    image.dimension.value.add(size=2, offset=0, full_size=2, binning=1, reverse=False)
    image.dimension.value.add(size=2, offset=0, full_size=2, binning=1, reverse=False)
    continuum = nt.NTContinuum(base=types.DoubleArray(value=[0, 1]),
                               value=types.DoubleArray(value=[10, 20, 11, 21]),
                               units=types.StringArray(value=["s", "A", "V"]))
    histogram = nt.NTHistogram(ranges=types.DoubleArray(value=[0, 1, 2]),
                               value=nt.HistogramValue(short_value=types.ShortArray(value=[2, 3])))
    aggregate = nt.NTAggregate(value=0, n=0)
    messages = [scalar, array, enumeration, matrix, uri, names, table, attribute,
                channels, image, continuum, histogram, aggregate]
    for message in messages:
        message.type_id = f"epics:nt/{message.DESCRIPTOR.name}:1.0"
    return messages


class ContractTests(unittest.TestCase):
    def roundtrip(self, message):
        validate.validate(message)
        result = type(message).FromString(message.SerializeToString())
        self.assertEqual(message, result)
        validate.validate(result)
        return result

    def invalid(self, message, path):
        with self.assertRaisesRegex(validate.ValidationError, path):
            validate.validate(message)

    def test_java_runtime_matches_existing_modules(self):
        # Maven can select the older direct service dependency over the NT
        # transitive runtime. Matching versions avoids class initialization errors.
        from xml.etree import ElementTree
        namespace = {"m": "http://maven.apache.org/POM/4.0.0"}
        epics = ElementTree.parse(ROOT / "build/java/controls-epics/pom.xml")
        version = epics.findtext("m:properties/m:protobuf.version", namespaces=namespace)
        for module in ("controls-daq", "controls-dpm"):
            tree = ElementTree.parse(ROOT / f"build/java/{module}/pom.xml")
            runtime = next(dependency.findtext("m:version", namespaces=namespace)
                           for dependency in tree.findall("m:dependencies/m:dependency", namespace)
                           if dependency.findtext("m:artifactId", namespaces=namespace) == "protobuf-java")
            self.assertEqual(version, runtime, module)

    def test_python_wire_fixture_matches_java_contract(self):
        message = nt.NTScalar(
            type_id="epics:nt/NTScalar:1.0", value=types.Scalar(ulong_value=2**64 - 1),
            descriptor_text="", time_stamp=time(),
        )
        expected = bytes.fromhex(
            "0a0b48ffffffffffffffffff011200221808ffffffffffffffffff01"
            "10fbffffffffffffffff01182aa2061565706963733a6e742f4e545363616c61723a312e30")
        self.roundtrip(message)
        self.assertEqual(message.SerializeToString(deterministic=True), expected)

    def test_all_thirteen_types_roundtrip(self):
        messages = minimal_messages()
        self.assertEqual(len(messages), 13)
        for message in messages:
            with self.subTest(type=message.DESCRIPTOR.name):
                self.roundtrip(message)

    def test_all_required_fields_reject_missing(self):
        for message in minimal_messages():
            required = validate.REQUIRED_FIELDS[message.DESCRIPTOR.name]
            for field in (*required, "type_id"):
                with self.subTest(type=message.DESCRIPTOR.name, field=field):
                    copy = type(message)()
                    copy.CopyFrom(message)
                    copy.ClearField(field)
                    self.invalid(copy, field)

    def test_schema_field_names_order_and_presence(self):
        # Explicit source-derived shapes guard against coordinated omissions in
        # the schema, validator, and minimal fixture. Transport metadata comes last.
        core = {
            "NTScalar": "value descriptor_text alarm time_stamp display control",
            "NTScalarArray": "value descriptor_text alarm time_stamp display control",
            "NTEnum": "value descriptor_text alarm time_stamp",
            "NTMatrix": "value dim descriptor_text alarm time_stamp display",
            "NTURI": "scheme authority path query",
            "NTNameValue": "name value descriptor_text alarm time_stamp",
            "NTTable": "labels value descriptor_text alarm time_stamp",
            "NTAttribute": "name value tags descriptor_text alarm time_stamp source_type source",
            "NTMultiChannel": "value channel_name descriptor_text alarm time_stamp severity status message seconds_past_epoch nanoseconds user_tag",
            "NTNDArray": "value codec compressed_size uncompressed_size dimension unique_id data_time_stamp attribute descriptor_text alarm time_stamp display",
            "NTContinuum": "base value units descriptor_text alarm time_stamp",
            "NTHistogram": "ranges value descriptor_text alarm time_stamp",
            "NTAggregate": "value n dispersion first first_time_stamp last last_time_stamp max min descriptor_text alarm time_stamp",
        }
        for name, fields in core.items():
            descriptor = getattr(nt, name).DESCRIPTOR
            expected = fields.split()
            self.assertEqual([field.name for field in descriptor.fields],
                             [*expected, "type_id", "extra_fields"])
            self.assertEqual([field.number for field in descriptor.fields],
                             [*range(1, len(expected) + 1), 100, 101])
            for field in descriptor.fields[:-1]:
                self.assertTrue(field.has_presence, f"{name}.{field.name}")
        for descriptor in (types.Scalar.DESCRIPTOR, types.ScalarArray.DESCRIPTOR):
            self.assertEqual({field.name: field.number for field in descriptor.fields},
                             validate.SCALAR_TYPES)

    def test_common_metadata_schema_presence_and_order(self):
        expected = {
            "EnumValue": "index choices",
            "Time": "seconds_past_epoch nanoseconds user_tag",
            "Alarm": "severity status message",
            "AlarmLimit": "active low_alarm_limit low_warning_limit high_warning_limit high_alarm_limit low_alarm_severity low_warning_severity high_warning_severity high_alarm_severity hysteresis",
            "Display": "limit_low limit_high description units precision form",
            "Control": "limit_low limit_high min_step",
        }
        for name, fields in expected.items():
            descriptor = getattr(types, name).DESCRIPTOR
            self.assertEqual([field.name for field in descriptor.fields], fields.split())
            self.assertEqual([field.number for field in descriptor.fields],
                             list(range(1, len(fields.split()) + 1)))
            self.assertTrue(all(field.has_presence for field in descriptor.fields))
        limit = types.AlarmLimit(active=False, low_alarm_limit=0, low_warning_limit=0,
                                 high_warning_limit=0, high_alarm_limit=0,
                                 low_alarm_severity=0, low_warning_severity=0,
                                 high_warning_severity=0, high_alarm_severity=0, hysteresis=0)
        self.assertEqual(limit, types.AlarmLimit.FromString(limit.SerializeToString()))
        self.assertTrue(limit.HasField("active"))

    def test_type_identifiers_are_exact_and_case_sensitive(self):
        for identifier in ("epics:nt/ntscalar:1.0", "epics:nt/NTScalar:2.0", "", "NTScalar"):
            message = minimal_messages()[0]
            message.type_id = identifier
            self.invalid(message, "type_id")

    def test_all_scalar_widths_and_homogeneous_arrays(self):
        cases = {
            "boolean_value": [False, True], "byte_value": [-128, 0, 127],
            "ubyte_value": [0, 255], "short_value": [-32768, 32767],
            "ushort_value": [0, 65535], "int_value": [-(2**31), 2**31 - 1],
            "uint_value": [0, 2**32 - 1], "long_value": [-(2**63), 2**63 - 1],
            "ulong_value": [0, 2**64 - 1], "float_value": [-0.0, 1.5],
            "double_value": [-0.0, 1.5], "string_value": ["", "\u03bcA"],
        }
        self.assertEqual(len(cases), 12)
        for case, values in cases.items():
            for value in values:
                with self.subTest(case=case, value=value):
                    message = nt.NTScalar(type_id="epics:nt/NTScalar:1.0")
                    setattr(message.value, case, value)
                    result = self.roundtrip(message)
                    self.assertEqual(result.value.WhichOneof("value"), case)
                    if case in ("float_value", "double_value") and value == 0:
                        self.assertEqual(math.copysign(1, getattr(result.value, case)), -1)
            for elements in (values, []):
                with self.subTest(case=case, array=elements):
                    message = nt.NTScalarArray(type_id="epics:nt/NTScalarArray:1.0")
                    wrapper = getattr(message.value, case)
                    wrapper.SetInParent()
                    wrapper.value.extend(elements)
                    result = self.roundtrip(message)
                    self.assertEqual(result.value.WhichOneof("value"), case)

    def test_narrow_integer_ranges_rejected_for_scalars_and_arrays(self):
        for case, value in (("byte_value", -129), ("byte_value", 128),
                            ("ubyte_value", 256), ("short_value", -32769),
                            ("short_value", 32768), ("ushort_value", 65536)):
            message = nt.NTScalar(type_id="epics:nt/NTScalar:1.0")
            setattr(message.value, case, value)
            self.invalid(message, case)
            array = nt.NTScalarArray(type_id="epics:nt/NTScalarArray:1.0")
            getattr(array.value, case).value.append(value)
            self.invalid(array, case)

    def test_unselected_scalar_is_not_zero_or_empty_array(self):
        for message in minimal_messages()[:2]:
            message.value.Clear()
            self.invalid(message, "value")

    def test_special_floating_point_values_are_preserved(self):
        for case in ("float_value", "double_value"):
            for value in (float("nan"), float("inf"), -float("inf")):
                message = nt.NTScalar(type_id="epics:nt/NTScalar:1.0")
                setattr(message.value, case, value)
                validate.validate(message)
                result = type(message).FromString(message.SerializeToString())
                validate.validate(result)
                actual = getattr(result.value, case)
                self.assertTrue(math.isnan(actual) if math.isnan(value) else actual == value)

    def test_absent_and_present_default_metadata(self):
        message = minimal_messages()[0]
        self.assertFalse(self.roundtrip(message).HasField("descriptor_text"))
        message.descriptor_text = ""
        message.time_stamp.CopyFrom(time())
        message.alarm.CopyFrom(alarm())
        message.display.CopyFrom(display())
        message.control.CopyFrom(types.Control(limit_low=0, limit_high=0, min_step=0))
        result = self.roundtrip(message)
        self.assertTrue(result.HasField("descriptor_text"))
        self.assertEqual(result.time_stamp.nanoseconds, -5)
        self.assertEqual(result.time_stamp.user_tag, 42)
        self.assertTrue(result.alarm.HasField("severity"))

    def test_present_empty_optional_array(self):
        message = minimal_messages()[7]
        self.assertFalse(message.HasField("tags"))
        message.tags.SetInParent()
        self.assertTrue(self.roundtrip(message).HasField("tags"))

    def test_metadata_validation(self):
        message = minimal_messages()[0]
        message.alarm.CopyFrom(alarm())
        message.alarm.severity = 5
        self.invalid(message, "severity")
        message.alarm.severity = 0
        message.alarm.status = -1
        self.invalid(message, "status")
        message.alarm.Clear()
        self.invalid(message, "severity")
        message.ClearField("alarm")
        message.time_stamp.CopyFrom(time())
        message.time_stamp.ClearField("user_tag")
        self.invalid(message, "user_tag")
        message.ClearField("time_stamp")
        message.display.CopyFrom(display())
        message.display.form.choices.value[0] = "other"
        self.invalid(message, "choices")
        enumeration = minimal_messages()[2]
        enumeration.value.index = 2
        self.invalid(enumeration, "index")

    def test_matrix_vector_and_dimensions(self):
        message = minimal_messages()[3]
        message.ClearField("dim")
        self.roundtrip(message)
        message.dim.value.extend([4])
        self.roundtrip(message)
        for dims in ([], [0], [-1], [3], [1, 2, 2]):
            message.dim.value[:] = dims
            self.invalid(message, "dim")

    def test_uri_ordered_typed_query(self):
        message = minimal_messages()[4]
        message.query.arguments.add(name="start", string_value="now")
        message.query.arguments.add(name="count", int_value=0)
        message.query.arguments.add(name="scale", double_value=1.5)
        self.assertEqual([arg.name for arg in self.roundtrip(message).query.arguments],
                         ["start", "count", "scale"])
        message.query.arguments[1].name = "start"
        self.invalid(message, "name")
        message.query.arguments[1].name = "count"
        message.query.arguments[1].ClearField("int_value")
        self.invalid(message, "value")
        message.query.Clear()
        self.assertTrue(self.roundtrip(message).HasField("query"))
        message.authority = ""
        self.assertTrue(self.roundtrip(message).HasField("authority"))

    def test_uri_scheme_is_a_nonempty_rfc3986_scheme(self):
        message = minimal_messages()[4]
        for scheme in ("", "1pva", "+pva", "pva with spaces", "\u03bcva"):
            message.scheme = scheme
            self.invalid(message, "scheme")
        for scheme in ("pva", "PVA", "example+v1.2-test"):
            message.scheme = scheme
            self.roundtrip(message)

    def test_name_value_association_is_positional(self):
        message = minimal_messages()[5]
        message.name.value.append("a")  # Duplicate labels are allowed, length mismatch is not.
        self.invalid(message, "name")
        message.value.long_value.value.append(0)
        self.roundtrip(message)

    def test_table_labels_are_not_column_names(self):
        message = minimal_messages()[6]
        message.labels.value.append("Other heading")
        message.value.columns.add(name="other", value=types.ScalarArray(
            int_value=types.IntArray(value=[1])))
        self.roundtrip(message)
        message.value.columns[1].value.int_value.value.append(2)
        self.invalid(message, "columns")
        message.value.columns[1].value.int_value.value.pop()
        message.value.columns[1].name = "field"
        self.invalid(message, "name")
        del message.value.columns[:]
        self.invalid(message, "labels")
        del message.labels.value[:]
        self.roundtrip(message)

    def test_variant_union_selected_and_unselected(self):
        message = minimal_messages()[7]
        message.value.CopyFrom(union())
        result = self.roundtrip(message)
        self.assertFalse(result.value.value.HasField("selected_value"))
        message.value.CopyFrom(union(selected=scalar_field("ulong_value", 2**64 - 1)))
        self.roundtrip(message)
        message.value.value.selected_name = "not-a-regular-union"
        self.invalid(message, "selected_name")
        message.value.CopyFrom(union(variant=False))
        self.invalid(message, "variant")

    def test_regular_union_descriptor_selection_and_null_elements(self):
        message = minimal_messages()[8]
        regular = union(variant=False, selected=scalar_field())
        message.value.type.CopyFrom(regular.type)
        message.value.type.kind = pv.PV_FIELD_KIND_UNION_ARRAY
        message.value.type.array_size = pv.PV_ARRAY_SIZE_VARIABLE
        message.value.value.elements[0].value.CopyFrom(regular.value)
        self.roundtrip(message)
        selection = message.value.value.elements[0].value
        selection.selected_name = "unknown"
        self.invalid(message, "selected_name")
        selection.selected_name = "reading"
        selection.selected_value.CopyFrom(scalar_field("double_value", 1.5))
        self.invalid(message, "selected_value")
        selection.Clear()
        self.roundtrip(message)  # Present but unselected.
        message.value.value.elements[0].ClearField("value")
        self.roundtrip(message)  # Null, distinct from unselected.
        del message.value.value.elements[:]
        del message.channel_name.value[:]
        self.roundtrip(message)  # Empty still preserves declared alternatives.
        message.value.type.fields.add().CopyFrom(message.value.type.fields[0])
        self.invalid(message, "name")

    def test_descriptor_inapplicable_and_missing_fields_rejected(self):
        message = minimal_messages()[7]
        field = scalar_field()
        for name, value in (("type_id", "custom"), ("variant", False),
                            ("array_size", pv.PV_ARRAY_SIZE_VARIABLE), ("array_capacity", 1)):
            setattr(field.type, name, value)
            message.value.value.selected_value.CopyFrom(field)
            self.invalid(message, name)
            field.type.ClearField(name)
        field.type.ClearField("scalar_type")
        message.value.value.selected_value.CopyFrom(field)
        self.invalid(message, "scalar_type")
        message.value.CopyFrom(union())
        message.value.type.ClearField("type_id")
        self.invalid(message, "type_id")
        message.value.type.type_id = ""  # An explicitly empty structure/union ID survives.
        self.roundtrip(message)
        message.value.type.ClearField("variant")
        self.invalid(message, "variant")
        message.value.CopyFrom(union(variant=False))
        del message.value.type.fields[:]
        self.invalid(message, "fields")

    def test_dynamic_union_array_as_a_variant_selection(self):
        message = minimal_messages()[7]
        field = pv.PVField()
        field.type.CopyFrom(union().type)
        field.type.kind = pv.PV_FIELD_KIND_UNION_ARRAY
        field.type.array_size = pv.PV_ARRAY_SIZE_VARIABLE
        field.union_array.elements.add().value.selected_value.CopyFrom(scalar_field())
        field.union_array.elements.add()  # Null.
        field.union_array.elements.add().value.SetInParent()  # Unselected.
        message.value.value.selected_value.CopyFrom(field)
        self.roundtrip(message)
        field.type.array_size = pv.PV_ARRAY_SIZE_FIXED
        field.type.array_capacity = 3
        message.value.value.selected_value.CopyFrom(field)
        self.invalid(message, "array_size")

    def test_dynamic_structures_and_empty_structure_arrays(self):
        message = minimal_messages()[7]
        field = pv.PVField()
        field.type.kind = pv.PV_FIELD_KIND_STRUCTURE
        field.type.type_id = "custom:reading:1.0"
        for name, value in (("second", 2), ("first", 1)):
            field.type.fields.add(name=name).type.CopyFrom(scalar_field().type)
            field.structure.fields.add().CopyFrom(scalar_field(value=value))
        message.value.CopyFrom(union(selected=field))
        result = self.roundtrip(message)
        self.assertEqual([f.name for f in result.value.value.selected_value.type.fields],
                         ["second", "first"])
        field.structure.fields.pop()
        message.value.value.selected_value.CopyFrom(field)
        self.invalid(message, "structure")
        field.type.kind = pv.PV_FIELD_KIND_STRUCTURE_ARRAY
        field.type.array_size = pv.PV_ARRAY_SIZE_VARIABLE
        field.ClearField("structure")
        field.structure_array.SetInParent()
        message.value.value.selected_value.CopyFrom(field)
        self.roundtrip(message)
        field.structure_array.elements.add()  # Null structure.
        row = field.structure_array.elements.add().value
        row.fields.add().CopyFrom(scalar_field(value=2))
        row.fields.add().CopyFrom(scalar_field(value=1))
        message.value.value.selected_value.CopyFrom(field)
        self.roundtrip(message)

    def test_dynamic_scalar_array_bounds_and_type_mismatch(self):
        message = minimal_messages()[7]
        field = pv.PVField()
        field.type.kind = pv.PV_FIELD_KIND_SCALAR_ARRAY
        field.type.scalar_type = pv.PV_SCALAR_TYPE_BYTE
        field.type.array_size = pv.PV_ARRAY_SIZE_FIXED
        field.type.array_capacity = 2
        field.scalar_array.byte_value.value.extend([1, 2])
        message.value.value.selected_value.CopyFrom(field)
        self.roundtrip(message)
        field.scalar_array.byte_value.value.append(3)
        message.value.value.selected_value.CopyFrom(field)
        self.invalid(message, "array_capacity")
        field.type.array_size = pv.PV_ARRAY_SIZE_BOUNDED
        field.type.array_capacity = 3
        message.value.value.selected_value.CopyFrom(field)
        self.roundtrip(message)
        field.type.scalar_type = pv.PV_SCALAR_TYPE_SHORT
        message.value.value.selected_value.CopyFrom(field)
        self.invalid(message, "scalar_type")

    def test_extensions_roundtrip_and_reject_canonical_name_collisions(self):
        message = minimal_messages()[0]
        message.extra_fields.add(name="custom").value.CopyFrom(scalar_field(value=123))
        self.roundtrip(message)
        message.extra_fields[0].name = "value"
        self.invalid(message, "name")
        message.extra_fields[0].name = "custom"
        message.extra_fields.add().CopyFrom(message.extra_fields[0])
        self.invalid(message, "name")
        message.extra_fields.pop()
        for name in ("descriptor", "timeStamp"):
            message.extra_fields[0].name = name
            self.invalid(message, "name")

    def test_multichannel_independent_metadata_and_lengths(self):
        message = minimal_messages()[8]
        message.severity.value.append(0)
        message.user_tag.value.append(99)
        self.roundtrip(message)
        message.status.SetInParent()
        self.invalid(message, "status")
        message.status.value.append(0)
        self.roundtrip(message)
        message.channel_name.value.append("extra")
        self.invalid(message, "channel_name")

    def test_image_numeric_alternatives_and_sizes(self):
        widths = {"boolean_value": 1, "byte_value": 1, "ubyte_value": 1,
                  "short_value": 2, "ushort_value": 2, "int_value": 4,
                  "uint_value": 4, "long_value": 8, "ulong_value": 8,
                  "float_value": 4, "double_value": 8}
        self.assertEqual(set(nt.NDArrayValue.DESCRIPTOR.fields_by_name), set(widths))
        for case, width in widths.items():
            message = minimal_messages()[9]
            message.value.Clear()
            getattr(message.value, case).value.extend([0, 1, 0, 1])
            message.compressed_size = 4 * width
            message.uncompressed_size = 4 * width
            self.roundtrip(message)
        message = minimal_messages()[9]
        message.value.ubyte_value.value.append(3)  # Uncompressed padding is permitted.
        message.compressed_size = 5
        self.roundtrip(message)
        message.compressed_size = 4
        self.invalid(message, "compressed_size")
        message.compressed_size = 5
        message.uncompressed_size = 5
        self.invalid(message, "uncompressed_size")
        message.uncompressed_size = 4
        del message.dimension.value[:]
        message.uncompressed_size = 0
        self.roundtrip(message)

    def test_compressed_image_is_codec_dependent(self):
        message = minimal_messages()[9]
        message.codec.name = "example:compressed"
        message.value.ubyte_value.value[:] = [1]
        message.compressed_size = 1
        message.uncompressed_size = 32  # Decoded type need not match stored ubyte.
        self.roundtrip(message)
        message.codec.parameters.CopyFrom(union(variant=False))
        self.invalid(message, "variant")

    def test_image_required_nested_fields_and_short_raw_buffer(self):
        message = minimal_messages()[9]
        for name in ("size", "offset", "full_size", "binning", "reverse"):
            copy = nt.NTNDArray()
            copy.CopyFrom(message)
            copy.dimension.value[0].ClearField(name)
            self.invalid(copy, name)
        message.dimension.value[0].size = -1
        self.invalid(message, "size")
        message.dimension.value[0].size = 2
        message.value.ubyte_value.value.pop()
        message.compressed_size = 3
        self.invalid(message, "value")
        message.codec.ClearField("name")
        self.invalid(message, "name")
        message.codec.name = ""
        message.codec.ClearField("parameters")
        self.invalid(message, "parameters")

    def test_image_attribute_requires_source_and_descriptor(self):
        message = minimal_messages()[9]
        attribute = message.attribute.value.add()
        attribute.CopyFrom(minimal_messages()[7])
        self.invalid(message, "descriptor_text")
        attribute.descriptor_text = ""
        self.invalid(message, "source_type")
        attribute.source_type = 0
        self.invalid(message, "source")
        attribute.source = ""
        self.roundtrip(message)
        attribute.source_type = 4
        self.invalid(message, "source_type")

    def test_continuum_point_interleaving_and_empty_policy(self):
        message = minimal_messages()[10]
        self.roundtrip(message)
        message.value.value.pop()
        self.invalid(message, "value")
        message.value.value.append(21)
        message.units.value.pop()
        self.invalid(message, "units")
        del message.base.value[:]
        self.invalid(message, "base")
        del message.value.value[:]
        self.roundtrip(message)  # Two units declare base and one component.
        message.units.value.pop()
        self.invalid(message, "units")

    def test_histogram_signed_widths_and_boundaries(self):
        for case in ("short_value", "int_value", "long_value"):
            message = minimal_messages()[11]
            message.value.Clear()
            getattr(message.value, case).value.extend([-1, 2])
            self.roundtrip(message)
            message.ranges.value.pop()
            self.invalid(message, "ranges")
        self.assertEqual(set(nt.HistogramValue.DESCRIPTOR.fields_by_name),
                         {"short_value", "int_value", "long_value"})

    def test_aggregate_independent_optional_fields(self):
        message = minimal_messages()[12]
        message.first = 0
        message.max = 0
        message.first_time_stamp.CopyFrom(time())
        result = self.roundtrip(message)
        self.assertTrue(result.HasField("first"))
        self.assertFalse(result.HasField("last"))
        message.n = -1
        self.invalid(message, "n")

    def test_validator_rejects_unsupported_messages_and_depth(self):
        self.invalid(types.Scalar(int_value=1), "unsupported")
        nested = scalar_field()
        for _ in range(10):
            wrapper = pv.PVField()
            wrapper.type.kind = pv.PV_FIELD_KIND_UNION
            wrapper.type.type_id = "any"
            wrapper.type.variant = True
            wrapper.union.selected_value.CopyFrom(nested)
            nested = wrapper
        message = minimal_messages()[7]
        message.value.value.selected_value.CopyFrom(nested)
        with self.assertRaisesRegex(validate.ValidationError, "depth"):
            validate.validate(message, max_depth=4)

    def test_binary_unknown_fields_survive_reserialization(self):
        message = minimal_messages()[0]
        # Add unknown varint field 200 with value 7. No JSON guarantee is implied.
        unknown = bytes([0xC0, 0x0C, 0x07])
        result = type(message).FromString(message.SerializeToString() + unknown)
        validate.validate(result)
        self.assertIn(unknown, result.SerializeToString())


if __name__ == "__main__":
    try:
        types, pv, nt = generate_bindings()
        # The validator resolves generated types through descriptors, not imports.
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        validate = importlib.import_module("validate")
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(ContractTests)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        print(f"Generated Python, Java, C++ and descriptors using {os.environ.get('PROTOC', 'protoc')}")
        sys.exit(0 if result.wasSuccessful() else 1)
    finally:
        TEMP.cleanup()

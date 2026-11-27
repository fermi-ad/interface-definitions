use epics_nt::FILE_DESCRIPTOR_SET;
use prost::Message;
use prost_types::{DescriptorProto, FileDescriptorSet};

fn messages() -> Vec<DescriptorProto> {
    FileDescriptorSet::decode(FILE_DESCRIPTOR_SET)
        .unwrap()
        .file
        .into_iter()
        .filter(|file| file.package.as_deref() == Some("common.epics.v1"))
        .flat_map(|file| file.message_type)
        .collect()
}

#[test]
fn all_thirteen_message_field_names_order_tags_and_presence_are_stable() {
    let expected = [
        (
            "NTScalar",
            "value descriptor_text alarm time_stamp display control",
        ),
        (
            "NTScalarArray",
            "value descriptor_text alarm time_stamp display control",
        ),
        ("NTEnum", "value descriptor_text alarm time_stamp"),
        (
            "NTMatrix",
            "value dim descriptor_text alarm time_stamp display",
        ),
        ("NTURI", "scheme authority path query"),
        ("NTNameValue", "name value descriptor_text alarm time_stamp"),
        ("NTTable", "labels value descriptor_text alarm time_stamp"),
        (
            "NTAttribute",
            "name value tags descriptor_text alarm time_stamp source_type source",
        ),
        (
            "NTMultiChannel",
            "value channel_name descriptor_text alarm time_stamp severity status message seconds_past_epoch nanoseconds user_tag",
        ),
        (
            "NTNDArray",
            "value codec compressed_size uncompressed_size dimension unique_id data_time_stamp attribute descriptor_text alarm time_stamp display",
        ),
        (
            "NTContinuum",
            "base value units descriptor_text alarm time_stamp",
        ),
        (
            "NTHistogram",
            "ranges value descriptor_text alarm time_stamp",
        ),
        (
            "NTAggregate",
            "value n dispersion first first_time_stamp last last_time_stamp max min descriptor_text alarm time_stamp",
        ),
    ];
    let messages = messages();
    assert_eq!(
        messages
            .iter()
            .filter(|v| v.name.as_ref().unwrap().starts_with("NT"))
            .count(),
        13
    );
    for (name, fields) in expected {
        let message = messages
            .iter()
            .find(|m| m.name.as_deref() == Some(name))
            .unwrap();
        let expected: Vec<_> = fields
            .split_whitespace()
            .chain(["type_id", "extra_fields"])
            .collect();
        assert_eq!(
            message
                .field
                .iter()
                .map(|f| f.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            expected,
            "{name}"
        );
        let tags: Vec<_> = (1..=(message.field.len() as i32 - 2))
            .chain([100, 101])
            .collect();
        assert_eq!(
            message
                .field
                .iter()
                .map(|f| f.number.unwrap())
                .collect::<Vec<_>>(),
            tags,
            "{name}"
        );
        for field in &message.field[..message.field.len() - 1] {
            assert!(
                field.r#type == Some(prost_types::field_descriptor_proto::Type::Message as i32)
                    || field.proto3_optional == Some(true),
                "{name}.{} lost presence",
                field.name.as_ref().unwrap()
            );
        }
    }
}

#[test]
fn all_scalar_discriminator_tags_and_metadata_shapes_are_stable() {
    let messages = messages();
    let cases = [
        "boolean_value",
        "byte_value",
        "ubyte_value",
        "short_value",
        "ushort_value",
        "int_value",
        "uint_value",
        "long_value",
        "ulong_value",
        "float_value",
        "double_value",
        "string_value",
    ];
    for name in ["Scalar", "ScalarArray"] {
        let message = messages
            .iter()
            .find(|m| m.name.as_deref() == Some(name))
            .unwrap();
        assert_eq!(message.field.len(), 12);
        for (index, field) in message.field.iter().enumerate() {
            assert_eq!(field.name.as_deref(), Some(cases[index]));
            assert_eq!(field.number, Some(index as i32 + 1));
            assert_eq!(field.oneof_index, Some(0));
        }
    }
    for (name, fields) in [
        ("EnumValue", "index choices"),
        ("Time", "seconds_past_epoch nanoseconds user_tag"),
        ("Alarm", "severity status message"),
        (
            "AlarmLimit",
            "active low_alarm_limit low_warning_limit high_warning_limit high_alarm_limit low_alarm_severity low_warning_severity high_warning_severity high_alarm_severity hysteresis",
        ),
        (
            "Display",
            "limit_low limit_high description units precision form",
        ),
        ("Control", "limit_low limit_high min_step"),
    ] {
        let message = messages
            .iter()
            .find(|m| m.name.as_deref() == Some(name))
            .unwrap();
        assert_eq!(
            message
                .field
                .iter()
                .map(|f| f.name.as_deref().unwrap())
                .collect::<Vec<_>>(),
            fields.split_whitespace().collect::<Vec<_>>()
        );
        for field in &message.field {
            assert!(
                field.proto3_optional == Some(true)
                    || field.r#type
                        == Some(prost_types::field_descriptor_proto::Type::Message as i32)
            );
        }
    }
    assert_eq!(
        messages
            .iter()
            .find(|m| m.name.as_deref() == Some("NDArrayValue"))
            .unwrap()
            .field
            .len(),
        11
    );
    assert_eq!(
        messages
            .iter()
            .find(|m| m.name.as_deref() == Some("HistogramValue"))
            .unwrap()
            .field
            .len(),
        3
    );
}

#[test]
fn java_generator_runtime_stays_aligned_with_legacy_modules() {
    let epics = include_str!("../../../java/controls-epics/pom.xml");
    let version = epics
        .split("<protobuf.version>")
        .nth(1)
        .unwrap()
        .split("</protobuf.version>")
        .next()
        .unwrap();
    for module in [
        include_str!("../../../java/controls-daq/pom.xml"),
        include_str!("../../../java/controls-dpm/pom.xml"),
    ] {
        let dependency = module
            .split("<artifactId>protobuf-java</artifactId>")
            .nth(1)
            .unwrap();
        let runtime = dependency
            .split("<version>")
            .nth(1)
            .unwrap()
            .split("</version>")
            .next()
            .unwrap();
        assert_eq!(
            version, runtime,
            "new Java classes must load with the DPM/DAQ runtime"
        );
    }
}

#[test]
fn compiler_keeps_java_packages_and_outer_descriptor_classes_separate() {
    let files = FileDescriptorSet::decode(FILE_DESCRIPTOR_SET).unwrap().file;
    let mut outer = std::collections::HashSet::new();
    for file in files
        .iter()
        .filter(|f| f.package.as_deref() == Some("common.epics.v1"))
    {
        let options = file.options.as_ref().unwrap();
        assert_eq!(
            options.java_package.as_deref(),
            Some("gov.fnal.controls.common.epics.v1")
        );
        assert_eq!(options.java_multiple_files, Some(true));
        assert!(outer.insert(options.java_outer_classname.as_deref().unwrap()));
    }
    assert_eq!(outer.len(), 3);
}

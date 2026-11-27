mod support;

use epics_nt::{Validate, proto::*};
use prost::Message;
use support::*;

#[test]
fn all_thirteen_types_roundtrip() {
    roundtrip(&scalar_message());
    roundtrip(&array_message());
    roundtrip(&enum_message());
    roundtrip(&matrix());
    roundtrip(&uri());
    roundtrip(&name_value());
    roundtrip(&table());
    roundtrip(&attribute());
    roundtrip(&channels());
    roundtrip(&image());
    roundtrip(&continuum());
    roundtrip(&histogram());
    roundtrip(&aggregate());
}

#[test]
fn every_required_field_rejects_missing() {
    macro_rules! missing {
        ($constructor:ident, $($field:ident),+) => { $(
            let mut message = $constructor();
            message.$field = None;
            invalid(&message, stringify!($field));
        )+ };
    }
    missing!(scalar_message, value, type_id);
    missing!(array_message, value, type_id);
    missing!(enum_message, value, type_id);
    missing!(matrix, value, type_id);
    missing!(uri, scheme, path, type_id);
    missing!(name_value, name, value, type_id);
    missing!(table, labels, value, type_id);
    missing!(attribute, name, value, type_id);
    missing!(channels, value, channel_name, type_id);
    missing!(
        image,
        value,
        codec,
        compressed_size,
        uncompressed_size,
        dimension,
        unique_id,
        data_time_stamp,
        attribute,
        type_id
    );
    missing!(continuum, base, value, units, type_id);
    missing!(histogram, ranges, value, type_id);
    missing!(aggregate, value, n, type_id);
}

#[test]
fn identifiers_are_exact_and_case_sensitive() {
    for id in [
        "epics:nt/ntscalar:1.0",
        "epics:nt/NTScalar:2.0",
        "",
        "NTScalar",
    ] {
        let mut message = scalar_message();
        message.type_id = Some(id.into());
        invalid(&message, "type_id");
    }
}

#[test]
fn all_scalar_types_and_integer_boundaries() {
    use scalar::Value as V;
    let cases = vec![
        V::BooleanValue(false),
        V::BooleanValue(true),
        V::ByteValue(-128),
        V::ByteValue(127),
        V::UbyteValue(0),
        V::UbyteValue(255),
        V::ShortValue(-32768),
        V::ShortValue(32767),
        V::UshortValue(65535),
        V::IntValue(i32::MIN),
        V::IntValue(i32::MAX),
        V::UintValue(u32::MAX),
        V::LongValue(i64::MIN),
        V::LongValue(i64::MAX),
        V::UlongValue(u64::MAX),
        V::FloatValue(1.5),
        V::DoubleValue(1.5),
        V::StringValue(String::new()),
        V::StringValue("μA".into()),
    ];
    for value in cases {
        let mut message = scalar_message();
        message.value.as_mut().unwrap().value = Some(value);
        roundtrip(&message);
    }
}

#[test]
fn all_homogeneous_array_types_include_selected_empty_arrays() {
    use scalar_array::Value as V;
    let cases = vec![
        V::BooleanValue(BooleanArray {
            value: vec![false, true],
        }),
        V::ByteValue(ByteArray {
            value: vec![-128, 127],
        }),
        V::UbyteValue(UByteArray {
            value: vec![0, 255],
        }),
        V::ShortValue(ShortArray {
            value: vec![-32768, 32767],
        }),
        V::UshortValue(UShortArray {
            value: vec![0, 65535],
        }),
        V::IntValue(IntArray {
            value: vec![i32::MIN, i32::MAX],
        }),
        V::UintValue(UIntArray {
            value: vec![0, u32::MAX],
        }),
        V::LongValue(LongArray {
            value: vec![i64::MIN, i64::MAX],
        }),
        V::UlongValue(ULongArray {
            value: vec![0, u64::MAX],
        }),
        V::FloatValue(FloatArray {
            value: vec![-0., 1.5],
        }),
        V::DoubleValue(DoubleArray {
            value: vec![-0., 1.5],
        }),
        V::StringValue(StringArray {
            value: vec![String::new(), "μA".into()],
        }),
    ];
    for value in cases {
        let mut message = array_message();
        message.value.as_mut().unwrap().value = Some(value.clone());
        roundtrip(&message);
        let empty = match value {
            V::BooleanValue(_) => V::BooleanValue(BooleanArray::default()),
            V::ByteValue(_) => V::ByteValue(ByteArray::default()),
            V::UbyteValue(_) => V::UbyteValue(UByteArray::default()),
            V::ShortValue(_) => V::ShortValue(ShortArray::default()),
            V::UshortValue(_) => V::UshortValue(UShortArray::default()),
            V::IntValue(_) => V::IntValue(IntArray::default()),
            V::UintValue(_) => V::UintValue(UIntArray::default()),
            V::LongValue(_) => V::LongValue(LongArray::default()),
            V::UlongValue(_) => V::UlongValue(ULongArray::default()),
            V::FloatValue(_) => V::FloatValue(FloatArray::default()),
            V::DoubleValue(_) => V::DoubleValue(DoubleArray::default()),
            V::StringValue(_) => V::StringValue(StringArray::default()),
        };
        message.value.as_mut().unwrap().value = Some(empty);
        roundtrip(&message);
    }
}

#[test]
fn narrow_ranges_are_checked_for_scalars_and_arrays() {
    for (value, path) in [
        (scalar::Value::ByteValue(-129), "byte_value"),
        (scalar::Value::ByteValue(128), "byte_value"),
        (scalar::Value::UbyteValue(256), "ubyte_value"),
        (scalar::Value::ShortValue(-32769), "short_value"),
        (scalar::Value::ShortValue(32768), "short_value"),
        (scalar::Value::UshortValue(65536), "ushort_value"),
    ] {
        let array = match &value {
            scalar::Value::ByteValue(v) => {
                scalar_array::Value::ByteValue(ByteArray { value: vec![*v] })
            }
            scalar::Value::UbyteValue(v) => {
                scalar_array::Value::UbyteValue(UByteArray { value: vec![*v] })
            }
            scalar::Value::ShortValue(v) => {
                scalar_array::Value::ShortValue(ShortArray { value: vec![*v] })
            }
            scalar::Value::UshortValue(v) => {
                scalar_array::Value::UshortValue(UShortArray { value: vec![*v] })
            }
            _ => unreachable!(),
        };
        let mut scalar = scalar_message();
        scalar.value.as_mut().unwrap().value = Some(value);
        invalid(&scalar, path);
        let mut message = array_message();
        message.value.as_mut().unwrap().value = Some(array);
        invalid(&message, path);
    }
}

#[test]
fn unselected_scalar_is_not_zero_or_empty_array() {
    let mut scalar = scalar_message();
    scalar.value.as_mut().unwrap().value = None;
    invalid(&scalar, "value");
    let mut array = array_message();
    array.value.as_mut().unwrap().value = None;
    invalid(&array, "value");
}

#[test]
fn floating_special_values_and_signed_zero_are_not_normalized() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0] {
        let mut message = scalar_message();
        message.value.as_mut().unwrap().value = Some(scalar::Value::DoubleValue(value));
        message.validate().unwrap();
        let restored = NtScalar::decode(message.encode_to_vec().as_slice()).unwrap();
        restored.validate().unwrap();
        let Some(scalar::Value::DoubleValue(actual)) = restored.value.unwrap().value else {
            panic!("lost type")
        };
        assert!(if value.is_nan() {
            actual.is_nan()
        } else {
            actual.to_bits() == value.to_bits()
        });
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.0] {
        let mut message = scalar_message();
        message.value.as_mut().unwrap().value = Some(scalar::Value::FloatValue(value));
        message.validate().unwrap();
        let restored = NtScalar::decode(message.encode_to_vec().as_slice()).unwrap();
        restored.validate().unwrap();
        let Some(scalar::Value::FloatValue(actual)) = restored.value.unwrap().value else {
            panic!("lost type")
        };
        assert!(if value.is_nan() {
            actual.is_nan()
        } else {
            actual.to_bits() == value.to_bits()
        });
    }
}

#[test]
fn absent_and_present_defaults_and_metadata_roundtrip() {
    let mut message = scalar_message();
    assert_eq!(roundtrip(&message).descriptor_text, None);
    message.descriptor_text = Some(String::new());
    message.alarm = Some(alarm());
    message.time_stamp = Some(time());
    message.display = Some(display());
    message.control = Some(Control {
        limit_low: Some(0.),
        limit_high: Some(0.),
        min_step: Some(0.),
    });
    let copy = roundtrip(&message);
    assert_eq!(copy.descriptor_text, Some(String::new()));
    assert_eq!(copy.time_stamp.unwrap().nanoseconds, Some(-5));
}

#[test]
fn optional_array_wrapper_distinguishes_absent_from_empty() {
    let mut message = attribute();
    assert!(message.tags.is_none());
    message.tags = Some(StringArray::default());
    assert!(roundtrip(&message).tags.is_some());
}

#[test]
fn metadata_missing_fields_invalid_codes_and_enum_indices() {
    let mut message = scalar_message();
    message.alarm = Some(alarm());
    message.alarm.as_mut().unwrap().severity = Some(5);
    invalid(&message, "severity");
    message.alarm.as_mut().unwrap().severity = Some(0);
    message.alarm.as_mut().unwrap().status = Some(-1);
    invalid(&message, "status");
    message.alarm = Some(Alarm::default());
    invalid(&message, "severity");
    message.alarm = None;
    message.time_stamp = Some(time());
    message.time_stamp.as_mut().unwrap().user_tag = None;
    invalid(&message, "user_tag");
    message.time_stamp = None;
    message.display = Some(display());
    message
        .display
        .as_mut()
        .unwrap()
        .form
        .as_mut()
        .unwrap()
        .choices
        .as_mut()
        .unwrap()
        .value[0] = "other".into();
    invalid(&message, "choices");
    let mut enumeration = enum_message();
    enumeration.value.as_mut().unwrap().index = Some(2);
    invalid(&enumeration, "index");
}

#[test]
fn matrix_vectors_and_invalid_dimensions() {
    let mut message = matrix();
    message.dim = None;
    roundtrip(&message);
    message.dim = Some(IntArray { value: vec![4] });
    roundtrip(&message);
    for dim in [vec![], vec![0], vec![-1], vec![3], vec![1, 2, 2]] {
        message.dim = Some(IntArray { value: dim });
        invalid(&message, "dim");
    }
}

#[test]
fn uri_ordered_typed_queries_empty_and_absent() {
    let mut message = uri();
    message.query = Some(UriQuery {
        arguments: vec![
            UriQueryArgument {
                name: Some("start".into()),
                value: Some(uri_query_argument::Value::StringValue("now".into())),
            },
            UriQueryArgument {
                name: Some("count".into()),
                value: Some(uri_query_argument::Value::IntValue(0)),
            },
            UriQueryArgument {
                name: Some("scale".into()),
                value: Some(uri_query_argument::Value::DoubleValue(1.5)),
            },
        ],
    });
    roundtrip(&message);
    message.query.as_mut().unwrap().arguments[1].name = Some("start".into());
    invalid(&message, "name");
    message.query.as_mut().unwrap().arguments[1].name = Some("count".into());
    message.query.as_mut().unwrap().arguments[1].value = None;
    invalid(&message, "value");
    message.query = Some(UriQuery::default());
    message.authority = Some(String::new());
    roundtrip(&message);
}

#[test]
fn uri_requires_rfc3986_scheme() {
    let mut message = uri();
    for scheme in ["", "1pva", "+pva", "pva with spaces", "μva"] {
        message.scheme = Some(scheme.into());
        invalid(&message, "scheme");
    }
    for scheme in ["pva", "PVA", "example+v1.2-test"] {
        message.scheme = Some(scheme.into());
        roundtrip(&message);
    }
}

#[test]
fn name_value_association_is_positional() {
    let mut message = name_value();
    message.name.as_mut().unwrap().value.push("a".into());
    invalid(&message, "name");
    let Some(scalar_array::Value::LongValue(values)) = &mut message.value.as_mut().unwrap().value
    else {
        panic!()
    };
    values.value.push(0);
    roundtrip(&message);
}

#[test]
fn tables_preserve_column_order_and_labels_not_field_names() {
    let mut message = table();
    message
        .labels
        .as_mut()
        .unwrap()
        .value
        .push("Other heading".into());
    message.value.as_mut().unwrap().columns.push(TableColumn {
        name: Some("other".into()),
        value: Some(ScalarArray {
            value: Some(scalar_array::Value::IntValue(IntArray { value: vec![1] })),
        }),
    });
    roundtrip(&message);
    let Some(scalar_array::Value::IntValue(values)) = &mut message.value.as_mut().unwrap().columns
        [1]
    .value
    .as_mut()
    .unwrap()
    .value
    else {
        panic!()
    };
    values.value.push(2);
    invalid(&message, "columns");
    message.value.as_mut().unwrap().columns[1].value = Some(ScalarArray {
        value: Some(scalar_array::Value::IntValue(IntArray { value: vec![1] })),
    });
    message.value.as_mut().unwrap().columns[1].name = Some("field".into());
    invalid(&message, "name");
    message.value.as_mut().unwrap().columns.clear();
    invalid(&message, "labels");
    message.labels.as_mut().unwrap().value.clear();
    roundtrip(&message);
}

#[test]
fn variant_union_selected_and_unselected() {
    let mut message = attribute();
    message.value = Some(union(true, None));
    roundtrip(&message);
    message.value = Some(union(
        true,
        Some(scalar_field(scalar::Value::UlongValue(u64::MAX))),
    ));
    roundtrip(&message);
    message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .selected_name = Some("not-named".into());
    invalid(&message, "selected_name");
    message.value = Some(union(false, None));
    invalid(&message, "variant");
}

#[test]
fn regular_union_declarations_selection_null_and_unselected_array_elements() {
    let mut message = channels();
    let regular = union(false, Some(int_field()));
    let mut descriptor = regular.r#type.unwrap();
    descriptor.kind = PvFieldKind::UnionArray as i32;
    descriptor.array_size = Some(PvArraySize::Variable as i32);
    let channel_values = message.value.as_mut().unwrap();
    channel_values.r#type = Some(descriptor);
    channel_values.value.as_mut().unwrap().elements[0].value = regular.value;
    roundtrip(&message);
    message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .elements[0]
        .value
        .as_mut()
        .unwrap()
        .selected_name = Some("unknown".into());
    invalid(&message, "selected_name");
    let selection = message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .elements[0]
        .value
        .as_mut()
        .unwrap();
    selection.selected_name = Some("reading".into());
    selection.selected_value = Some(Box::new(scalar_field(scalar::Value::DoubleValue(1.5))));
    invalid(&message, "selected_value");
    message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .elements[0]
        .value = Some(PvUnionValue::default());
    roundtrip(&message);
    message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .elements[0]
        .value = None;
    roundtrip(&message);
    message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .elements
        .clear();
    message.channel_name.as_mut().unwrap().value.clear();
    roundtrip(&message);
    let descriptor = message.value.as_mut().unwrap().r#type.as_mut().unwrap();
    descriptor.fields.push(descriptor.fields[0].clone());
    invalid(&message, "name");
}

#[test]
fn descriptors_reject_inapplicable_missing_and_unspecified_fields() {
    let mut message = attribute();
    let mut field = int_field();
    field.r#type.as_mut().unwrap().type_id = Some("custom".into());
    message.value = Some(union(true, Some(field)));
    invalid(&message, "type_id");
    for field in [
        PvFieldType {
            scalar_type: None,
            ..int_field().r#type.unwrap()
        },
        PvFieldType {
            scalar_type: Some(0),
            ..int_field().r#type.unwrap()
        },
        PvFieldType {
            kind: 100,
            ..int_field().r#type.unwrap()
        },
        PvFieldType {
            variant: Some(false),
            ..int_field().r#type.unwrap()
        },
        PvFieldType {
            array_size: Some(1),
            ..int_field().r#type.unwrap()
        },
    ] {
        let mut value = int_field();
        value.r#type = Some(field);
        message.value = Some(union(true, Some(value)));
        assert!(message.validate().is_err());
    }
    message.value = Some(union(true, None));
    message
        .value
        .as_mut()
        .unwrap()
        .r#type
        .as_mut()
        .unwrap()
        .type_id = None;
    invalid(&message, "type_id");
    message
        .value
        .as_mut()
        .unwrap()
        .r#type
        .as_mut()
        .unwrap()
        .type_id = Some(String::new());
    roundtrip(&message);
    message
        .value
        .as_mut()
        .unwrap()
        .r#type
        .as_mut()
        .unwrap()
        .variant = None;
    invalid(&message, "variant");
    let mut regular = union(false, None);
    regular.r#type.as_mut().unwrap().fields.clear();
    message.value = Some(regular);
    assert!(message.validate().is_err());
}

#[test]
fn dynamic_union_array_selection_preserves_null_and_unselected() {
    let mut descriptor = union(true, None).r#type.unwrap();
    descriptor.kind = PvFieldKind::UnionArray as i32;
    descriptor.array_size = Some(PvArraySize::Variable as i32);
    let field = PvField {
        r#type: Some(descriptor),
        value: Some(pv_field::Value::UnionArray(PvUnionArrayValue {
            elements: vec![
                PvNullableUnionValue {
                    value: union(true, Some(int_field())).value,
                },
                PvNullableUnionValue { value: None },
                PvNullableUnionValue {
                    value: Some(PvUnionValue::default()),
                },
            ],
        })),
    };
    let mut message = attribute();
    message.value = Some(union(true, Some(field)));
    roundtrip(&message);
    let field = message
        .value
        .as_mut()
        .unwrap()
        .value
        .as_mut()
        .unwrap()
        .selected_value
        .as_mut()
        .unwrap();
    field.r#type.as_mut().unwrap().array_size = Some(PvArraySize::Fixed as i32);
    field.r#type.as_mut().unwrap().array_capacity = Some(3);
    invalid(&message, "array_size");
}

#[test]
fn dynamic_structures_and_empty_nullable_structure_arrays() {
    let descriptor = PvFieldType {
        kind: PvFieldKind::Structure as i32,
        type_id: Some("custom:reading:1.0".into()),
        fields: vec![
            PvNamedType {
                name: Some("second".into()),
                r#type: int_field().r#type,
            },
            PvNamedType {
                name: Some("first".into()),
                r#type: int_field().r#type,
            },
        ],
        ..Default::default()
    };
    let values = PvStructureValue {
        fields: vec![
            scalar_field(scalar::Value::IntValue(2)),
            scalar_field(scalar::Value::IntValue(1)),
        ],
    };
    let field = PvField {
        r#type: Some(descriptor),
        value: Some(pv_field::Value::Structure(values.clone())),
    };
    let mut message = attribute();
    message.value = Some(union(true, Some(field.clone())));
    roundtrip(&message);
    let mut bad = field.clone();
    let Some(pv_field::Value::Structure(v)) = &mut bad.value else {
        panic!()
    };
    v.fields.pop();
    message.value = Some(union(true, Some(bad)));
    invalid(&message, "structure");
    let mut array = field;
    array.r#type.as_mut().unwrap().kind = PvFieldKind::StructureArray as i32;
    array.r#type.as_mut().unwrap().array_size = Some(PvArraySize::Variable as i32);
    array.value = Some(pv_field::Value::StructureArray(
        PvStructureArrayValue::default(),
    ));
    message.value = Some(union(true, Some(array.clone())));
    roundtrip(&message);
    array.value = Some(pv_field::Value::StructureArray(PvStructureArrayValue {
        elements: vec![
            PvNullableStructureValue { value: None },
            PvNullableStructureValue {
                value: Some(values),
            },
        ],
    }));
    message.value = Some(union(true, Some(array)));
    roundtrip(&message);
}

#[test]
fn fixed_and_bounded_scalar_array_policy_and_type_mismatch() {
    let mut field = PvField {
        r#type: Some(PvFieldType {
            kind: PvFieldKind::ScalarArray as i32,
            scalar_type: Some(PvScalarType::Byte as i32),
            array_size: Some(PvArraySize::Fixed as i32),
            array_capacity: Some(2),
            ..Default::default()
        }),
        value: Some(pv_field::Value::ScalarArray(ScalarArray {
            value: Some(scalar_array::Value::ByteValue(ByteArray {
                value: vec![1, 2],
            })),
        })),
    };
    let mut message = attribute();
    message.value = Some(union(true, Some(field.clone())));
    roundtrip(&message);
    let Some(pv_field::Value::ScalarArray(v)) = &mut field.value else {
        panic!()
    };
    let Some(scalar_array::Value::ByteValue(v)) = &mut v.value else {
        panic!()
    };
    v.value.push(3);
    message.value = Some(union(true, Some(field.clone())));
    invalid(&message, "array_capacity");
    field.r#type.as_mut().unwrap().array_size = Some(PvArraySize::Bounded as i32);
    field.r#type.as_mut().unwrap().array_capacity = Some(3);
    message.value = Some(union(true, Some(field.clone())));
    roundtrip(&message);
    field.r#type.as_mut().unwrap().scalar_type = Some(PvScalarType::Short as i32);
    message.value = Some(union(true, Some(field)));
    invalid(&message, "scalar_type");
}

#[test]
fn extensions_preserve_order_and_reject_duplicate_and_canonical_names() {
    let mut message = scalar_message();
    message.extra_fields = vec![PvNamedField {
        name: Some("custom".into()),
        value: Some(int_field()),
    }];
    roundtrip(&message);
    for name in ["value", "descriptor", "timeStamp"] {
        message.extra_fields[0].name = Some(name.into());
        invalid(&message, "name");
    }
    message.extra_fields[0].name = Some("custom".into());
    message.extra_fields.push(message.extra_fields[0].clone());
    invalid(&message, "name");
}

#[test]
fn multichannel_metadata_arrays_are_independent_and_aligned() {
    let mut message = channels();
    message.severity = Some(IntArray { value: vec![0] });
    message.user_tag = Some(IntArray { value: vec![99] });
    roundtrip(&message);
    message.status = Some(IntArray::default());
    invalid(&message, "status");
    message.status = Some(IntArray { value: vec![0] });
    roundtrip(&message);
    message
        .channel_name
        .as_mut()
        .unwrap()
        .value
        .push("extra".into());
    invalid(&message, "channel_name");
}

#[test]
fn image_all_numeric_discriminators_and_logical_byte_widths() {
    use nd_array_value::Value as V;
    let cases = vec![
        (
            V::BooleanValue(BooleanArray {
                value: vec![false; 4],
            }),
            1,
        ),
        (V::ByteValue(ByteArray { value: vec![0; 4] }), 1),
        (V::UbyteValue(UByteArray { value: vec![0; 4] }), 1),
        (V::ShortValue(ShortArray { value: vec![0; 4] }), 2),
        (V::UshortValue(UShortArray { value: vec![0; 4] }), 2),
        (V::IntValue(IntArray { value: vec![0; 4] }), 4),
        (V::UintValue(UIntArray { value: vec![0; 4] }), 4),
        (V::LongValue(LongArray { value: vec![0; 4] }), 8),
        (V::UlongValue(ULongArray { value: vec![0; 4] }), 8),
        (V::FloatValue(FloatArray { value: vec![0.; 4] }), 4),
        (V::DoubleValue(DoubleArray { value: vec![0.; 4] }), 8),
    ];
    for (value, width) in cases {
        let mut message = image();
        message.value.as_mut().unwrap().value = Some(value);
        message.compressed_size = Some(4 * width);
        message.uncompressed_size = Some(4 * width);
        roundtrip(&message);
    }
}

#[test]
fn raw_image_padding_sizes_and_empty_dimensions() {
    let mut message = image();
    let Some(nd_array_value::Value::UbyteValue(v)) = &mut message.value.as_mut().unwrap().value
    else {
        panic!()
    };
    v.value.push(3);
    message.compressed_size = Some(5);
    roundtrip(&message);
    message.compressed_size = Some(4);
    invalid(&message, "compressed_size");
    message.compressed_size = Some(5);
    message.uncompressed_size = Some(5);
    invalid(&message, "uncompressed_size");
    message.dimension.as_mut().unwrap().value.clear();
    message.uncompressed_size = Some(0);
    roundtrip(&message);
}

#[test]
fn compressed_image_content_remains_codec_dependent() {
    let mut message = image();
    message.codec.as_mut().unwrap().name = Some("example:compressed".into());
    message.value.as_mut().unwrap().value = Some(nd_array_value::Value::UbyteValue(UByteArray {
        value: vec![1],
    }));
    message.compressed_size = Some(1);
    message.uncompressed_size = Some(32);
    roundtrip(&message);
    message.codec.as_mut().unwrap().parameters = Some(union(false, None));
    invalid(&message, "variant");
}

#[test]
fn image_nested_required_fields_short_buffer_and_dimension_overflow() {
    macro_rules! missing { ($($field:ident),+) => { $(let mut message = image(); message.dimension.as_mut().unwrap().value[0].$field = None; invalid(&message, stringify!($field));)+ }; }
    missing!(size, offset, full_size, binning, reverse);
    let mut message = image();
    message.dimension.as_mut().unwrap().value[0].size = Some(-1);
    invalid(&message, "size");
    message = image();
    message.value.as_mut().unwrap().value = Some(nd_array_value::Value::UbyteValue(UByteArray {
        value: vec![1; 3],
    }));
    message.compressed_size = Some(3);
    invalid(&message, "value");
    message = image();
    message.codec.as_mut().unwrap().name = None;
    invalid(&message, "name");
    message.codec.as_mut().unwrap().name = Some(String::new());
    message.codec.as_mut().unwrap().parameters = None;
    invalid(&message, "parameters");
    message = image();
    message.dimension.as_mut().unwrap().value = vec![
        Dimension {
            size: Some(i32::MAX),
            ..message.dimension.as_ref().unwrap().value[0]
        };
        4
    ];
    invalid(&message, "dimension");
    let zero = Dimension {
        size: Some(0),
        ..message.dimension.as_ref().unwrap().value[0]
    };
    message.dimension.as_mut().unwrap().value.push(zero);
    message.uncompressed_size = Some(0);
    roundtrip(&message);
}

#[test]
fn image_attributes_require_descriptor_and_source_fields() {
    let mut message = image();
    message.attribute.as_mut().unwrap().value.push(attribute());
    invalid(&message, "descriptor_text");
    message.attribute.as_mut().unwrap().value[0].descriptor_text = Some(String::new());
    invalid(&message, "source_type");
    message.attribute.as_mut().unwrap().value[0].source_type = Some(0);
    invalid(&message, "source");
    message.attribute.as_mut().unwrap().value[0].source = Some(String::new());
    roundtrip(&message);
    message.attribute.as_mut().unwrap().value[0].source_type = Some(4);
    invalid(&message, "source_type");
}

#[test]
fn continuum_point_interleaving_and_empty_policy() {
    let mut message = continuum();
    message.value.as_mut().unwrap().value.pop();
    invalid(&message, "value");
    message.value.as_mut().unwrap().value.push(21.);
    message.units.as_mut().unwrap().value.pop();
    invalid(&message, "units");
    message.base.as_mut().unwrap().value.clear();
    invalid(&message, "base");
    message.value.as_mut().unwrap().value.clear();
    roundtrip(&message);
    message.units.as_mut().unwrap().value.pop();
    invalid(&message, "units");
}

#[test]
fn histogram_signed_widths_and_boundary_count() {
    for value in [
        histogram_value::Value::ShortValue(ShortArray { value: vec![-1, 2] }),
        histogram_value::Value::IntValue(IntArray { value: vec![-1, 2] }),
        histogram_value::Value::LongValue(LongArray { value: vec![-1, 2] }),
    ] {
        let mut message = histogram();
        message.value.as_mut().unwrap().value = Some(value);
        roundtrip(&message);
        message.ranges.as_mut().unwrap().value.pop();
        invalid(&message, "ranges");
    }
}

#[test]
fn aggregate_independent_optional_fields_and_nonnegative_count() {
    let mut message = aggregate();
    message.first = Some(0.);
    message.max = Some(0.);
    message.first_time_stamp = Some(time());
    let copy = roundtrip(&message);
    assert_eq!(copy.first, Some(0.));
    assert_eq!(copy.last, None);
    message.n = Some(-1);
    invalid(&message, "n");
}

#[test]
fn validation_depth_is_bounded() {
    let mut nested = int_field();
    for _ in 0..10 {
        nested = PvField {
            r#type: union(true, None).r#type,
            value: Some(pv_field::Value::Union(Box::new(PvUnionValue {
                selected_name: None,
                selected_value: Some(Box::new(nested)),
            }))),
        };
    }
    let mut message = attribute();
    message.value = Some(union(true, Some(nested)));
    assert!(
        message
            .validate_with_limit(4)
            .unwrap_err()
            .reason
            .contains("depth")
    );
    assert!(message.validate_with_limit(0).is_err());
}

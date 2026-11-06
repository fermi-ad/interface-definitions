#![allow(dead_code)]

use epics_nt::{Validate, proto::*};
use prost::Message;
use std::fmt::Debug;

pub fn roundtrip<T>(value: &T) -> T
where
    T: Message + Default + Validate + PartialEq + Debug,
{
    value.validate().unwrap();
    let copy = T::decode(value.encode_to_vec().as_slice()).unwrap();
    assert_eq!(*value, copy);
    copy.validate().unwrap();
    copy
}

pub fn invalid<T: Validate>(value: &T, path: &str) {
    let error = value.validate().unwrap_err();
    assert!(error.path.contains(path), "expected {path}, got {error}");
}

pub fn scalar_field(value: scalar::Value) -> PvField {
    let scalar_type = match &value {
        scalar::Value::BooleanValue(_) => PvScalarType::Boolean,
        scalar::Value::ByteValue(_) => PvScalarType::Byte,
        scalar::Value::UbyteValue(_) => PvScalarType::Ubyte,
        scalar::Value::ShortValue(_) => PvScalarType::Short,
        scalar::Value::UshortValue(_) => PvScalarType::Ushort,
        scalar::Value::IntValue(_) => PvScalarType::Int,
        scalar::Value::UintValue(_) => PvScalarType::Uint,
        scalar::Value::LongValue(_) => PvScalarType::Long,
        scalar::Value::UlongValue(_) => PvScalarType::Ulong,
        scalar::Value::FloatValue(_) => PvScalarType::Float,
        scalar::Value::DoubleValue(_) => PvScalarType::Double,
        scalar::Value::StringValue(_) => PvScalarType::String,
    };
    PvField {
        r#type: Some(PvFieldType {
            kind: PvFieldKind::Scalar as i32,
            scalar_type: Some(scalar_type as i32),
            ..Default::default()
        }),
        value: Some(pv_field::Value::Scalar(Scalar { value: Some(value) })),
    }
}

pub fn int_field() -> PvField {
    scalar_field(scalar::Value::IntValue(0))
}

pub fn union(variant: bool, selected: Option<PvField>) -> PvUnion {
    let fields = if variant {
        vec![]
    } else {
        vec![PvNamedType {
            name: Some("reading".into()),
            r#type: int_field().r#type,
        }]
    };
    PvUnion {
        r#type: Some(PvFieldType {
            kind: PvFieldKind::Union as i32,
            type_id: Some(if variant { "any" } else { "union" }.into()),
            variant: Some(variant),
            fields,
            ..Default::default()
        }),
        value: Some(PvUnionValue {
            selected_name: if !variant && selected.is_some() {
                Some("reading".into())
            } else {
                None
            },
            selected_value: selected.map(Box::new),
        }),
    }
}

pub fn time() -> Time {
    Time {
        seconds_past_epoch: Some(-1),
        nanoseconds: Some(-5),
        user_tag: Some(42),
    }
}
pub fn alarm() -> Alarm {
    Alarm {
        severity: Some(0),
        status: Some(0),
        message: Some(String::new()),
    }
}
pub fn enum_value() -> EnumValue {
    EnumValue {
        index: Some(0),
        choices: Some(StringArray {
            value: vec!["off".into(), "on".into()],
        }),
    }
}
pub fn display() -> Display {
    Display {
        limit_low: Some(0.0),
        limit_high: Some(10.0),
        description: Some("value".into()),
        units: Some("A".into()),
        precision: Some(3),
        form: Some(EnumValue {
            index: Some(0),
            choices: Some(StringArray {
                value: [
                    "Default",
                    "String",
                    "Binary",
                    "Decimal",
                    "Hex",
                    "Exponential",
                    "Engineering",
                ]
                .map(String::from)
                .to_vec(),
            }),
        }),
    }
}

pub fn scalar_message() -> NtScalar {
    NtScalar {
        value: Some(Scalar {
            value: Some(scalar::Value::IntValue(0)),
        }),
        type_id: Some("epics:nt/NTScalar:1.0".into()),
        ..Default::default()
    }
}
pub fn array_message() -> NtScalarArray {
    NtScalarArray {
        value: Some(ScalarArray {
            value: Some(scalar_array::Value::DoubleValue(DoubleArray::default())),
        }),
        type_id: Some("epics:nt/NTScalarArray:1.0".into()),
        ..Default::default()
    }
}
pub fn enum_message() -> NtEnum {
    NtEnum {
        value: Some(enum_value()),
        type_id: Some("epics:nt/NTEnum:1.0".into()),
        ..Default::default()
    }
}
pub fn matrix() -> NtMatrix {
    NtMatrix {
        value: Some(DoubleArray {
            value: vec![1., 2., 3., 4.],
        }),
        dim: Some(IntArray { value: vec![2, 2] }),
        type_id: Some("epics:nt/NTMatrix:1.0".into()),
        ..Default::default()
    }
}
pub fn uri() -> Nturi {
    Nturi {
        scheme: Some("pva".into()),
        path: Some("channel".into()),
        type_id: Some("epics:nt/NTURI:1.0".into()),
        ..Default::default()
    }
}
pub fn name_value() -> NtNameValue {
    NtNameValue {
        name: Some(StringArray {
            value: vec!["a".into()],
        }),
        value: Some(ScalarArray {
            value: Some(scalar_array::Value::LongValue(LongArray {
                value: vec![i64::MAX],
            })),
        }),
        type_id: Some("epics:nt/NTNameValue:1.0".into()),
        ..Default::default()
    }
}
pub fn table() -> NtTable {
    NtTable {
        labels: Some(StringArray {
            value: vec!["Heading".into()],
        }),
        value: Some(TableValue {
            columns: vec![TableColumn {
                name: Some("field".into()),
                value: Some(ScalarArray {
                    value: Some(scalar_array::Value::StringValue(StringArray {
                        value: vec!["row".into()],
                    })),
                }),
            }],
        }),
        type_id: Some("epics:nt/NTTable:1.0".into()),
        ..Default::default()
    }
}
pub fn attribute() -> NtAttribute {
    NtAttribute {
        name: Some("key".into()),
        value: Some(union(true, Some(int_field()))),
        type_id: Some("epics:nt/NTAttribute:1.0".into()),
        ..Default::default()
    }
}
pub fn channels() -> NtMultiChannel {
    let u = union(true, Some(int_field()));
    let mut descriptor = u.r#type.unwrap();
    descriptor.kind = PvFieldKind::UnionArray as i32;
    descriptor.array_size = Some(PvArraySize::Variable as i32);
    NtMultiChannel {
        value: Some(PvUnionArray {
            r#type: Some(descriptor),
            value: Some(PvUnionArrayValue {
                elements: vec![PvNullableUnionValue { value: u.value }],
            }),
        }),
        channel_name: Some(StringArray {
            value: vec!["a".into()],
        }),
        type_id: Some("epics:nt/NTMultiChannel:1.0".into()),
        ..Default::default()
    }
}
pub fn image() -> NtndArray {
    NtndArray {
        value: Some(NdArrayValue {
            value: Some(nd_array_value::Value::UbyteValue(UByteArray {
                value: vec![0, 255, 1, 2],
            })),
        }),
        codec: Some(Codec {
            name: Some(String::new()),
            parameters: Some(union(true, None)),
        }),
        compressed_size: Some(4),
        uncompressed_size: Some(4),
        dimension: Some(DimensionArray {
            value: vec![
                Dimension {
                    size: Some(2),
                    offset: Some(0),
                    full_size: Some(2),
                    binning: Some(1),
                    reverse: Some(false)
                };
                2
            ],
        }),
        unique_id: Some(0),
        data_time_stamp: Some(time()),
        attribute: Some(AttributeArray::default()),
        type_id: Some("epics:nt/NTNDArray:1.0".into()),
        ..Default::default()
    }
}
pub fn continuum() -> NtContinuum {
    NtContinuum {
        base: Some(DoubleArray {
            value: vec![0., 1.],
        }),
        value: Some(DoubleArray {
            value: vec![10., 20., 11., 21.],
        }),
        units: Some(StringArray {
            value: vec!["s".into(), "A".into(), "V".into()],
        }),
        type_id: Some("epics:nt/NTContinuum:1.0".into()),
        ..Default::default()
    }
}
pub fn histogram() -> NtHistogram {
    NtHistogram {
        ranges: Some(DoubleArray {
            value: vec![0., 1., 2.],
        }),
        value: Some(HistogramValue {
            value: Some(histogram_value::Value::ShortValue(ShortArray {
                value: vec![2, 3],
            })),
        }),
        type_id: Some("epics:nt/NTHistogram:1.0".into()),
        ..Default::default()
    }
}
pub fn aggregate() -> NtAggregate {
    NtAggregate {
        value: Some(0.),
        n: Some(0),
        type_id: Some("epics:nt/NTAggregate:1.0".into()),
        ..Default::default()
    }
}

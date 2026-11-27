//! Reference validation for the supported canonical NT mapping. These checks do
//! not perform live pvData introspection or codec-dependent image decoding.

use crate::proto::*;
use std::{collections::HashSet, error::Error, fmt};

/// A contract violation with its protobuf field path and reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub path: String,
    pub reason: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.reason)
    }
}
impl Error for ValidationError {}

type Result<T = ()> = std::result::Result<T, ValidationError>;

/// Validate a generated NT message without mutating or normalizing it.
///
/// Bound untrusted input size before protobuf decoding. The default maximum
/// recursive validation depth is 64. No codec-specific content validation occurs.
pub trait Validate: sealed::ValidateAt {
    const NT_NAME: &'static str;

    fn validate(&self) -> Result {
        self.validate_with_limit(64)
    }

    fn validate_with_limit(&self, max_depth: usize) -> Result {
        if max_depth == 0 {
            return fail(Self::NT_NAME, "maximum depth must be positive");
        }
        self.validate_at(Self::NT_NAME, 0, max_depth)
    }
}

mod sealed {
    use super::Result;
    pub trait ValidateAt {
        fn validate_at(&self, path: &str, depth: usize, limit: usize) -> Result;
    }
}

fn fail<T>(path: impl Into<String>, reason: impl Into<String>) -> Result<T> {
    Err(ValidationError {
        path: path.into(),
        reason: reason.into(),
    })
}

fn required<'a, T>(value: &'a Option<T>, path: &str, field: &str) -> Result<&'a T> {
    value.as_ref().ok_or_else(|| ValidationError {
        path: format!("{path}.{field}"),
        reason: "required field is absent".into(),
    })
}

macro_rules! require {
    ($message:expr, $field:ident, $path:expr) => {
        required(
            &$message.$field,
            $path,
            stringify!($field).trim_start_matches("r#"),
        )?
    };
}

fn forbid(present: bool, path: &str, field: &str) -> Result {
    if present {
        fail(
            format!("{path}.{field}"),
            "not applicable to this field kind",
        )
    } else {
        Ok(())
    }
}

fn depth_check(path: &str, depth: usize, limit: usize) -> Result {
    if depth > limit {
        fail(path, format!("maximum nesting depth {limit} exceeded"))
    } else {
        Ok(())
    }
}

fn unique_name<'a>(
    name: &'a Option<String>,
    seen: &mut HashSet<&'a str>,
    path: &str,
) -> Result<&'a str> {
    let name = required(name, path, "name")?;
    if name.is_empty() {
        return fail(format!("{path}.name"), "field name is empty");
    }
    if !seen.insert(name) {
        return fail(
            format!("{path}.name"),
            format!("duplicate field name {name:?}"),
        );
    }
    Ok(name)
}

fn bounded<T: PartialOrd + fmt::Display>(value: T, low: T, high: T, path: &str) -> Result {
    if value < low || value > high {
        fail(path, format!("expected {low}..{high}, got {value}"))
    } else {
        Ok(())
    }
}

fn signed_array(values: &[i32], low: i32, high: i32, path: &str) -> Result {
    for (index, &value) in values.iter().enumerate() {
        bounded(value, low, high, &format!("{path}.value[{index}]"))?;
    }
    Ok(())
}
fn unsigned_array(values: &[u32], high: u32, path: &str) -> Result {
    for (index, &value) in values.iter().enumerate() {
        bounded(value, 0, high, &format!("{path}.value[{index}]"))?;
    }
    Ok(())
}

fn scalar(value: &Scalar, path: &str) -> Result<PvScalarType> {
    use scalar::Value as V;
    Ok(match require!(value, value, path) {
        V::BooleanValue(_) => PvScalarType::Boolean,
        V::ByteValue(v) => {
            bounded(*v, -128, 127, &format!("{path}.byte_value"))?;
            PvScalarType::Byte
        }
        V::UbyteValue(v) => {
            bounded(*v, 0, 255, &format!("{path}.ubyte_value"))?;
            PvScalarType::Ubyte
        }
        V::ShortValue(v) => {
            bounded(*v, -32768, 32767, &format!("{path}.short_value"))?;
            PvScalarType::Short
        }
        V::UshortValue(v) => {
            bounded(*v, 0, 65535, &format!("{path}.ushort_value"))?;
            PvScalarType::Ushort
        }
        V::IntValue(_) => PvScalarType::Int,
        V::UintValue(_) => PvScalarType::Uint,
        V::LongValue(_) => PvScalarType::Long,
        V::UlongValue(_) => PvScalarType::Ulong,
        V::FloatValue(_) => PvScalarType::Float,
        V::DoubleValue(_) => PvScalarType::Double,
        V::StringValue(_) => PvScalarType::String,
    })
}

fn scalar_array(value: &ScalarArray, path: &str) -> Result<(PvScalarType, usize)> {
    use scalar_array::Value as V;
    Ok(match require!(value, value, path) {
        V::BooleanValue(v) => (PvScalarType::Boolean, v.value.len()),
        V::ByteValue(v) => {
            signed_array(&v.value, -128, 127, &format!("{path}.byte_value"))?;
            (PvScalarType::Byte, v.value.len())
        }
        V::UbyteValue(v) => {
            unsigned_array(&v.value, 255, &format!("{path}.ubyte_value"))?;
            (PvScalarType::Ubyte, v.value.len())
        }
        V::ShortValue(v) => {
            signed_array(&v.value, -32768, 32767, &format!("{path}.short_value"))?;
            (PvScalarType::Short, v.value.len())
        }
        V::UshortValue(v) => {
            unsigned_array(&v.value, 65535, &format!("{path}.ushort_value"))?;
            (PvScalarType::Ushort, v.value.len())
        }
        V::IntValue(v) => (PvScalarType::Int, v.value.len()),
        V::UintValue(v) => (PvScalarType::Uint, v.value.len()),
        V::LongValue(v) => (PvScalarType::Long, v.value.len()),
        V::UlongValue(v) => (PvScalarType::Ulong, v.value.len()),
        V::FloatValue(v) => (PvScalarType::Float, v.value.len()),
        V::DoubleValue(v) => (PvScalarType::Double, v.value.len()),
        V::StringValue(v) => (PvScalarType::String, v.value.len()),
    })
}

fn enum_value(value: &EnumValue, path: &str) -> Result {
    let index = *require!(value, index, path);
    let choices = require!(value, choices, path);
    if index < 0 || index as usize >= choices.value.len() {
        return fail(format!("{path}.index"), "must select an entry in choices");
    }
    Ok(())
}
fn time(value: &Time, path: &str) -> Result {
    require!(value, seconds_past_epoch, path);
    require!(value, nanoseconds, path);
    require!(value, user_tag, path);
    Ok(())
}
fn alarm(value: &Alarm, path: &str) -> Result {
    bounded(
        *require!(value, severity, path),
        0,
        4,
        &format!("{path}.severity"),
    )?;
    bounded(
        *require!(value, status, path),
        0,
        7,
        &format!("{path}.status"),
    )?;
    require!(value, message, path);
    Ok(())
}
fn metadata(
    path: &str,
    alarm_value: Option<&Alarm>,
    time_value: Option<&Time>,
    display: Option<&Display>,
    control: Option<&Control>,
) -> Result {
    if let Some(v) = alarm_value {
        alarm(v, &format!("{path}.alarm"))?;
    }
    if let Some(v) = time_value {
        time(v, &format!("{path}.time_stamp"))?;
    }
    if let Some(v) = display {
        let p = format!("{path}.display");
        require!(v, limit_low, &p);
        require!(v, limit_high, &p);
        require!(v, description, &p);
        require!(v, units, &p);
        require!(v, precision, &p);
        let form = require!(v, form, &p);
        enum_value(form, &format!("{p}.form"))?;
        let expected = [
            "Default",
            "String",
            "Binary",
            "Decimal",
            "Hex",
            "Exponential",
            "Engineering",
        ];
        if form.choices.as_ref().unwrap().value != expected {
            return fail(
                format!("{p}.form.choices"),
                "must use the standard ordered display forms",
            );
        }
    }
    if let Some(v) = control {
        let p = format!("{path}.control");
        require!(v, limit_low, &p);
        require!(v, limit_high, &p);
        require!(v, min_step, &p);
    }
    Ok(())
}

fn field_type(value: &PvFieldType, path: &str, depth: usize, limit: usize) -> Result<PvFieldKind> {
    depth_check(path, depth, limit)?;
    let kind = PvFieldKind::try_from(value.kind)
        .ok()
        .filter(|v| *v != PvFieldKind::Unspecified)
        .ok_or_else(|| ValidationError {
            path: format!("{path}.kind"),
            reason: "unknown or unspecified pvData field kind".into(),
        })?;
    if matches!(kind, PvFieldKind::Scalar | PvFieldKind::ScalarArray) {
        let scalar = *require!(value, scalar_type, path);
        if PvScalarType::try_from(scalar)
            .ok()
            .filter(|v| *v != PvScalarType::Unspecified)
            .is_none()
        {
            return fail(
                format!("{path}.scalar_type"),
                "unknown or unspecified scalar type",
            );
        }
        forbid(value.type_id.is_some(), path, "type_id")?;
        forbid(value.variant.is_some(), path, "variant")?;
        if !value.fields.is_empty() {
            return fail(
                format!("{path}.fields"),
                "scalar fields cannot declare members",
            );
        }
    } else {
        require!(value, type_id, path);
        forbid(value.scalar_type.is_some(), path, "scalar_type")?;
        if matches!(kind, PvFieldKind::Union | PvFieldKind::UnionArray) {
            let variant = *require!(value, variant, path);
            if variant && !value.fields.is_empty() {
                return fail(
                    format!("{path}.fields"),
                    "variant unions do not declare alternatives",
                );
            }
            if !variant && value.fields.is_empty() {
                return fail(
                    format!("{path}.fields"),
                    "regular unions require at least one alternative",
                );
            }
        } else {
            forbid(value.variant.is_some(), path, "variant")?;
        }
        let mut seen = HashSet::new();
        for (index, declaration) in value.fields.iter().enumerate() {
            let p = format!("{path}.fields[{index}]");
            unique_name(&declaration.name, &mut seen, &p)?;
            field_type(
                require!(declaration, r#type, &p),
                &format!("{p}.type"),
                depth + 1,
                limit,
            )?;
        }
    }
    if matches!(
        kind,
        PvFieldKind::ScalarArray | PvFieldKind::StructureArray | PvFieldKind::UnionArray
    ) {
        let policy = *require!(value, array_size, path);
        match PvArraySize::try_from(policy) {
            Ok(PvArraySize::Variable) => {
                forbid(value.array_capacity.is_some(), path, "array_capacity")?
            }
            Ok(PvArraySize::Fixed | PvArraySize::Bounded) if kind == PvFieldKind::ScalarArray => {
                require!(value, array_capacity, path);
            }
            Ok(PvArraySize::Fixed | PvArraySize::Bounded) => {
                return fail(
                    format!("{path}.array_size"),
                    "structure and union arrays are variable-sized",
                );
            }
            _ => {
                return fail(
                    format!("{path}.array_size"),
                    "unknown or unspecified array size policy",
                );
            }
        }
    } else {
        forbid(value.array_size.is_some(), path, "array_size")?;
        forbid(value.array_capacity.is_some(), path, "array_capacity")?;
    }
    Ok(kind)
}

fn structure_value(
    value: &PvStructureValue,
    descriptor: &PvFieldType,
    path: &str,
    depth: usize,
    limit: usize,
) -> Result {
    depth_check(path, depth, limit)?;
    if value.fields.len() != descriptor.fields.len() {
        return fail(
            path,
            "structure values must match the declaration count and order",
        );
    }
    for (index, (field, declaration)) in value.fields.iter().zip(&descriptor.fields).enumerate() {
        let p = format!("{path}.fields[{index}]");
        pv_field(field, &p, depth + 1, limit)?;
        if field.r#type != declaration.r#type {
            return fail(
                format!("{p}.type"),
                "does not match its ordered declaration",
            );
        }
    }
    Ok(())
}

fn union_value(
    value: &PvUnionValue,
    descriptor: &PvFieldType,
    path: &str,
    depth: usize,
    limit: usize,
) -> Result {
    depth_check(path, depth, limit)?;
    if descriptor.variant == Some(true) {
        if value.selected_name.is_some() {
            return fail(
                format!("{path}.selected_name"),
                "variant unions do not select a named alternative",
            );
        }
        if let Some(field) = &value.selected_value {
            pv_field(field, &format!("{path}.selected_value"), depth + 1, limit)?;
        }
    } else {
        match (&value.selected_name, &value.selected_value) {
            (None, None) => {}
            (Some(name), Some(field)) => {
                let declaration = descriptor
                    .fields
                    .iter()
                    .find(|v| v.name.as_ref() == Some(name))
                    .ok_or_else(|| ValidationError {
                        path: format!("{path}.selected_name"),
                        reason: "not a declared regular-union alternative".into(),
                    })?;
                pv_field(field, &format!("{path}.selected_value"), depth + 1, limit)?;
                if field.r#type != declaration.r#type {
                    return fail(
                        format!("{path}.selected_value.type"),
                        "does not match the selected declaration",
                    );
                }
            }
            _ => {
                return fail(
                    format!("{path}.selected_value"),
                    "regular union selection requires both name and value",
                );
            }
        }
    }
    Ok(())
}
fn union_array(
    value: &PvUnionArrayValue,
    descriptor: &PvFieldType,
    path: &str,
    depth: usize,
    limit: usize,
) -> Result {
    for (index, element) in value.elements.iter().enumerate() {
        if let Some(v) = &element.value {
            union_value(
                v,
                descriptor,
                &format!("{path}.elements[{index}].value"),
                depth + 1,
                limit,
            )?;
        }
    }
    Ok(())
}

fn pv_field(value: &PvField, path: &str, depth: usize, limit: usize) -> Result {
    depth_check(path, depth, limit)?;
    let descriptor = require!(value, r#type, path);
    let kind = field_type(descriptor, &format!("{path}.type"), depth, limit)?;
    use pv_field::Value as V;
    match (kind, require!(value, value, path)) {
        (PvFieldKind::Scalar, V::Scalar(v)) => {
            if Some(scalar(v, &format!("{path}.scalar"))? as i32) != descriptor.scalar_type {
                return fail(
                    format!("{path}.type.scalar_type"),
                    "does not match the selected scalar type",
                );
            }
        }
        (PvFieldKind::ScalarArray, V::ScalarArray(v)) => {
            let (scalar, len) = scalar_array(v, &format!("{path}.scalar_array"))?;
            if Some(scalar as i32) != descriptor.scalar_type {
                return fail(
                    format!("{path}.type.scalar_type"),
                    "does not match the selected array type",
                );
            }
            if let Some(capacity) = descriptor.array_capacity {
                let len = len as u64;
                if descriptor.array_size == Some(PvArraySize::Fixed as i32) && len != capacity {
                    return fail(
                        format!("{path}.type.array_capacity"),
                        "fixed array length must equal its capacity",
                    );
                }
                if descriptor.array_size == Some(PvArraySize::Bounded as i32) && len > capacity {
                    return fail(
                        format!("{path}.type.array_capacity"),
                        "bounded array length exceeds its capacity",
                    );
                }
            }
        }
        (PvFieldKind::Structure, V::Structure(v)) => {
            structure_value(v, descriptor, &format!("{path}.structure"), depth, limit)?
        }
        (PvFieldKind::StructureArray, V::StructureArray(v)) => {
            for (index, element) in v.elements.iter().enumerate() {
                if let Some(v) = &element.value {
                    structure_value(
                        v,
                        descriptor,
                        &format!("{path}.structure_array.elements[{index}].value"),
                        depth + 1,
                        limit,
                    )?;
                }
            }
        }
        (PvFieldKind::Union, V::Union(v)) => {
            union_value(v, descriptor, &format!("{path}.union"), depth, limit)?
        }
        (PvFieldKind::UnionArray, V::UnionArray(v)) => {
            union_array(v, descriptor, &format!("{path}.union_array"), depth, limit)?
        }
        _ => return fail(path, "value case does not match its declared field kind"),
    }
    Ok(())
}

fn pv_union(value: &PvUnion, path: &str, depth: usize, limit: usize, variant: bool) -> Result {
    let descriptor = require!(value, r#type, path);
    if field_type(descriptor, &format!("{path}.type"), depth, limit)? != PvFieldKind::Union {
        return fail(format!("{path}.type.kind"), "expected a union");
    }
    if variant && descriptor.variant != Some(true) {
        return fail(
            format!("{path}.type.variant"),
            "this NT field requires a variant union",
        );
    }
    union_value(
        require!(value, value, path),
        descriptor,
        &format!("{path}.value"),
        depth,
        limit,
    )
}
fn pv_union_array(value: &PvUnionArray, path: &str, depth: usize, limit: usize) -> Result {
    let descriptor = require!(value, r#type, path);
    if field_type(descriptor, &format!("{path}.type"), depth, limit)? != PvFieldKind::UnionArray {
        return fail(format!("{path}.type.kind"), "expected a union array");
    }
    union_array(
        require!(value, value, path),
        descriptor,
        &format!("{path}.value"),
        depth,
        limit,
    )
}

fn header(
    id: &Option<String>,
    name: &str,
    extras: &[PvNamedField],
    canonical: &[&str],
    path: &str,
    depth: usize,
    limit: usize,
) -> Result {
    depth_check(path, depth, limit)?;
    if required(id, path, "type_id")? != &format!("epics:nt/{name}:1.0") {
        return fail(
            format!("{path}.type_id"),
            format!("expected case-sensitive epics:nt/{name}:1.0"),
        );
    }
    let mut seen = HashSet::new();
    for (index, extra) in extras.iter().enumerate() {
        let p = format!("{path}.extra_fields[{index}]");
        let name = unique_name(&extra.name, &mut seen, &p)?;
        if canonical.contains(&name) {
            return fail(
                format!("{p}.name"),
                "collides with a canonical pvData field name",
            );
        }
        pv_field(
            require!(extra, value, &p),
            &format!("{p}.value"),
            depth + 1,
            limit,
        )?;
    }
    Ok(())
}

// Declarations list pvData names so extension collision checks are independent
// of the Rust/Java naming convention. The macro only supplies shared entry checks.
macro_rules! nt {
    ($ty:ty, $name:literal, $canonical:expr, $body:ident) => {
        impl Validate for $ty {
            const NT_NAME: &'static str = $name;
        }
        impl sealed::ValidateAt for $ty {
            fn validate_at(&self, path: &str, depth: usize, limit: usize) -> Result {
                header(
                    &self.type_id,
                    $name,
                    &self.extra_fields,
                    $canonical,
                    path,
                    depth,
                    limit,
                )?;
                $body(self, path, depth, limit)
            }
        }
    };
}

nt!(
    NtScalar,
    "NTScalar",
    &[
        "value",
        "descriptor",
        "alarm",
        "timeStamp",
        "display",
        "control"
    ],
    nt_scalar
);
fn nt_scalar(v: &NtScalar, p: &str, _: usize, _: usize) -> Result {
    scalar(require!(v, value, p), &format!("{p}.value"))?;
    metadata(
        p,
        v.alarm.as_ref(),
        v.time_stamp.as_ref(),
        v.display.as_ref(),
        v.control.as_ref(),
    )
}
nt!(
    NtScalarArray,
    "NTScalarArray",
    &[
        "value",
        "descriptor",
        "alarm",
        "timeStamp",
        "display",
        "control"
    ],
    nt_scalar_array
);
fn nt_scalar_array(v: &NtScalarArray, p: &str, _: usize, _: usize) -> Result {
    scalar_array(require!(v, value, p), &format!("{p}.value"))?;
    metadata(
        p,
        v.alarm.as_ref(),
        v.time_stamp.as_ref(),
        v.display.as_ref(),
        v.control.as_ref(),
    )
}
nt!(
    NtEnum,
    "NTEnum",
    &["value", "descriptor", "alarm", "timeStamp"],
    nt_enum
);
fn nt_enum(v: &NtEnum, p: &str, _: usize, _: usize) -> Result {
    enum_value(require!(v, value, p), &format!("{p}.value"))?;
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtMatrix,
    "NTMatrix",
    &[
        "value",
        "dim",
        "descriptor",
        "alarm",
        "timeStamp",
        "display"
    ],
    nt_matrix
);
fn nt_matrix(v: &NtMatrix, p: &str, _: usize, _: usize) -> Result {
    let values = require!(v, value, p);
    if let Some(dim) = &v.dim {
        if !(1..=2).contains(&dim.value.len()) || dim.value.iter().any(|v| *v <= 0) {
            return fail(
                format!("{p}.dim"),
                "must contain one or two strictly positive dimensions",
            );
        }
        let count: u64 = dim.value.iter().map(|v| *v as u64).product();
        if count != values.value.len() as u64 {
            return fail(
                format!("{p}.dim"),
                "dimension product must equal value length",
            );
        }
    }
    metadata(
        p,
        v.alarm.as_ref(),
        v.time_stamp.as_ref(),
        v.display.as_ref(),
        None,
    )
}
nt!(
    Nturi,
    "NTURI",
    &["scheme", "authority", "path", "query"],
    nt_uri
);
fn nt_uri(v: &Nturi, p: &str, _: usize, _: usize) -> Result {
    let scheme = require!(v, scheme, p);
    require!(v, path, p);
    if !scheme
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_alphabetic)
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+.-".contains(&b))
    {
        return fail(format!("{p}.scheme"), "must be a nonempty RFC 3986 scheme");
    }
    if let Some(query) = &v.query {
        let mut seen = HashSet::new();
        for (index, arg) in query.arguments.iter().enumerate() {
            let p = format!("{p}.query.arguments[{index}]");
            unique_name(&arg.name, &mut seen, &p)?;
            require!(arg, value, &p);
        }
    }
    Ok(())
}
nt!(
    NtNameValue,
    "NTNameValue",
    &["name", "value", "descriptor", "alarm", "timeStamp"],
    nt_name_value
);
fn nt_name_value(v: &NtNameValue, p: &str, _: usize, _: usize) -> Result {
    let names = require!(v, name, p);
    let (_, len) = scalar_array(require!(v, value, p), &format!("{p}.value"))?;
    if names.value.len() != len {
        return fail(
            format!("{p}.name"),
            "name and value arrays must have equal lengths",
        );
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtTable,
    "NTTable",
    &["labels", "value", "descriptor", "alarm", "timeStamp"],
    nt_table
);
fn nt_table(v: &NtTable, p: &str, _: usize, _: usize) -> Result {
    let labels = require!(v, labels, p);
    let table = require!(v, value, p);
    if labels.value.len() != table.columns.len() {
        return fail(format!("{p}.labels"), "label count must equal column count");
    }
    let mut seen = HashSet::new();
    let mut rows = None;
    for (index, column) in table.columns.iter().enumerate() {
        let p = format!("{p}.value.columns[{index}]");
        unique_name(&column.name, &mut seen, &p)?;
        let (_, len) = scalar_array(require!(column, value, &p), &format!("{p}.value"))?;
        if rows.is_some_and(|r| r != len) {
            return fail(p, "all columns must have the same row count");
        }
        rows = Some(len);
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtAttribute,
    "NTAttribute",
    &[
        "name",
        "value",
        "tags",
        "descriptor",
        "alarm",
        "timeStamp",
        "sourceType",
        "source"
    ],
    nt_attribute
);
fn nt_attribute(v: &NtAttribute, p: &str, depth: usize, limit: usize) -> Result {
    require!(v, name, p);
    pv_union(
        require!(v, value, p),
        &format!("{p}.value"),
        depth,
        limit,
        true,
    )?;
    if let Some(source) = v.source_type {
        bounded(source, 0, 3, &format!("{p}.source_type"))?;
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtMultiChannel,
    "NTMultiChannel",
    &[
        "value",
        "channelName",
        "descriptor",
        "alarm",
        "timeStamp",
        "severity",
        "status",
        "message",
        "secondsPastEpoch",
        "nanoseconds",
        "userTag"
    ],
    nt_multi_channel
);
fn nt_multi_channel(v: &NtMultiChannel, p: &str, depth: usize, limit: usize) -> Result {
    let values = require!(v, value, p);
    pv_union_array(values, &format!("{p}.value"), depth, limit)?;
    let len = values.value.as_ref().unwrap().elements.len();
    if require!(v, channel_name, p).value.len() != len {
        return fail(
            format!("{p}.channel_name"),
            "channel names must match the value count",
        );
    }
    for (name, count) in [
        ("severity", v.severity.as_ref().map(|v| v.value.len())),
        ("status", v.status.as_ref().map(|v| v.value.len())),
        ("message", v.message.as_ref().map(|v| v.value.len())),
        (
            "seconds_past_epoch",
            v.seconds_past_epoch.as_ref().map(|v| v.value.len()),
        ),
        ("nanoseconds", v.nanoseconds.as_ref().map(|v| v.value.len())),
        ("user_tag", v.user_tag.as_ref().map(|v| v.value.len())),
    ] {
        if count.is_some_and(|count| count != len) {
            return fail(
                format!("{p}.{name}"),
                "per-channel metadata must match the value count",
            );
        }
    }
    if let Some(v) = &v.severity {
        signed_array(&v.value, 0, 4, &format!("{p}.severity"))?;
    }
    if let Some(v) = &v.status {
        signed_array(&v.value, 0, 7, &format!("{p}.status"))?;
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}

fn image_array(value: &NdArrayValue, path: &str) -> Result<(usize, u64)> {
    use nd_array_value::Value as V;
    Ok(match require!(value, value, path) {
        V::BooleanValue(v) => (v.value.len(), 1),
        V::ByteValue(v) => {
            signed_array(&v.value, -128, 127, &format!("{path}.byte_value"))?;
            (v.value.len(), 1)
        }
        V::UbyteValue(v) => {
            unsigned_array(&v.value, 255, &format!("{path}.ubyte_value"))?;
            (v.value.len(), 1)
        }
        V::ShortValue(v) => {
            signed_array(&v.value, -32768, 32767, &format!("{path}.short_value"))?;
            (v.value.len(), 2)
        }
        V::UshortValue(v) => {
            unsigned_array(&v.value, 65535, &format!("{path}.ushort_value"))?;
            (v.value.len(), 2)
        }
        V::IntValue(v) => (v.value.len(), 4),
        V::UintValue(v) => (v.value.len(), 4),
        V::LongValue(v) => (v.value.len(), 8),
        V::UlongValue(v) => (v.value.len(), 8),
        V::FloatValue(v) => (v.value.len(), 4),
        V::DoubleValue(v) => (v.value.len(), 8),
    })
}
nt!(
    NtndArray,
    "NTNDArray",
    &[
        "value",
        "codec",
        "compressedSize",
        "uncompressedSize",
        "dimension",
        "uniqueId",
        "dataTimeStamp",
        "attribute",
        "descriptor",
        "alarm",
        "timeStamp",
        "display"
    ],
    nt_nd_array
);
fn nt_nd_array(v: &NtndArray, p: &str, depth: usize, limit: usize) -> Result {
    let (len, width) = image_array(require!(v, value, p), &format!("{p}.value"))?;
    let codec = require!(v, codec, p);
    let name = require!(codec, name, &format!("{p}.codec"));
    pv_union(
        require!(codec, parameters, &format!("{p}.codec")),
        &format!("{p}.codec.parameters"),
        depth,
        limit,
        true,
    )?;
    let compressed = *require!(v, compressed_size, p);
    if compressed < 0 || (len as u64).checked_mul(width) != Some(compressed as u64) {
        return fail(
            format!("{p}.compressed_size"),
            "must equal stored element count times scalar byte width",
        );
    }
    let uncompressed = *require!(v, uncompressed_size, p);
    if uncompressed < 0 {
        return fail(format!("{p}.uncompressed_size"), "must be nonnegative");
    }
    let dimensions = require!(v, dimension, p);
    let mut sizes = Vec::new();
    for (index, dimension) in dimensions.value.iter().enumerate() {
        let p = format!("{p}.dimension.value[{index}]");
        let size = *require!(dimension, size, &p);
        require!(dimension, offset, &p);
        require!(dimension, full_size, &p);
        require!(dimension, binning, &p);
        require!(dimension, reverse, &p);
        if size < 0 {
            return fail(format!("{p}.size"), "must be nonnegative");
        }
        sizes.push(size as u64);
    }
    if dimensions.value.is_empty() && uncompressed != 0 {
        return fail(
            format!("{p}.uncompressed_size"),
            "must be zero when there are no dimensions",
        );
    }
    if name.is_empty() {
        let count = if sizes.is_empty() || sizes.contains(&0) {
            0
        } else {
            sizes
                .into_iter()
                .try_fold(1_u64, |n, size| n.checked_mul(size))
                .ok_or_else(|| ValidationError {
                    path: format!("{p}.dimension"),
                    reason: "dimension product overflows supported size".into(),
                })?
        };
        if (len as u64) < count {
            return fail(
                format!("{p}.value"),
                "raw element count must cover the dimension product",
            );
        }
        if count.checked_mul(width) != Some(uncompressed as u64) {
            return fail(
                format!("{p}.uncompressed_size"),
                "must equal dimension product times scalar byte width",
            );
        }
    }
    require!(v, unique_id, p);
    time(
        require!(v, data_time_stamp, p),
        &format!("{p}.data_time_stamp"),
    )?;
    let attributes = require!(v, attribute, p);
    for (index, attribute) in attributes.value.iter().enumerate() {
        let p = format!("{p}.attribute.value[{index}]");
        require!(attribute, descriptor_text, &p);
        require!(attribute, source_type, &p);
        require!(attribute, source, &p);
        sealed::ValidateAt::validate_at(attribute, &p, depth + 1, limit)?;
    }
    metadata(
        p,
        v.alarm.as_ref(),
        v.time_stamp.as_ref(),
        v.display.as_ref(),
        None,
    )
}
nt!(
    NtContinuum,
    "NTContinuum",
    &["base", "value", "units", "descriptor", "alarm", "timeStamp"],
    nt_continuum
);
fn nt_continuum(v: &NtContinuum, p: &str, _: usize, _: usize) -> Result {
    let base = require!(v, base, p).value.len();
    let values = require!(v, value, p).value.len();
    let units = require!(v, units, p).value.len();
    if base == 0 {
        if values != 0 {
            return fail(
                format!("{p}.base"),
                "an empty base requires an empty value array",
            );
        }
        if units < 2 {
            return fail(
                format!("{p}.units"),
                "empty data requires base units and at least one component's units",
            );
        }
    } else {
        if values % base != 0 || values / base < 1 {
            return fail(
                format!("{p}.value"),
                "must contain one or more complete components for every base point",
            );
        }
        if (values / base).checked_add(1) != Some(units) {
            return fail(
                format!("{p}.units"),
                "must contain base units plus one unit per point component",
            );
        }
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtHistogram,
    "NTHistogram",
    &["ranges", "value", "descriptor", "alarm", "timeStamp"],
    nt_histogram
);
fn nt_histogram(v: &NtHistogram, p: &str, _: usize, _: usize) -> Result {
    let ranges = require!(v, ranges, p).value.len();
    let values = require!(v, value, p);
    let len = match require!(values, value, &format!("{p}.value")) {
        histogram_value::Value::ShortValue(v) => {
            signed_array(&v.value, -32768, 32767, &format!("{p}.value.short_value"))?;
            v.value.len()
        }
        histogram_value::Value::IntValue(v) => v.value.len(),
        histogram_value::Value::LongValue(v) => v.value.len(),
    };
    if len.checked_add(1) != Some(ranges) {
        return fail(
            format!("{p}.ranges"),
            "range boundary count must equal bin count plus one",
        );
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}
nt!(
    NtAggregate,
    "NTAggregate",
    &[
        "value",
        "N",
        "dispersion",
        "first",
        "firstTimeStamp",
        "last",
        "lastTimeStamp",
        "max",
        "min",
        "descriptor",
        "alarm",
        "timeStamp"
    ],
    nt_aggregate
);
fn nt_aggregate(v: &NtAggregate, p: &str, _: usize, _: usize) -> Result {
    require!(v, value, p);
    if *require!(v, n, p) < 0 {
        return fail(format!("{p}.n"), "observation count must be nonnegative");
    }
    if let Some(v) = &v.first_time_stamp {
        time(v, &format!("{p}.first_time_stamp"))?;
    }
    if let Some(v) = &v.last_time_stamp {
        time(v, &format!("{p}.last_time_stamp"))?;
    }
    metadata(p, v.alarm.as_ref(), v.time_stamp.as_ref(), None, None)
}

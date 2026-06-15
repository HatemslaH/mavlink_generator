use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::dart::writer::DartWriter;
use crate::generate::rust::util::{
    dialect_name_from_path, dialect_struct_name, enum_type_name, message_struct_name,
    parse_local_name, rust_field_access, rust_field_name, unique_enum_entry_rust_name,
};
use crate::xml::{
    BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage, camel_case,
};

fn write_rust_doc(writer: &mut DartWriter, text: &str) {
    for line in text.lines() {
        let trimmed = line.trim_start().trim_end();
        if !trimmed.is_empty() {
            writer.line(&format!("/// {trimmed}"));
        }
    }
}

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("use crate::mavlink_dialect::MavlinkDialect;");
    writer.line("use crate::mavlink_message::MavlinkMessage;");
    writer.line("use crate::mavlink_types::*;");

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            write_rust_doc(&mut writer, description);
        }
        writer.line(&format!("/// {}", enm.name));
        let type_name = enum_type_name(&enm.name);
        writer.line("#[repr(i32)]");
        writer.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
        writer.block(&format!("pub enum {type_name} {{"), "}", |w| {
            render_enum_entries(w, enm.entries.as_slice());
        });
        writer.blank();
        render_enum_impl(
            &mut writer,
            &type_name,
            enm.entries.as_slice(),
            enm.is_bitmask,
        );
    }

    for msg in doc.messages.messages() {
        writer.blank();
        render_message(&mut writer, msg)?;
    }

    writer.blank();
    render_dialect_impl(&mut writer, doc, &dialect_name_from_path(src_dialect_path))?;

    Ok(writer.into_string())
}

fn render_enum_entries(writer: &mut DartWriter, entries: &[DialectEntry]) {
    let mut used_enum_entry_names = HashSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_name = unique_enum_entry_rust_name(entry, &mut used_enum_entry_names);
        let separator = if index + 1 == entries.len() { "" } else { "," };

        if entry.wip {
            writer.line("/// WIP.");
        }
        if let Some(description) = &entry.description {
            write_rust_doc(writer, description);
        }
        writer.line(&format!("/// {}", entry.name));
        writer.line(&format!("{entry_name} = {}{separator}", entry.value));
    }
}

fn render_enum_impl(
    writer: &mut DartWriter,
    enum_name: &str,
    entries: &[DialectEntry],
    is_bitmask: bool,
) {
    writer.block(&format!("impl {enum_name} {{"), "}", |w| {
        w.line("pub fn from_value(value: i32) -> Self {");
        w.indent();
        if is_bitmask {
            w.line("let variants = [");
            w.indent();
            for entry in entries {
                w.line(&format!("Self::{},", entry.name));
            }
            w.dedent();
            w.line("];");
            w.block("for variant in variants {", "}", |w| {
                w.line("if (variant as i32) == value {");
                w.indent();
                w.line("return variant;");
                w.dedent();
                w.line("}");
            });
            w.line(
                "let mut sorted: Vec<Self> = variants.into_iter().filter(|v| *v as i32 > 0).collect();",
            );
            w.line("sorted.sort_by(|a, b| (*b as i32).cmp(&(*a as i32)));");
            w.block("for variant in sorted {", "}", |w| {
                w.line("if (value & (variant as i32)) != 0 {");
                w.indent();
                w.line("return variant;");
                w.dedent();
                w.line("}");
            });
            if let Some(first_zero) = entries.iter().find(|e| e.value == 0) {
                w.line(&format!("return Self::{};", first_zero.name));
            } else if let Some(first) = entries.first() {
                w.line(&format!("return Self::{};", first.name));
            } else {
                w.line("return variants[0];");
            }
        } else {
            w.line("match value {");
            w.indent();
            for entry in entries {
                w.line(&format!("{} => Self::{},", entry.value, entry.name));
            }
            if let Some(first) = entries.first() {
                w.line(&format!("_ => Self::{},", first.name));
            }
            w.dedent();
            w.line("}");
        }
        w.dedent();
        w.line("}");
    });
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let struct_name = message_struct_name(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    write_rust_doc(writer, &msg.description);
    writer.line(&format!("/// {}", msg.name));
    writer.line("#[derive(Debug, Clone, PartialEq)]");
    writer.block(&format!("pub struct {struct_name} {{"), "}", |w| {
        for field in &msg.ordered_fields {
            w.blank();
            render_field(w, field);
        }
    });

    writer.blank();
    writer.line(&format!("impl {struct_name} {{"));
    writer.indent();
    writer.line(&format!("pub const MSG_ID: u32 = {};", msg.id));
    writer.line(&format!("pub const CRC_EXTRA: u8 = {crc_extra};"));
    writer.line(&format!(
        "pub const MAVLINK_ENCODED_LENGTH: usize = {encoded_length};"
    ));
    writer.blank();
    render_parse(writer, msg, &struct_name)?;
    writer.dedent();
    writer.line("}");
    writer.blank();
    writer.line(&format!("impl MavlinkMessage for {struct_name} {{"));
    writer.indent();
    writer.line("fn mavlink_message_id(&self) -> u32 {");
    writer.indent();
    writer.line("Self::MSG_ID");
    writer.dedent();
    writer.line("}");
    writer.blank();
    writer.line("fn mavlink_crc_extra(&self) -> u8 {");
    writer.indent();
    writer.line("Self::CRC_EXTRA");
    writer.dedent();
    writer.line("}");
    writer.blank();
    render_serialize(writer, msg)?;
    writer.dedent();
    writer.line("}");

    Ok(())
}

fn render_field(writer: &mut DartWriter, field: &DialectField) {
    write_rust_doc(writer, &field.description);
    writer.line(&format!("/// MAVLink type: {}", field.field_type));
    if let Some(units) = &field.units {
        writer.line(&format!("/// units: {units}"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line(&format!("/// enum: [{}]", camel_case(enum_name)));
    }
    if field.is_extension {
        writer.line("/// Extensions field for MAVLink 2.");
    }
    writer.line(&format!("/// {}", field.name));
    writer.line(&format!(
        "pub {}: {},",
        rust_field_name(&field.name),
        as_rust_type(
            &field.field_type,
            field.enum_name.as_deref(),
            field.is_bitmask
        )
    ));
}

fn render_parse(writer: &mut DartWriter, msg: &DialectMessage, struct_name: &str) -> Result<()> {
    writer.line("pub fn parse(data: &[u8]) -> Self {");
    writer.indent();
    writer.line(
        "let padded = crate::mavlink_message::pad_payload(data, Self::MAVLINK_ENCODED_LENGTH);",
    );

    let mut byte_offset = 0usize;
    for field in &msg.ordered_fields {
        byte_offset += render_parse_field(writer, field, byte_offset)?;
    }

    writer.blank();
    writer.line(&format!("{struct_name} {{"));
    writer.indent();
    for (index, field) in msg.ordered_fields.iter().enumerate() {
        let comma = if index + 1 == msg.ordered_fields.len() {
            ""
        } else {
            ","
        };
        writer.line(&format!(
            "{}: {}{comma}",
            rust_field_name(&field.name),
            parse_local_name(&field.name)
        ));
    }
    writer.dedent();
    writer.line("}");
    writer.dedent();
    writer.line("}");
    Ok(())
}

fn is_char_field(field_type: &str) -> bool {
    field_type == "char" || field_type.starts_with("char[")
}

fn render_parse_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: usize,
) -> Result<usize> {
    let parsed = field.parsed_type()?;
    let local_name = parse_local_name(&field.name);
    let _field_name = &field.name;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let enum_type = field
        .enum_name
        .as_deref()
        .map(enum_type_name)
        .unwrap_or_default();

    if parsed.is_array() {
        if is_char_field(&field.field_type) {
            let get_fn = format!(
                "crate::mavlink_message::get_u8_array::<{}>",
                parsed.array_length
            );
            writer.line(&format!(
                "let {local_name} = {get_fn}(&padded, {byte_offset});"
            ));
        } else if is_enum {
            let get_fn = parse_array_get_fn(parsed.basic_type, parsed.bit, parsed.array_length);
            writer.line(&format!(
                "let {local_name}_raw = {get_fn}(&padded, {byte_offset});"
            ));
            writer.line(&format!(
                "let {local_name} = {local_name}_raw.map(|v| {enum_type}::from_value(v as i32));"
            ));
        } else {
            let get_fn = parse_array_get_fn(parsed.basic_type, parsed.bit, parsed.array_length);
            writer.line(&format!(
                "let {local_name} = {get_fn}(&padded, {byte_offset});"
            ));
        }
    } else if is_enum {
        let get_fn = parse_scalar_get_fn(parsed.basic_type, parsed.bit);
        writer.line(&format!(
            "let {local_name}_raw = {get_fn}(&padded, {byte_offset});"
        ));
        writer.line(&format!(
            "let {local_name} = {enum_type}::from_value({local_name}_raw as i32);"
        ));
    } else {
        let get_fn = if is_char_field(&field.field_type) {
            "crate::mavlink_message::get_u8"
        } else {
            parse_scalar_get_fn(parsed.basic_type, parsed.bit)
        };
        writer.line(&format!(
            "let {local_name} = {get_fn}(&padded, {byte_offset});"
        ));
    }

    Ok((parsed.byte() * parsed.array_length) as usize)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    writer.line("fn serialize(&self) -> Vec<u8> {");
    writer.indent();
    writer.line("let mut data = vec![0u8; Self::MAVLINK_ENCODED_LENGTH];");

    let mut byte_offset = 0usize;
    for field in &msg.ordered_fields {
        byte_offset += render_serialize_field(writer, field, byte_offset)?;
    }

    writer.line("data");
    writer.dedent();
    writer.line("}");
    Ok(())
}

fn render_serialize_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: usize,
) -> Result<usize> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let field_access = rust_field_access(&field.name);

    if parsed.is_array() {
        if is_char_field(&field.field_type) {
            let set_fn = format!(
                "crate::mavlink_message::set_u8_array::<{}>",
                parsed.array_length
            );
            writer.line(&format!(
                "{set_fn}(&mut data, {byte_offset}, &{field_access});"
            ));
        } else if is_enum {
            writer.line(&format!(
                "let {}_serialized: [i32; {}] = std::array::from_fn(|i| {}[i] as i32);",
                field.name, parsed.array_length, field_access
            ));
            let set_fn = serialize_array_set_fn(parsed.basic_type, parsed.bit, parsed.array_length);
            writer.line(&format!(
                "{}(&mut data, {byte_offset}, &{}_serialized);",
                set_fn, field.name
            ));
        } else {
            let set_fn = serialize_array_set_fn(parsed.basic_type, parsed.bit, parsed.array_length);
            writer.line(&format!(
                "{set_fn}(&mut data, {byte_offset}, &{field_access});"
            ));
        }
    } else {
        let value = if is_enum {
            serialize_enum_value(&field_access, parsed.basic_type, parsed.bit)
        } else {
            field_access
        };
        let set_fn = serialize_scalar_set_fn(parsed.basic_type, parsed.bit);
        writer.line(&format!("{set_fn}(&mut data, {byte_offset}, {value});"));
    }

    Ok((parsed.byte() * parsed.array_length) as usize)
}

fn render_dialect_impl(
    writer: &mut DartWriter,
    doc: &DialectDocument,
    dialect_name: &str,
) -> Result<()> {
    let dialect_struct = dialect_struct_name(dialect_name);

    writer.line("#[derive(Debug, Clone, Copy, Default)]");
    writer.line(&format!("pub struct {dialect_struct};"));
    writer.blank();
    writer.try_block(
        &format!("impl MavlinkDialect for {dialect_struct} {{"),
        "}",
        |w| {
            w.line(&format!("fn version(&self) -> u8 {{ {} }}", doc.version));
            w.blank();
            w.line(
            "fn parse(&self, message_id: u32, data: &[u8]) -> Option<Box<dyn MavlinkMessage>> {",
        );
            w.indent();
            w.line("match message_id {");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_struct_name(&msg.name);
                w.line(&format!(
                    "{}::MSG_ID => Some(Box::new({class_name}::parse(data))),",
                    class_name
                ));
            }
            w.line("_ => None,");
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.blank();
            w.line("fn crc_extra(&self, message_id: u32) -> i32 {");
            w.indent();
            w.line("match message_id {");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_struct_name(&msg.name);
                w.line(&format!(
                    "{}::MSG_ID => {}::CRC_EXTRA as i32,",
                    class_name, class_name
                ));
            }
            w.line("_ => -1,");
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            Ok(())
        },
    )
}

fn serialize_enum_value(field_access: &str, basic_type: BasicType, bit: u32) -> String {
    match (basic_type, bit) {
        (BasicType::Int, 8) => format!("{field_access} as i32 as i8"),
        (BasicType::Uint, 8) => format!("{field_access} as i32 as u8"),
        (BasicType::Int, 16) => format!("{field_access} as i32 as i16"),
        (BasicType::Uint, 16) => format!("{field_access} as i32 as u16"),
        (BasicType::Int, 32) => format!("{field_access} as i32"),
        (BasicType::Uint, 32) => format!("{field_access} as i32 as u32"),
        (BasicType::Int, 64) => format!("{field_access} as i64"),
        (BasicType::Uint, 64) => format!("{field_access} as i64 as u64"),
        (BasicType::Float, 32) => format!("{field_access} as i32 as f32"),
        (BasicType::Float, 64) => format!("{field_access} as i64 as f64"),
        _ => format!("{field_access} as i32 as u8"),
    }
}

fn parse_scalar_get_fn(basic_type: BasicType, bit: u32) -> &'static str {
    match (basic_type, bit) {
        (BasicType::Int, 8) => "crate::mavlink_message::get_i8",
        (BasicType::Uint, 8) => "crate::mavlink_message::get_u8",
        (BasicType::Int, 16) => "crate::mavlink_message::get_i16",
        (BasicType::Uint, 16) => "crate::mavlink_message::get_u16",
        (BasicType::Int, 32) => "crate::mavlink_message::get_i32",
        (BasicType::Uint, 32) => "crate::mavlink_message::get_u32",
        (BasicType::Int, 64) => "crate::mavlink_message::get_i64",
        (BasicType::Uint, 64) => "crate::mavlink_message::get_u64",
        (BasicType::Float, 32) => "crate::mavlink_message::get_f32",
        (BasicType::Float, 64) => "crate::mavlink_message::get_f64",
        _ => "crate::mavlink_message::get_u8",
    }
}

fn parse_array_get_fn(basic_type: BasicType, bit: u32, length: u32) -> String {
    let base = match (basic_type, bit) {
        (BasicType::Int, 8) => "get_i8_array",
        (BasicType::Uint, 8) => "get_u8_array",
        (BasicType::Int, 16) => "get_i16_array",
        (BasicType::Uint, 16) => "get_u16_array",
        (BasicType::Int, 32) => "get_i32_array",
        (BasicType::Uint, 32) => "get_u32_array",
        (BasicType::Int, 64) => "get_i64_array",
        (BasicType::Uint, 64) => "get_u64_array",
        (BasicType::Float, 32) => "get_f32_array",
        (BasicType::Float, 64) => "get_f64_array",
        _ => "get_u8_array",
    };
    format!("crate::mavlink_message::{base}::<{length}>")
}

fn serialize_scalar_set_fn(basic_type: BasicType, bit: u32) -> &'static str {
    match (basic_type, bit) {
        (BasicType::Int, 8) => "crate::mavlink_message::set_i8",
        (BasicType::Uint, 8) => "crate::mavlink_message::set_u8",
        (BasicType::Int, 16) => "crate::mavlink_message::set_i16",
        (BasicType::Uint, 16) => "crate::mavlink_message::set_u16",
        (BasicType::Int, 32) => "crate::mavlink_message::set_i32",
        (BasicType::Uint, 32) => "crate::mavlink_message::set_u32",
        (BasicType::Int, 64) => "crate::mavlink_message::set_i64",
        (BasicType::Uint, 64) => "crate::mavlink_message::set_u64",
        (BasicType::Float, 32) => "crate::mavlink_message::set_f32",
        (BasicType::Float, 64) => "crate::mavlink_message::set_f64",
        _ => "crate::mavlink_message::set_u8",
    }
}

fn serialize_array_set_fn(basic_type: BasicType, bit: u32, length: u32) -> String {
    let base = match (basic_type, bit) {
        (BasicType::Int, 8) => "set_i8_array",
        (BasicType::Uint, 8) => "set_u8_array",
        (BasicType::Int, 16) => "set_i16_array",
        (BasicType::Uint, 16) => "set_u16_array",
        (BasicType::Int, 32) => "set_i32_array",
        (BasicType::Uint, 32) => "set_u32_array",
        (BasicType::Int, 64) => "set_i64_array",
        (BasicType::Uint, 64) => "set_u64_array",
        (BasicType::Float, 32) => "set_f32_array",
        (BasicType::Float, 64) => "set_f64_array",
        _ => "set_u8_array",
    };
    format!("crate::mavlink_message::{base}::<{length}>")
}

pub fn as_rust_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[(&str, &str)] = &[
        ("int8_t", "i8"),
        ("uint8_t", "u8"),
        ("int16_t", "i16"),
        ("uint16_t", "u16"),
        ("int32_t", "i32"),
        ("uint32_t", "u32"),
        ("int64_t", "i64"),
        ("uint64_t", "u64"),
        ("char", "u8"),
        ("float", "f32"),
        ("double", "f64"),
    ];

    for (basic_type, rust_type) in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_type_name(enum_name);
            }
            return (*rust_type).to_string();
        }

        let prefix = format!("{basic_type}[");
        if let Some(rest) = mavlink_type.strip_prefix(&prefix)
            && rest.ends_with(']')
            && rest.len() > 1
            && rest[..rest.len() - 1].chars().all(|ch| ch.is_ascii_digit())
        {
            let length = &rest[..rest.len() - 1];
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return format!("[{enum_name}; {length}]");
            }
            return format!("[{rust_type}; {length}]");
        }
    }

    format!("/* Unknown({mavlink_type}) */ u8")
}

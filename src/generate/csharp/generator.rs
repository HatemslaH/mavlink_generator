use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::csharp::util::{
    dialect_name_from_path, enum_class_name, field_property_name_for_message, message_class_name,
    unique_enum_entry_csharp_name,
};
use crate::generate::dart::writer::DartWriter;
use crate::xml::{
    BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage, camel_case,
};

fn write_csharp_doc(writer: &mut DartWriter, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    writer.line("/// <remarks>");
    for line in trimmed.lines() {
        let line = line.trim_start().trim_end();
        if !line.is_empty() {
            writer.line(&format!("/// {line}"));
        }
    }
    writer.line("/// </remarks>");
}

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("using System;");
    writer.line("using System.Linq;");
    writer.line("using Mavlink;");
    writer.blank();
    writer.line("namespace Mavlink.Dialects;");
    writer.blank();

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            write_csharp_doc(&mut writer, description);
        }
        writer.line(&format!("/// <summary>{name}</summary>", name = enm.name));
        let class_name = enum_class_name(&enm.name);
        if enm.is_bitmask {
            writer.line("[Flags]");
        }
        writer.block(&format!("public enum {class_name} {{"), "}", |w| {
            render_enum_entries(w, enm.entries.as_slice());
        });
        writer.blank();
        render_enum_from_value(&mut writer, &class_name, enm.is_bitmask);
    }

    for msg in doc.messages.messages() {
        writer.blank();
        render_message(&mut writer, msg)?;
    }

    writer.blank();
    render_dialect_class(&mut writer, doc, &dialect_name_from_path(src_dialect_path))?;

    Ok(writer.into_string())
}

fn render_enum_entries(writer: &mut DartWriter, entries: &[DialectEntry]) {
    let mut used_enum_entry_names = HashSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_name = unique_enum_entry_csharp_name(entry, &mut used_enum_entry_names);
        let separator = if index + 1 == entries.len() { "" } else { "," };

        if entry.wip {
            writer.line("/// <summary>WIP.</summary>");
        }
        if let Some(description) = &entry.description {
            write_csharp_doc(writer, description);
        }
        writer.line(&format!("/// <summary>{name}</summary>", name = entry.name));
        writer.line(&format!("{entry_name} = {}{separator}", entry.value));
    }
}

fn render_enum_from_value(writer: &mut DartWriter, enum_name: &str, is_bitmask: bool) {
    writer.block(
        &format!("public static class {enum_name}Extensions {{"),
        "}",
        |w| {
            w.block(
                &format!("public static {enum_name} FromValue(int value) {{"),
                "}",
                |w| {
                    w.block(
                        &format!("foreach (var member in Enum.GetValues<{enum_name}>()) {{"),
                        "}",
                        |w| {
                            w.block("if ((int)member == value) {", "}", |w| {
                                w.line("return member;");
                            });
                        },
                    );
                    if is_bitmask {
                        w.line(&format!(
                            "var sorted = Enum.GetValues<{enum_name}>().Where(m => (int)m > 0).OrderByDescending(m => (int)m);"
                        ));
                        w.block("foreach (var member in sorted) {", "}", |w| {
                            w.block("if ((value & (int)member) != 0) {", "}", |w| {
                                w.line("return member;");
                            });
                        });
                        w.block(
                            &format!("foreach (var member in Enum.GetValues<{enum_name}>()) {{"),
                            "}",
                            |w| {
                                w.block("if ((int)member == 0) {", "}", |w| {
                                    w.line("return member;");
                                });
                            },
                        );
                    }
                    w.line(&format!("return Enum.GetValues<{enum_name}>()[0];"));
                },
            );
        },
    );
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let class_name = message_class_name(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    write_csharp_doc(writer, &msg.description);
    writer.line(&format!("/// <summary>{name}</summary>", name = msg.name));
    writer.try_block(
        &format!("public sealed class {class_name} : MavlinkMessage {{"),
        "}",
        |w| {
            w.line(&format!("public const int MsgId = {};", msg.id));
            w.line(&format!("public const int CrcExtra = {crc_extra};"));
            w.line(&format!(
                "public const int MavlinkEncodedLength = {encoded_length};"
            ));
            w.blank();
            w.line("public override int MavlinkMessageId => MsgId;");
            w.line("public override int MavlinkCrcExtra => CrcExtra;");
            w.blank();

            for field in &msg.ordered_fields {
                w.blank();
                render_field(w, field, &class_name);
            }

            w.blank();
            render_constructor(w, msg, &class_name)?;
            w.blank();
            render_parse_factory(w, msg, &class_name)?;
            w.blank();
            render_serialize(w, msg, &class_name)?;
            Ok(())
        },
    )
}

fn render_field(writer: &mut DartWriter, field: &DialectField, message_class: &str) {
    write_csharp_doc(writer, &field.description);
    writer.line(&format!(
        "/// <summary>MAVLink type: {ty}</summary>",
        ty = field.field_type
    ));
    if let Some(units) = &field.units {
        writer.line(&format!("/// <summary>units: {units}</summary>"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line(&format!(
            "/// <summary>enum: [{enum_name}]</summary>",
            enum_name = camel_case(enum_name)
        ));
    }
    if field.is_extension {
        writer.line("/// <summary>Extensions field for MAVLink 2.</summary>");
    }
    writer.line(&format!("/// <summary>{name}</summary>", name = field.name));
    writer.line(&format!(
        "public {} {prop} {{ get; init; }}",
        as_csharp_type(
            &field.field_type,
            field.enum_name.as_deref(),
            field.is_bitmask
        ),
        prop = field_property_name_for_message(&field.name, message_class)
    ));
}

fn render_constructor(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    class_name: &str,
) -> Result<()> {
    let params: Vec<String> = msg
        .ordered_fields
        .iter()
        .map(|field| {
            format!(
                "{} {}",
                as_csharp_type(
                    &field.field_type,
                    field.enum_name.as_deref(),
                    field.is_bitmask
                ),
                field_property_name_for_message(&field.name, class_name)
            )
        })
        .collect();

    writer.line(&format!(
        "public {class_name}({params}) {{",
        params = params.join(", ")
    ));
    writer.indent();
    for field in &msg.ordered_fields {
        let prop = field_property_name_for_message(&field.name, class_name);
        writer.line(&format!("{prop} = {prop};"));
    }
    writer.dedent();
    writer.line("}");
    Ok(())
}

fn render_parse_factory(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    class_name: &str,
) -> Result<()> {
    writer.try_block(
        &format!("public static {class_name} Parse(ReadOnlySpan<byte> data) {{"),
        "}",
        |w| {
            w.block("if (data.Length < MavlinkEncodedLength) {", "}", |w| {
                w.line("data = PadPayload(data, MavlinkEncodedLength);");
            });

            let mut byte_offset = 0;
            for field in &msg.ordered_fields {
                byte_offset += render_parse_field(w, field, byte_offset, class_name)?;
            }

            w.blank();
            w.line(&format!("return new {class_name}("));
            w.indent();
            for (index, field) in msg.ordered_fields.iter().enumerate() {
                let prop = field_property_name_for_message(&field.name, class_name);
                let comma = if index + 1 == msg.ordered_fields.len() {
                    ""
                } else {
                    ","
                };
                w.line(&format!("{prop}: {prop}{comma}"));
            }
            w.dedent();
            w.line(");");
            Ok(())
        },
    )
}

fn uses_unsigned_wire(field: &DialectField) -> bool {
    let unit = crate::xml::ParsedMavlinkType::unit_type(&field.field_type);
    unit.starts_with("uint") || unit.starts_with("char")
}

fn wire_basic_type(field: &DialectField, parsed: &crate::xml::ParsedMavlinkType) -> BasicType {
    if uses_unsigned_wire(field) {
        BasicType::Uint
    } else {
        parsed.basic_type
    }
}

fn serialize_field_access(
    field: &DialectField,
    parsed: &crate::xml::ParsedMavlinkType,
    wire_type: BasicType,
    prop: &str,
) -> String {
    if field.enum_name.is_some() || field.is_bitmask {
        match (wire_type, parsed.bit) {
            (BasicType::Uint, 8) => format!("(byte)(int){prop}"),
            (BasicType::Uint, 16) => format!("(ushort)(int){prop}"),
            (BasicType::Uint, 32) => format!("(uint)(int){prop}"),
            (BasicType::Uint, 64) => format!("(ulong)(int){prop}"),
            (BasicType::Int, 8) => format!("(sbyte)(int){prop}"),
            (BasicType::Int, 16) => format!("(short)(int){prop}"),
            (BasicType::Int, 32) => format!("(int){prop}"),
            (BasicType::Int, 64) => format!("(long)(int){prop}"),
            _ => format!("(int){prop}"),
        }
    } else {
        prop.to_string()
    }
}

fn render_parse_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
    message_class: &str,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let enum_type_name = field
        .enum_name
        .as_deref()
        .map(enum_class_name)
        .unwrap_or_default();
    let prop = field_property_name_for_message(&field.name, message_class);
    let wire_type = wire_basic_type(field, &parsed);

    if parsed.is_array() {
        if is_enum {
            match wire_type {
                BasicType::Int => writer.line(&format!(
                    "var {prop}Raw = MavlinkMessage.GetInt{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
                BasicType::Uint => writer.line(&format!(
                    "var {prop}Raw = MavlinkMessage.GetUint{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
                BasicType::Float => writer.line(&format!(
                    "var {prop}Raw = MavlinkMessage.GetFloat{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
            }
            let from_value = if wire_type == BasicType::Uint {
                format!("(int)v")
            } else {
                "v".to_string()
            };
            writer.line(&format!(
                "var {prop} = {prop}Raw.Select(v => {enum_type_name}Extensions.FromValue({from_value})).ToArray();",
                prop = prop,
                enum_type_name = enum_type_name,
                from_value = from_value
            ));
        } else {
            match wire_type {
                BasicType::Int => writer.line(&format!(
                    "var {prop} = MavlinkMessage.GetInt{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
                BasicType::Uint => writer.line(&format!(
                    "var {prop} = MavlinkMessage.GetUint{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
                BasicType::Float => writer.line(&format!(
                    "var {prop} = MavlinkMessage.GetFloat{bit}List(data, {byte_offset}, {len});",
                    bit = parsed.bit,
                    len = parsed.array_length,
                    prop = prop
                )),
            }
        }
    } else if is_enum {
        match wire_type {
            BasicType::Int => writer.line(&format!(
                "var {prop}Raw = MavlinkMessage.GetInt{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
            BasicType::Uint => writer.line(&format!(
                "var {prop}Raw = MavlinkMessage.GetUint{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
            BasicType::Float => writer.line(&format!(
                "var {prop}Raw = MavlinkMessage.GetFloat{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
        }
        writer.line(&format!(
            "var {prop} = {enum_type_name}Extensions.FromValue({from_value_arg});",
            prop = prop,
            enum_type_name = enum_type_name,
            from_value_arg = if wire_type == BasicType::Uint {
                format!("(int){prop}Raw")
            } else {
                format!("{prop}Raw")
            }
        ));
    } else {
        match wire_type {
            BasicType::Int => writer.line(&format!(
                "var {prop} = MavlinkMessage.GetInt{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
            BasicType::Uint => writer.line(&format!(
                "var {prop} = MavlinkMessage.GetUint{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
            BasicType::Float => writer.line(&format!(
                "var {prop} = MavlinkMessage.GetFloat{bit}(data, {byte_offset});",
                bit = parsed.bit,
                prop = prop
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) -> Result<()> {
    writer.try_block("public override byte[] Serialize() {", "}", |w| {
        w.line("var data = new byte[MavlinkEncodedLength];");

        let mut byte_offset = 0;
        for field in &msg.ordered_fields {
            byte_offset += render_serialize_field(w, field, byte_offset, class_name)?;
        }

        w.line("return data;");
        Ok(())
    })
}

fn render_serialize_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
    message_class: &str,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let prop = field_property_name_for_message(&field.name, message_class);
    let field_access = format!("{prop}");
    let wire_type = wire_basic_type(field, &parsed);

    if parsed.is_array() {
        if is_enum {
            match wire_type {
                BasicType::Int if parsed.bit == 8 => {
                    writer.line(&format!(
                        "var {prop}Serialized = {prop}.Select(v => (sbyte)(int)v).ToArray();",
                        prop = prop
                    ));
                    writer.line(&format!(
                        "MavlinkMessage.SetInt{bit}List(data, {byte_offset}, {prop}Serialized);",
                        bit = parsed.bit,
                        prop = prop
                    ));
                }
                BasicType::Int => {
                    writer.line(&format!(
                        "var {prop}Serialized = {prop}.Select(v => (int)v).ToArray();",
                        prop = prop
                    ));
                    writer.line(&format!(
                        "MavlinkMessage.SetInt{bit}List(data, {byte_offset}, {prop}Serialized);",
                        bit = parsed.bit,
                        prop = prop
                    ));
                }
                BasicType::Uint => {
                    writer.line(&format!(
                        "var {prop}Serialized = {prop}.Select(v => (byte)(int)v).ToArray();",
                        prop = prop
                    ));
                    writer.line(&format!(
                        "MavlinkMessage.SetUint{bit}List(data, {byte_offset}, {prop}Serialized);",
                        bit = parsed.bit,
                        prop = prop
                    ));
                }
                BasicType::Float => {
                    writer.line(&format!(
                        "var {prop}Serialized = {prop}.Select(v => (float)(double)v).ToArray();",
                        prop = prop
                    ));
                    writer.line(&format!(
                        "MavlinkMessage.SetFloat{bit}List(data, {byte_offset}, {prop}Serialized);",
                        bit = parsed.bit,
                        prop = prop
                    ));
                }
            }
        } else {
            match wire_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage.SetInt{bit}List(data, {byte_offset}, {field_access});",
                    bit = parsed.bit
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage.SetUint{bit}List(data, {byte_offset}, {field_access});",
                    bit = parsed.bit
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage.SetFloat{bit}List(data, {byte_offset}, {field_access});",
                    bit = parsed.bit
                )),
            }
        }
    } else {
        let access = serialize_field_access(field, &parsed, wire_type, &prop);
        match wire_type {
            BasicType::Int => writer.line(&format!(
                "MavlinkMessage.SetInt{bit}(data, {byte_offset}, {access});",
                bit = parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "MavlinkMessage.SetUint{bit}(data, {byte_offset}, {access});",
                bit = parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "MavlinkMessage.SetFloat{bit}(data, {byte_offset}, {access});",
                bit = parsed.bit
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_dialect_class(
    writer: &mut DartWriter,
    doc: &DialectDocument,
    dialect_name: &str,
) -> Result<()> {
    let class_name = format!("MavlinkDialect{dialect_name}");

    writer.try_block(
        &format!("public sealed class {class_name} : MavlinkDialect {{"),
        "}",
        |w| {
        w.line(&format!("public const int MavlinkVersion = {};", doc.version));
        w.blank();
        w.line("public override int Version => MavlinkVersion;");
        w.blank();
        w.line("public override MavlinkMessage? Parse(int messageId, ReadOnlySpan<byte> data) => messageId switch");
        w.line("{");
        w.indent();
        for msg in doc.messages.messages() {
            let class_name = message_class_name(&msg.name);
            w.line(&format!("{class_name}.MsgId => {class_name}.Parse(data),"));
        }
        w.line("_ => null,");
        w.dedent();
        w.line("};");
        w.blank();
        w.line("public override int CrcExtra(int messageId) => messageId switch");
        w.line("{");
        w.indent();
        for msg in doc.messages.messages() {
            let class_name = message_class_name(&msg.name);
            w.line(&format!("{class_name}.MsgId => {class_name}.CrcExtra,"));
        }
        w.line("_ => -1,");
        w.dedent();
        w.line("};");
        Ok(())
    })
}

pub fn as_csharp_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[(&str, &str)] = &[
        ("int8_t", "sbyte"),
        ("uint8_t", "byte"),
        ("int16_t", "short"),
        ("uint16_t", "ushort"),
        ("int32_t", "int"),
        ("uint32_t", "uint"),
        ("int64_t", "long"),
        ("uint64_t", "ulong"),
        ("char", "byte"),
        ("float", "float"),
        ("double", "double"),
    ];

    for (basic_type, csharp_type) in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_class_name(enum_name);
            }
            return (*csharp_type).to_string();
        }

        let prefix = format!("{basic_type}[");
        if let Some(rest) = mavlink_type.strip_prefix(&prefix)
            && rest.ends_with(']')
            && rest.len() > 1
            && rest[..rest.len() - 1].chars().all(|ch| ch.is_ascii_digit())
        {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return format!("{}[]", enum_class_name(enum_name));
            }
            return format!("{csharp_type}[]");
        }
    }

    format!("object /* Unknown({mavlink_type}) */")
}

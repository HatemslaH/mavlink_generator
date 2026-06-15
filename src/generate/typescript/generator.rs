use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::dart::writer::DartWriter;
use crate::generate::typescript::util::{
    dialect_name_from_path, enum_class_name, message_class_name, unique_enum_entry_typescript_name,
};
use crate::xml::{
    BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage, camel_case,
};

fn write_ts_doc(writer: &mut DartWriter, text: &str) {
    for line in text.lines() {
        let trimmed = line.trim_start().trim_end();
        if !trimmed.is_empty() {
            writer.line(&format!("/** {trimmed} */"));
        }
    }
}

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("import type { MavlinkDialect } from '../mavlink_dialect';");
    writer.line("import { MavlinkMessage } from '../mavlink_message';");

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            write_ts_doc(&mut writer, description);
        }
        writer.line(&format!("/** {} */", enm.name));
        let class_name = enum_class_name(&enm.name);
        writer.block(&format!("export enum {class_name} {{"), "}", |w| {
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
        let entry_name = unique_enum_entry_typescript_name(entry, &mut used_enum_entry_names);
        let separator = if index + 1 == entries.len() { "" } else { "," };

        if entry.wip {
            writer.line("/** WIP. */");
        }
        if let Some(description) = &entry.description {
            write_ts_doc(writer, description);
        }
        writer.line(&format!("/** {} */", entry.name));
        writer.line(&format!("{entry_name} = {}{separator}", entry.value));
    }
}

fn render_enum_from_value(writer: &mut DartWriter, enum_name: &str, is_bitmask: bool) {
    writer.block(&format!("export namespace {enum_name} {{"), "}", |w| {
        w.block(
            &format!("export function fromValue(value: number): {enum_name} {{"),
            "}",
            |w| {
                if is_bitmask {
                    w.line("// Exact match first");
                    w.block(
                        &format!(
                            "for (const member of Object.values({enum_name}).filter((v): v is number => typeof v === 'number') as {enum_name}[]) {{"
                        ),
                        "}",
                        |w| {
                            w.line("if (member === value) {");
                            w.indent();
                            w.line("return member;");
                            w.dedent();
                            w.line("}");
                        },
                    );
                    w.line("// For bitmasks, find the highest priority set bit");
                    w.line(&format!(
                        "const sorted = (Object.values({enum_name}).filter((v): v is number => typeof v === 'number' && v > 0) as {enum_name}[]).sort((a, b) => b - a);"
                    ));
                    w.block("for (const member of sorted) {", "}", |w| {
                        w.line("if ((value & member) !== 0) {");
                        w.indent();
                        w.line("return member;");
                        w.dedent();
                        w.line("}");
                    });
                    w.line(&format!(
                        "const zero = Object.values({enum_name}).find((v): v is number => typeof v === 'number' && v === 0) as {enum_name} | undefined;"
                    ));
                    w.line(&format!(
                        "return zero ?? (Object.values({enum_name}).find((v): v is number => typeof v === 'number') as {enum_name});"
                    ));
                } else {
                    w.block(
                        &format!(
                            "for (const member of Object.values({enum_name}).filter((v): v is number => typeof v === 'number') as {enum_name}[]) {{"
                        ),
                        "}",
                        |w| {
                            w.line("if (member === value) {");
                            w.indent();
                            w.line("return member;");
                            w.dedent();
                            w.line("}");
                        },
                    );
                    w.line(&format!(
                        "return Object.values({enum_name}).find((v): v is number => typeof v === 'number') as {enum_name};"
                    ));
                }
            },
        );
    });
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let class_name = message_class_name(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    write_ts_doc(writer, &msg.description);
    writer.line(&format!("/** {} */", msg.name));
    writer.try_block(
        &format!("export class {class_name} extends MavlinkMessage {{"),
        "}",
        |w| {
            w.line(&format!("static readonly MSG_ID = {};", msg.id));
            w.line(&format!("static readonly CRC_EXTRA = {crc_extra};"));
            w.line(&format!(
                "static readonly MAVLINK_ENCODED_LENGTH = {encoded_length};"
            ));
            w.blank();
            w.line("constructor(");
            w.indent();
            for (index, field) in msg.ordered_fields.iter().enumerate() {
                let comma = if index + 1 == msg.ordered_fields.len() {
                    ""
                } else {
                    ","
                };
                w.line(&format!(
                    "public readonly {}: {}{comma}",
                    field.name,
                    as_typescript_type(
                        &field.field_type,
                        field.enum_name.as_deref(),
                        field.is_bitmask
                    )
                ));
            }
            w.dedent();
            w.line(") {");
            w.indent();
            w.line("super();");
            w.dedent();
            w.line("}");

            for field in &msg.ordered_fields {
                w.blank();
                render_field_doc(w, field);
            }

            w.blank();
            w.line("get mavlinkMessageId(): number {");
            w.indent();
            w.line(&format!("return {class_name}.MSG_ID;"));
            w.dedent();
            w.line("}");
            w.blank();
            w.line("get mavlinkCrcExtra(): number {");
            w.indent();
            w.line(&format!("return {class_name}.CRC_EXTRA;"));
            w.dedent();
            w.line("}");

            w.blank();
            render_copy_with(w, msg, &class_name);
            w.blank();
            render_parse_factory(w, msg, &class_name)?;
            w.blank();
            render_serialize(w, msg, &class_name)?;
            Ok(())
        },
    )
}

fn render_field_doc(writer: &mut DartWriter, field: &DialectField) {
    write_ts_doc(writer, &field.description);
    writer.line(&format!("/** MAVLink type: {} */", field.field_type));
    if let Some(units) = &field.units {
        writer.line(&format!("/** units: {units} */"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line(&format!("/** enum: [{}] */", camel_case(enum_name)));
    }
    if field.is_extension {
        writer.line("/** Extensions field for MAVLink 2. */");
    }
    writer.line(&format!("/** {} */", field.name));
}

fn render_copy_with(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) {
    writer.line(&format!(
        "copyWith(partial: Partial<{class_name}>): {class_name} {{"
    ));
    writer.indent();
    writer.line(&format!("return new {class_name}("));
    writer.indent();
    for (index, field) in msg.ordered_fields.iter().enumerate() {
        let comma = if index + 1 == msg.ordered_fields.len() {
            ""
        } else {
            ","
        };
        writer.line(&format!(
            "partial.{name} ?? this.{name}{comma}",
            name = field.name
        ));
    }
    writer.dedent();
    writer.line(");");
    writer.dedent();
    writer.line("}");
}

fn render_parse_factory(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    class_name: &str,
) -> Result<()> {
    writer.try_block(
        &format!("static parse(wire: Uint8Array): {class_name} {{"),
        "}",
        |w| {
            w.line("let bytes = wire;");
            w.block(
                &format!("if (bytes.length < {class_name}.MAVLINK_ENCODED_LENGTH) {{"),
                "}",
                |w| {
                    w.line(&format!(
                        "const padded = new Uint8Array({class_name}.MAVLINK_ENCODED_LENGTH);"
                    ));
                    w.line("padded.set(bytes);");
                    w.line("bytes = padded;");
                },
            );

            let mut byte_offset = 0;
            for field in &msg.ordered_fields {
                byte_offset += render_parse_field(w, field, byte_offset, "bytes")?;
            }

            w.blank();
            w.line(&format!("return new {class_name}("));
            w.indent();
            for (index, field) in msg.ordered_fields.iter().enumerate() {
                let comma = if index + 1 == msg.ordered_fields.len() {
                    ""
                } else {
                    ","
                };
                w.line(&format!("{}{comma}", field.name));
            }
            w.dedent();
            w.line(");");
            Ok(())
        },
    )
}

fn render_parse_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
    buffer_name: &str,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let enum_type_name = field
        .enum_name
        .as_deref()
        .map(enum_class_name)
        .unwrap_or_default();

    let data = buffer_name;

    if parsed.is_array() {
        if is_enum {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage.getInt{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
                BasicType::Uint => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage.getUint{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
                BasicType::Float => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage.getFloat{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
            }
            writer.line(&format!(
                "const {field} = {field}_raw.map((v) => {enum_type_name}.fromValue(v));",
                field = field.name,
                enum_type_name = enum_type_name
            ));
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "const {field} = MavlinkMessage.getInt{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
                BasicType::Uint => writer.line(&format!(
                    "const {field} = MavlinkMessage.getUint{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
                BasicType::Float => writer.line(&format!(
                    "const {field} = MavlinkMessage.getFloat{}List({data}, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name,
                    data = data
                )),
            }
        }
    } else if is_enum {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "const {field}_raw = MavlinkMessage.getInt{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
            BasicType::Uint => writer.line(&format!(
                "const {field}_raw = MavlinkMessage.getUint{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
            BasicType::Float => writer.line(&format!(
                "const {field}_raw = MavlinkMessage.getFloat{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
        }
        writer.line(&format!(
            "const {field} = {enum_type_name}.fromValue({field}_raw);",
            field = field.name,
            enum_type_name = enum_type_name
        ));
    } else {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "const {field} = MavlinkMessage.getInt{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
            BasicType::Uint => writer.line(&format!(
                "const {field} = MavlinkMessage.getUint{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
            BasicType::Float => writer.line(&format!(
                "const {field} = MavlinkMessage.getFloat{}({data}, {byte_offset});",
                parsed.bit,
                field = field.name,
                data = data
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) -> Result<()> {
    writer.try_block("serialize(): Uint8Array {", "}", |w| {
        w.line(&format!(
            "const buffer = new Uint8Array({class_name}.MAVLINK_ENCODED_LENGTH);"
        ));

        let mut byte_offset = 0;
        for field in &msg.ordered_fields {
            byte_offset += render_serialize_field(w, field, byte_offset)?;
        }

        w.line("return buffer;");
        Ok(())
    })
}

fn render_serialize_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;

    if parsed.is_array() {
        if is_enum {
            writer.line(&format!(
                "const {field}_serialized = this.{field}.map((v) => v as number);",
                field = field.name
            ));
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage.setInt{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage.setUint{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage.setFloat{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
            }
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage.setInt{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage.setUint{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage.setFloat{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
            }
        }
    } else {
        let access = if is_enum {
            format!("this.{} as number", field.name)
        } else {
            format!("this.{}", field.name)
        };
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "MavlinkMessage.setInt{}(buffer, {byte_offset}, {access});",
                parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "MavlinkMessage.setUint{}(buffer, {byte_offset}, {access});",
                parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "MavlinkMessage.setFloat{}(buffer, {byte_offset}, {access});",
                parsed.bit
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
    writer.try_block(
        &format!("export class MavlinkDialect{dialect_name} implements MavlinkDialect {{"),
        "}",
        |w| {
            w.line(&format!(
                "static readonly MAVLINK_VERSION = {};",
                doc.version
            ));
            w.blank();
            w.line("get version(): number {");
            w.indent();
            w.line(&format!(
                "return MavlinkDialect{dialect_name}.MAVLINK_VERSION;"
            ));
            w.dedent();
            w.line("}");
            w.blank();
            w.line("parse(messageId: number, data: Uint8Array): MavlinkMessage | null {");
            w.indent();
            w.line("switch (messageId) {");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_class_name(&msg.name);
                w.line(&format!("case {class_name}.MSG_ID:"));
                w.indent();
                w.line(&format!("return {class_name}.parse(data);"));
                w.dedent();
            }
            w.line("default:");
            w.indent();
            w.line("return null;");
            w.dedent();
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.blank();
            w.line("crcExtra(messageId: number): number {");
            w.indent();
            w.line("switch (messageId) {");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_class_name(&msg.name);
                w.line(&format!("case {class_name}.MSG_ID:"));
                w.indent();
                w.line(&format!("return {class_name}.CRC_EXTRA;"));
                w.dedent();
            }
            w.line("default:");
            w.indent();
            w.line("return -1;");
            w.dedent();
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            Ok(())
        },
    )
}

pub fn as_typescript_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[&str] = &[
        "int8_t", "uint8_t", "int16_t", "uint16_t", "int32_t", "uint32_t", "int64_t", "uint64_t",
        "char", "float", "double",
    ];

    for basic_type in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_class_name(enum_name);
            }
            return "number".to_string();
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
            return "number[]".to_string();
        }
    }

    format!("unknown /* {mavlink_type} */")
}

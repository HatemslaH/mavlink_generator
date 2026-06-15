use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::dart::writer::DartWriter;
use crate::generate::javascript::util::{
    dialect_name_from_path, enum_class_name, message_class_name, unique_enum_entry_js_name,
};
use crate::xml::{BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage};

fn write_js_doc(writer: &mut DartWriter, text: &str) {
    for line in text.lines() {
        let trimmed = line.trim_start().trim_end();
        if !trimmed.is_empty() {
            writer.line(&format!("// {trimmed}"));
        }
    }
}

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("import { MavlinkDialect } from '../mavlink_dialect.js';");
    writer.line("import { MavlinkMessage } from '../mavlink_message.js';");
    writer.line("import '../mavlink_types.js';");

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            write_js_doc(&mut writer, description);
        }
        writer.line(&format!("// {}", enm.name));
        let class_name = enum_class_name(&enm.name);
        writer.line(&format!("export const {class_name} = {{"));
        writer.indent();
        render_enum_entries(&mut writer, enm.entries.as_slice());
        writer.dedent();
        writer.line("};");
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
        let entry_name = unique_enum_entry_js_name(entry, &mut used_enum_entry_names);
        let separator = if index + 1 == entries.len() { "" } else { "," };

        if entry.wip {
            writer.line("// WIP.");
        }
        if let Some(description) = &entry.description {
            write_js_doc(writer, description);
        }
        writer.line(&format!("// {}", entry.name));
        writer.line(&format!("{entry_name}: {}{separator}", entry.value));
    }
}

fn render_enum_from_value(writer: &mut DartWriter, enum_name: &str, is_bitmask: bool) {
    writer.line(&format!("{enum_name}.fromValue = function(value) {{"));
    writer.indent();
    if is_bitmask {
        writer.line("// Exact match first");
        writer.block(
            &format!("for (const key of Object.keys({enum_name})) {{"),
            "}",
            |w| {
                w.line("if (key === 'fromValue') continue;");
                w.line(&format!(
                    "if ({enum_name}[key] === value) return {enum_name}[key];"
                ));
            },
        );
        writer.line("// For bitmasks, find the highest priority set bit");
        writer.line(&format!(
            "const sorted = Object.keys({enum_name}).filter((k) => k !== 'fromValue' && {enum_name}[k] > 0).sort((a, b) => {enum_name}[b] - {enum_name}[a]);"
        ));
        writer.block("for (const key of sorted) {", "}", |w| {
            w.line(&format!(
                "if ((value & {enum_name}[key]) !== 0) return {enum_name}[key];"
            ));
        });
        writer.line(&format!(
            "const zero = Object.keys({enum_name}).find((k) => k !== 'fromValue' && {enum_name}[k] === 0);"
        ));
        writer.line(&format!(
            "return zero !== undefined ? {enum_name}[zero] : {enum_name}[Object.keys({enum_name}).find((k) => k !== 'fromValue')];"
        ));
    } else {
        writer.block(
            &format!("for (const key of Object.keys({enum_name})) {{"),
            "}",
            |w| {
                w.line("if (key === 'fromValue') continue;");
                w.line(&format!(
                    "if ({enum_name}[key] === value) return {enum_name}[key];"
                ));
            },
        );
        writer.line(&format!(
            "return {enum_name}[Object.keys({enum_name}).find((k) => k !== 'fromValue')];"
        ));
    }
    writer.dedent();
    writer.line("};");
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let class_name = message_class_name(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    write_js_doc(writer, &msg.description);
    writer.line(&format!("// {}", msg.name));
    writer.try_block(
        &format!("export class {class_name} extends MavlinkMessage {{"),
        "}",
        |w| {
            w.line(&format!("static MSG_ID = {};", msg.id));
            w.line(&format!("static CRC_EXTRA = {crc_extra};"));
            w.line(&format!(
                "static MAVLINK_ENCODED_LENGTH = {encoded_length};"
            ));
            w.blank();
            w.line("get mavlinkMessageId() {");
            w.indent();
            w.line(&format!("return {class_name}.MSG_ID;"));
            w.dedent();
            w.line("}");
            w.blank();
            w.line("get mavlinkCrcExtra() {");
            w.indent();
            w.line(&format!("return {class_name}.CRC_EXTRA;"));
            w.dedent();
            w.line("}");
            w.blank();
            render_constructor(w, msg, &class_name);
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

fn render_constructor(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) {
    let _ = class_name;
    let params: Vec<String> = msg.ordered_fields.iter().map(|f| f.name.clone()).collect();
    writer.line(&format!("constructor({}) {{", params.join(", ")));
    writer.indent();
    writer.line("super();");
    for field in &msg.ordered_fields {
        writer.line(&format!("this.{} = {};", field.name, field.name));
    }
    writer.dedent();
    writer.line("}");
}

fn render_copy_with(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) {
    writer.line("copyWith(overrides = {}) {");
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
            "overrides.{field} ?? this.{field}{comma}",
            field = field.name
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
    writer.try_block("static parse(wire) {", "}", |w| {
        w.block(
            "if (wire.length < this.MAVLINK_ENCODED_LENGTH) {",
            "}",
            |w| {
                w.line("const padded = new Uint8Array(this.MAVLINK_ENCODED_LENGTH);");
                w.line("padded.set(wire);");
                w.line("wire = padded;");
            },
        );

        let mut byte_offset = 0;
        for field in &msg.ordered_fields {
            byte_offset += render_parse_field(w, field, byte_offset)?;
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
            w.line(&format!("{field}{comma}", field = field.name));
        }
        w.dedent();
        w.line(");");
        Ok(())
    })
}

fn render_parse_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let enum_type_name = field
        .enum_name
        .as_deref()
        .map(enum_class_name)
        .unwrap_or_default();

    if parsed.is_array() {
        if is_enum {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage._getInt{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage._getUint{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "const {field}_raw = MavlinkMessage._getFloat{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
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
                    "const {field} = MavlinkMessage._getInt{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "const {field} = MavlinkMessage._getUint{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "const {field} = MavlinkMessage._getFloat{}List(wire, {byte_offset}, {});",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
            }
        }
    } else if is_enum {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "const {field}_raw = MavlinkMessage._getInt{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
            )),
            BasicType::Uint => writer.line(&format!(
                "const {field}_raw = MavlinkMessage._getUint{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
            )),
            BasicType::Float => writer.line(&format!(
                "const {field}_raw = MavlinkMessage._getFloat{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
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
                "const {field} = MavlinkMessage._getInt{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
            )),
            BasicType::Uint => writer.line(&format!(
                "const {field} = MavlinkMessage._getUint{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
            )),
            BasicType::Float => writer.line(&format!(
                "const {field} = MavlinkMessage._getFloat{}(wire, {byte_offset});",
                parsed.bit,
                field = field.name
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage, class_name: &str) -> Result<()> {
    writer.try_block("serialize() {", "}", |w| {
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
                "const {field}_serialized = this.{field}.map((v) => v);",
                field = field.name
            ));
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage._setInt{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage._setUint{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage._setFloat{}List(buffer, {byte_offset}, {field}_serialized);",
                    parsed.bit,
                    field = field.name
                )),
            }
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage._setInt{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage._setUint{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage._setFloat{}List(buffer, {byte_offset}, this.{field});",
                    parsed.bit,
                    field = field.name
                )),
            }
        }
    } else {
        let access = if is_enum {
            format!("this.{}", field.name)
        } else {
            format!("this.{}", field.name)
        };
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "MavlinkMessage._setInt{}(buffer, {byte_offset}, {access});",
                parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "MavlinkMessage._setUint{}(buffer, {byte_offset}, {access});",
                parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "MavlinkMessage._setFloat{}(buffer, {byte_offset}, {access});",
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
        &format!("export class MavlinkDialect{dialect_name} extends MavlinkDialect {{"),
        "}",
        |w| {
            w.line(&format!("static MAVLINK_VERSION = {};", doc.version));
            w.blank();
            w.line("get version() {");
            w.indent();
            w.line(&format!(
                "return MavlinkDialect{dialect_name}.MAVLINK_VERSION;"
            ));
            w.dedent();
            w.line("}");
            w.blank();
            w.line("parse(messageId, data) {");
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
            w.line("crcExtra(messageId) {");
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

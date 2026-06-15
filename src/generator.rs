use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::dart_writer::DartWriter;
use crate::dialect_entry::DialectEntry;
use crate::dialect_field::DialectField;
use crate::dialect_message::DialectMessage;
use crate::document::DialectDocument;
use crate::error::Result;
use crate::mavlink_type::BasicType;
use crate::util::{camel_case, dialect_name_from_path, unique_enum_entry_dart_name};

pub fn generate_code(dst_path: impl AsRef<Path>, src_dialect_path: impl AsRef<Path>) -> Result<()> {
    let dst_path = dst_path.as_ref();
    let src_dialect_path = src_dialect_path.as_ref();
    let doc = DialectDocument::parse(src_dialect_path)?;
    let content = render_dialect(&doc, src_dialect_path)?;

    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dst_path, content)?;
    Ok(())
}

fn render_dialect(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("import 'dart:typed_data';");
    writer.line("import '../mavlink.dart';");

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            writer.documentation(description);
        }
        writer.line("///");
        writer.line(&format!("/// {}", enm.name));
        writer.block(&format!("enum {} {{", enm.name_for_dart), "}", |w| {
            render_enum_entries(w, enm.entries.as_slice());
            w.blank();
            w.line(&format!("const {}(this.value);", enm.name_for_dart));
            w.line("final int value;");
            w.blank();
            render_enum_from_value(w, &enm.name_for_dart, enm.is_bitmask);
        });
    }

    for msg in doc.messages.messages() {
        writer.blank();
        render_message(&mut writer, msg)?;
    }

    writer.blank();
    render_dialect_class(
        &mut writer,
        doc,
        &dialect_name_from_path(src_dialect_path),
    )?;

    Ok(writer.into_string())
}

fn render_enum_entries(writer: &mut DartWriter, entries: &[DialectEntry]) {
    let mut used_enum_entry_names = HashSet::new();

    for (index, entry) in entries.iter().enumerate() {
        if index > 0 {
            writer.blank();
        }

        let separator = if index + 1 == entries.len() { ';' } else { ',' };
        let entry_dart_name = unique_enum_entry_dart_name(entry, &mut used_enum_entry_names);

        if entry.wip {
            writer.line("/// WIP.");
        }
        if let Some(description) = &entry.description {
            writer.documentation(description);
        }
        writer.line("///");
        writer.line(&format!("/// {}", entry.name));
        if let Some(deprecated) = &entry.deprecated {
            write_deprecated(
                writer,
                &format!(
                    "Replaced by [{}] since {}. {}",
                    deprecated.replaced_by, deprecated.since, deprecated.text
                ),
            );
        }
        writer.line(&format!("{entry_dart_name}({}){separator}", entry.value));
    }
}

fn render_enum_from_value(writer: &mut DartWriter, enum_name: &str, is_bitmask: bool) {
    writer.block(&format!("static {enum_name} fromValue(int value) {{"), "}", |w| {
        if is_bitmask {
            w.line("// Exact match first");
            w.block(&format!("for (var e in {enum_name}.values) {{"), "}", |w| {
                w.line("if (e.value == value) return e;");
            });
            w.line("// For bitmasks, find the highest priority set bit");
            w.line("// Sort by value descending to check highest bits first");
            write_bitmask_sorted(w, enum_name);
            w.block("for (var e in sorted) {", "}", |w| {
                w.line("if ((value & e.value) != 0) return e;");
            });
            w.line("// No bits set, try to find value 0 or return first value");
            write_first_where_fallback(w, enum_name);
        } else {
            w.block(&format!("for (var e in {enum_name}.values) {{"), "}", |w| {
                w.line("if (e.value == value) return e;");
            });
            w.line("// Value not found, return first enum value");
            w.line(&format!("return {enum_name}.values.first;"));
        }
    });
}

fn write_bitmask_sorted(writer: &mut DartWriter, enum_name: &str) {
    let where_expr = format!("{enum_name}.values.where((e) => e.value > 0)");
    let cascade = "..sort((a, b) => b.value.compareTo(a.value));";
    let single = format!("var sorted = {where_expr}.toList(){cascade}");

    if writer.fits(&single) {
        writer.line(&single);
        return;
    }

    let split_at_to_list = format!("var sorted = {where_expr}.toList()");
    if writer.fits(&split_at_to_list) {
        writer.line(&split_at_to_list);
        writer.indent();
        writer.line(cascade);
        writer.dedent();
        return;
    }

    writer.line("var sorted =");
    writer.indent();
    writer.indent();
    writer.line(&format!("{where_expr}.toList()"));
    writer.indent();
    writer.line(cascade);
    writer.dedent();
    writer.dedent();
    writer.dedent();
}

fn write_first_where_fallback(writer: &mut DartWriter, enum_name: &str) {
    let single = format!(
        "return {enum_name}.values.firstWhere((e) => e.value == 0, orElse: () => {enum_name}.values.first);"
    );

    if writer.fits(&single) {
        writer.line(&single);
        return;
    }

    writer.line(&format!("return {enum_name}.values.firstWhere("));
    writer.indent();
    writer.line("(e) => e.value == 0,");
    writer.line(&format!("orElse: () => {enum_name}.values.first,"));
    writer.dedent();
    writer.line(");");
}

fn write_named_assignment(writer: &mut DartWriter, name: &str, value: &str) {
    let single = format!("{name}: {value},");
    if writer.fits(&single) {
        writer.line(&single);
        return;
    }

    writer.line(&format!("{name}:"));
    writer.indent();
    writer.line(&format!("{value},"));
    writer.dedent();
}

fn write_deprecated(writer: &mut DartWriter, text: &str) {
    let single = format!("@Deprecated(\"{text}\")");
    if writer.fits(&single) {
        writer.line(&single);
        return;
    }

    writer.line("@Deprecated(");
    writer.indent();
    writer.line(&format!("\"{text}\","));
    writer.dedent();
    writer.line(")");
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    writer.documentation(&msg.description);
    writer.line("///");
    writer.line(&format!("/// {}", msg.name));

    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    writer.try_block(
        &format!("class {} implements MavlinkMessage {{", msg.name_for_dart),
        "}",
        |w| {
            w.line(&format!("static const int msgId = {};", msg.id));
            w.blank();
            w.line(&format!("static const int crcExtra = {crc_extra};"));
            w.blank();
            w.line(&format!(
                "static const int mavlinkEncodedLength = {encoded_length};"
            ));
            w.blank();
            w.line("@override");
            w.line("int get mavlinkMessageId => msgId;");
            w.blank();
            w.line("@override");
            w.line("int get mavlinkCrcExtra => crcExtra;");

            for field in &msg.ordered_fields {
                w.blank();
                render_field(w, field);
            }

            w.blank();
            render_constructor(w, msg);
            w.blank();
            render_copy_with(w, msg);
            w.blank();
            render_parse_factory(w, msg)?;
            w.blank();
            render_serialize(w, msg)?;
            Ok(())
        },
    )
}

fn render_field(writer: &mut DartWriter, field: &DialectField) {
    writer.documentation(&field.description);
    writer.line("///");
    writer.line(&format!("/// MAVLink type: {}", field.field_type));
    if let Some(units) = &field.units {
        writer.line("///");
        writer.line(&format!("/// units: {units}"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line("///");
        writer.line(&format!("/// enum: [{}]", camel_case(enum_name)));
    }
    if field.is_extension {
        writer.line("///");
        writer.line("/// Extensions field for MAVLink 2.");
    }
    writer.line("///");
    writer.line(&format!("/// {}", field.name));
    writer.line(&format!(
        "final {} {};",
        as_dart_type(
            &field.field_type,
            field.enum_name.as_deref(),
            field.is_bitmask
        ),
        field.name_for_dart
    ));
}

fn render_constructor(writer: &mut DartWriter, msg: &DialectMessage) {
    if msg.ordered_fields.is_empty() {
        writer.line(&format!("{}();", msg.name_for_dart));
        return;
    }

    writer.line(&format!("{}({{", msg.name_for_dart));
    writer.indent();
    for field in &msg.ordered_fields {
        writer.line(&format!("required this.{},", field.name_for_dart));
    }
    writer.dedent();
    writer.line("});");
}

fn render_copy_with(writer: &mut DartWriter, msg: &DialectMessage) {
    writer.line(&format!("{} copyWith({{", msg.name_for_dart));
    writer.indent();
    for field in &msg.ordered_fields {
        writer.line(&format!(
            "{}? {},",
            as_dart_type(
                &field.field_type,
                field.enum_name.as_deref(),
                field.is_bitmask
            ),
            field.name_for_dart
        ));
    }
    writer.dedent();
    writer.line("}) {");
    writer.indent();
    writer.line(&format!("return {}(", msg.name_for_dart));
    writer.indent();
    for field in &msg.ordered_fields {
        write_named_assignment(
            writer,
            &field.name_for_dart,
            &format!(
                "{} ?? this.{}",
                field.name_for_dart, field.name_for_dart
            ),
        );
    }
    writer.dedent();
    writer.line(");");
    writer.dedent();
    writer.line("}");
}

fn render_parse_factory(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    writer.try_block(
        &format!("factory {}.parse(ByteData data_) {{", msg.name_for_dart),
        "}",
        |w| {
            w.block(
                &format!(
                    "if (data_.lengthInBytes < {}.mavlinkEncodedLength) {{",
                    msg.name_for_dart
                ),
                "}",
                |w| {
                    w.line(&format!(
                        "var len = {}.mavlinkEncodedLength - data_.lengthInBytes;",
                        msg.name_for_dart
                    ));
                    w.line("var d =");
                    w.indent();
                    w.line("data_.buffer.asUint8List().sublist(0, data_.lengthInBytes) +");
                    w.line("List<int>.filled(len, 0);");
                    w.dedent();
                    w.line("data_ = Uint8List.fromList(d).buffer.asByteData();");
                },
            );

            let mut byte_offset = 0;
            for field in &msg.ordered_fields {
                byte_offset += render_parse_field(w, field, byte_offset)?;
            }

            w.blank();
            w.line(&format!("return {}(", msg.name_for_dart));
            w.indent();
            for field in &msg.ordered_fields {
                write_named_assignment(w, &field.name_for_dart, &field.name_for_dart);
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
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let endian_argument = if parsed.bit == 8 {
        String::new()
    } else {
        ", Endian.little".to_string()
    };
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let enum_type_name = field
        .enum_name
        .as_deref()
        .map(camel_case)
        .unwrap_or_default();

    if parsed.is_array() {
        if is_enum {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "var {}_raw = MavlinkMessage.asInt{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
                BasicType::Uint => writer.line(&format!(
                    "var {}_raw = MavlinkMessage.asUint{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
                BasicType::Float => writer.line(&format!(
                    "var {}_raw = MavlinkMessage.asFloat{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
            }
            writer.line(&format!(
                "var {} = {}_raw.map((v) => {}.fromValue(v)).toList();",
                field.name_for_dart, field.name_for_dart, enum_type_name
            ));
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "var {} = MavlinkMessage.asInt{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
                BasicType::Uint => writer.line(&format!(
                    "var {} = MavlinkMessage.asUint{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
                BasicType::Float => writer.line(&format!(
                    "var {} = MavlinkMessage.asFloat{}List(data_, {byte_offset}, {});",
                    field.name_for_dart, parsed.bit, parsed.array_length
                )),
            }
        }
    } else if is_enum {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "var {}_raw = data_.getInt{}({byte_offset}{endian_argument});",
                field.name_for_dart, parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "var {}_raw = data_.getUint{}({byte_offset}{endian_argument});",
                field.name_for_dart, parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "var {}_raw = data_.getFloat{}({byte_offset}, Endian.little);",
                field.name_for_dart, parsed.bit
            )),
        }
        writer.line(&format!(
            "var {} = {}.fromValue({}_raw);",
            field.name_for_dart, enum_type_name, field.name_for_dart
        ));
    } else {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "var {} = data_.getInt{}({byte_offset}{endian_argument});",
                field.name_for_dart, parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "var {} = data_.getUint{}({byte_offset}{endian_argument});",
                field.name_for_dart, parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "var {} = data_.getFloat{}({byte_offset}, Endian.little);",
                field.name_for_dart, parsed.bit
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    writer.line("@override");
    writer.try_block("ByteData serialize() {", "}", |w| {
        w.line("var data_ = ByteData(mavlinkEncodedLength);");

        let mut byte_offset = 0;
        for field in &msg.ordered_fields {
            byte_offset += render_serialize_field(w, field, byte_offset)?;
        }

        w.line("return data_;");
        Ok(())
    })
}

fn render_serialize_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: u32,
) -> Result<u32> {
    let parsed = field.parsed_type()?;
    let endian_argument = if parsed.bit == 8 {
        String::new()
    } else {
        ", Endian.little".to_string()
    };
    let is_enum = field.enum_name.is_some() && !field.is_bitmask;
    let field_access = if is_enum {
        format!("{}.value", field.name_for_dart)
    } else {
        field.name_for_dart.clone()
    };

    if parsed.is_array() {
        if is_enum {
            writer.line(&format!(
                "var {}_serialized = {}.map((e) => e.value).toList();",
                field.name_for_dart, field.name_for_dart
            ));
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage.setInt{}List(data_, {byte_offset}, {}_serialized);",
                    parsed.bit, field.name_for_dart
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage.setUint{}List(data_, {byte_offset}, {}_serialized);",
                    parsed.bit, field.name_for_dart
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage.setFloat{}List(data_, {byte_offset}, {}_serialized);",
                    parsed.bit, field.name_for_dart
                )),
            }
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage.setInt{}List(data_, {byte_offset}, {});",
                    parsed.bit, field.name_for_dart
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage.setUint{}List(data_, {byte_offset}, {});",
                    parsed.bit, field.name_for_dart
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage.setFloat{}List(data_, {byte_offset}, {});",
                    parsed.bit, field.name_for_dart
                )),
            }
        }
    } else {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "data_.setInt{}({byte_offset}, {field_access}{endian_argument});",
                parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "data_.setUint{}({byte_offset}, {field_access}{endian_argument});",
                parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "data_.setFloat{}({byte_offset}, {field_access}, Endian.little);",
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
        &format!("class MavlinkDialect{dialect_name} implements MavlinkDialect {{"),
        "}",
        |w| {
            w.line(&format!("static const int mavlinkVersion = {};", doc.version));
            w.blank();
            w.line("@override");
            w.line("int get version => mavlinkVersion;");
            w.blank();
            w.line("@override");
            w.block("MavlinkMessage? parse(int messageID, ByteData data) {", "}", |w| {
                w.block("switch (messageID) {", "}", |w| {
                    for msg in doc.messages.messages() {
                        w.line(&format!("case {}:", msg.id));
                        w.indent();
                        w.line(&format!("return {}.parse(data);", msg.name_for_dart));
                        w.dedent();
                    }
                    w.line("default:");
                    w.indent();
                    w.line("return null;");
                    w.dedent();
                });
            });
            w.blank();
            w.line("@override");
            w.block("int crcExtra(int messageID) {", "}", |w| {
                w.block("switch (messageID) {", "}", |w| {
                    for msg in doc.messages.messages() {
                        w.line(&format!("case {}:", msg.id));
                        w.indent();
                        w.line(&format!("return {}.crcExtra;", msg.name_for_dart));
                        w.dedent();
                    }
                    w.line("default:");
                    w.indent();
                    w.line("return -1;");
                    w.dedent();
                });
            });
            Ok(())
        },
    )
}

pub fn as_dart_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[&str] = &[
        "int8_t", "uint8_t", "int16_t", "uint16_t", "int32_t", "uint32_t", "int64_t", "uint64_t",
        "char", "float", "double",
    ];

    for basic_type in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return camel_case(enum_name);
            }
            return (*basic_type).to_string();
        }

        let prefix = format!("{basic_type}[");
        if let Some(rest) = mavlink_type.strip_prefix(&prefix)
            && rest.ends_with(']')
            && rest.len() > 1
            && rest[..rest.len() - 1].chars().all(|ch| ch.is_ascii_digit())
        {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return format!("List<{}>", camel_case(enum_name));
            }
            return format!("List<{basic_type}>");
        }
    }

    format!("Unknown({mavlink_type})")
}

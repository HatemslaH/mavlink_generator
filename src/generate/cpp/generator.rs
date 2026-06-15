use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::cpp::util::{
    as_cpp_base_type, as_cpp_type, dialect_name_from_path, dialect_struct_name, message_prefix,
    message_struct_name, message_type_name, unique_enum_entry_cpp_name,
};
use crate::generate::dart::writer::DartWriter;
use crate::xml::{BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage};

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let dialect_name = dialect_name_from_path(src_dialect_path);
    let dialect_struct = dialect_struct_name(&dialect_name);

    let mut writer = DartWriter::new();
    writer.line("#pragma once");
    writer.blank();
    writer.line("#include \"../types.hpp\"");
    writer.line("#include \"../mavlink_message.hpp\"");
    writer.line("#include \"../mavlink_dialect.hpp\"");
    writer.blank();
    writer.line("namespace mavlink {");
    writer.indent();

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            writer.documentation(description);
        }
        writer.line("///");
        writer.line(&format!("/// {}", enm.name));
        let underlying = enum_underlying_type(enm.entries.as_slice());
        writer.line(&format!("enum {} : {underlying} {{", enm.name));
        writer.indent();
        render_enum_entries(&mut writer, enm.entries.as_slice());
        writer.dedent();
        writer.line("};");
    }

    for msg in doc.messages.messages() {
        writer.blank();
        render_message(&mut writer, msg)?;
    }

    writer.blank();
    render_dialect(&mut writer, doc, &dialect_name, &dialect_struct)?;

    writer.dedent();
    writer.line("}  // namespace mavlink");
    writer.blank();

    Ok(writer.into_string())
}

fn enum_underlying_type(entries: &[DialectEntry]) -> &'static str {
    let mut min_value = 0i64;
    let mut max_value = 0i64;

    for entry in entries {
        let value = i64::from(entry.value);
        min_value = min_value.min(value);
        max_value = max_value.max(value);
    }

    if min_value < 0 {
        if min_value >= i64::from(i8::MIN) && max_value <= i64::from(i8::MAX) {
            return "int8_t";
        }
        if min_value >= i64::from(i16::MIN) && max_value <= i64::from(i16::MAX) {
            return "int16_t";
        }
        if min_value >= i64::from(i32::MIN) && max_value <= i64::from(i32::MAX) {
            return "int32_t";
        }
        return "int64_t";
    }

    if max_value <= u8::MAX as i64 {
        "uint8_t"
    } else if max_value <= u16::MAX as i64 {
        "uint16_t"
    } else if max_value <= u32::MAX as i64 {
        "uint32_t"
    } else {
        "uint64_t"
    }
}

fn render_enum_entries(writer: &mut DartWriter, entries: &[DialectEntry]) {
    let mut used_enum_entry_names = HashSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_name = unique_enum_entry_cpp_name(entry, &mut used_enum_entry_names);
        let separator = if index + 1 == entries.len() { "" } else { "," };

        if entry.wip {
            writer.line("/// WIP.");
        }
        if let Some(description) = &entry.description {
            writer.documentation(description);
        }
        writer.line("///");
        writer.line(&format!("/// {}", entry.name));
        writer.line(&format!("{entry_name} = {}{separator}", entry.value));
    }
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let type_name = message_type_name(&msg.name);
    let struct_alias = message_struct_name(&msg.name);
    let prefix = message_prefix(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    writer.documentation(&msg.description);
    writer.line("///");
    writer.line(&format!("/// {}", msg.name));
    writer.line(&format!(
        "inline constexpr uint32_t {prefix}_MSG_ID = {};",
        msg.id
    ));
    writer.line(&format!(
        "inline constexpr uint8_t {prefix}_CRC_EXTRA = {crc_extra};"
    ));
    writer.line(&format!(
        "inline constexpr size_t {prefix}_ENCODED_LENGTH = {encoded_length};"
    ));
    writer.line(&format!("struct {type_name} {{"));
    writer.indent();
    for field in &msg.ordered_fields {
        render_field(writer, field)?;
    }
    writer.dedent();
    writer.line("};");
    writer.line(&format!("using {struct_alias} = {type_name};"));
    writer.blank();
    render_serialize(writer, msg, &type_name, &prefix)?;
    writer.blank();
    render_parse(writer, msg, &type_name, &prefix)?;
    Ok(())
}

fn render_field(writer: &mut DartWriter, field: &DialectField) -> Result<()> {
    writer.documentation(&field.description);
    writer.line("///");
    writer.line(&format!("/// MAVLink type: {}", field.field_type));
    if let Some(units) = &field.units {
        writer.line("///");
        writer.line(&format!("/// units: {units}"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line("///");
        writer.line(&format!("/// enum: [{enum_name}]"));
    }
    if field.is_extension {
        writer.line("///");
        writer.line("/// Extensions field for MAVLink 2.");
    }
    writer.line("///");
    writer.line(&format!("/// {}", field.name));
    let parsed = field.parsed_type()?;
    if parsed.is_array() {
        let base_type = as_cpp_base_type(
            &field.field_type,
            field.enum_name.as_deref(),
            field.is_bitmask,
        );
        writer.line(&format!(
            "{} {}[{}];",
            base_type, field.name, parsed.array_length
        ));
    } else {
        writer.line(&format!(
            "{} {};",
            as_cpp_type(
                &field.field_type,
                field.enum_name.as_deref(),
                field.is_bitmask
            ),
            field.name
        ));
    }
    Ok(())
}

fn render_serialize(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    type_name: &str,
    prefix: &str,
) -> Result<()> {
    writer.try_block(
        &format!("inline void {prefix}_serialize(const {type_name}& msg, uint8_t* data) {{"),
        "}",
        |w| {
            w.line(&format!(
                "mavlink_memset_s(data, {prefix}_ENCODED_LENGTH, 0, {prefix}_ENCODED_LENGTH);"
            ));
            let mut byte_offset = 0usize;
            for field in &msg.ordered_fields {
                byte_offset += render_serialize_field(w, field, prefix, byte_offset)?;
            }
            Ok(())
        },
    )
}

fn render_serialize_field(
    writer: &mut DartWriter,
    field: &DialectField,
    prefix: &str,
    byte_offset: usize,
) -> Result<usize> {
    let parsed = field.parsed_type()?;
    let field_access = format!("msg.{}", field.name);
    let capacity = format!("{prefix}_ENCODED_LENGTH");
    let enum_cast = if let Some(_) = field.enum_name.as_deref().filter(|_| !field.is_bitmask) {
        let wire_type = wire_integral_type(parsed.basic_type, parsed.bit);
        format!("static_cast<{wire_type}>({field_access})")
    } else {
        field_access.clone()
    };

    if parsed.is_array() {
        if parsed.basic_type == BasicType::Int
            && parsed.bit == 8
            && field.field_type.starts_with("char")
        {
            writer.line(&format!(
                "mavlink_put_bytes(data, {capacity}, {byte_offset}, reinterpret_cast<const uint8_t*>({field_access}), {});",
                parsed.array_length
            ));
        } else {
            let put_fn = serialize_put_fn(parsed.basic_type, parsed.bit);
            writer.block(
                &format!("for (size_t i = 0; i < {}; i++) {{", parsed.array_length),
                "}",
                |w| {
                    let array_access = if field.enum_name.is_some() && !field.is_bitmask {
                        let wire_type = wire_integral_type(parsed.basic_type, parsed.bit);
                        format!("static_cast<{wire_type}>({}[i])", field_access)
                    } else {
                        format!("{field_access}[i]")
                    };
                    w.line(&format!(
                        "{put_fn}(data, {capacity}, {byte_offset} + i * {}, {array_access});",
                        parsed.byte()
                    ));
                },
            );
        }
    } else {
        let put_fn = serialize_put_fn(parsed.basic_type, parsed.bit);
        writer.line(&format!(
            "{put_fn}(data, {capacity}, {byte_offset}, {enum_cast});"
        ));
    }

    Ok((parsed.byte() * parsed.array_length) as usize)
}

fn serialize_put_fn(basic_type: BasicType, bit: u32) -> &'static str {
    match (basic_type, bit) {
        (BasicType::Int, 8) => "mavlink_put_int8",
        (BasicType::Uint, 8) => "mavlink_put_uint8",
        (BasicType::Int, 16) => "mavlink_put_int16",
        (BasicType::Uint, 16) => "mavlink_put_uint16",
        (BasicType::Int, 32) => "mavlink_put_int32",
        (BasicType::Uint, 32) => "mavlink_put_uint32",
        (BasicType::Int, 64) => "mavlink_put_int64",
        (BasicType::Uint, 64) => "mavlink_put_uint64",
        (BasicType::Float, 32) => "mavlink_put_float",
        (BasicType::Float, 64) => "mavlink_put_double",
        _ => "mavlink_put_uint8",
    }
}

fn wire_integral_type(basic_type: BasicType, bit: u32) -> &'static str {
    match (basic_type, bit) {
        (BasicType::Int, 8) => "int8_t",
        (BasicType::Uint, 8) => "uint8_t",
        (BasicType::Int, 16) => "int16_t",
        (BasicType::Uint, 16) => "uint16_t",
        (BasicType::Int, 32) => "int32_t",
        (BasicType::Uint, 32) => "uint32_t",
        (BasicType::Int, 64) => "int64_t",
        (BasicType::Uint, 64) => "uint64_t",
        _ => "uint8_t",
    }
}

fn render_parse(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    type_name: &str,
    prefix: &str,
) -> Result<()> {
    writer.try_block(
        &format!("inline void {prefix}_parse(const uint8_t* data, {type_name}& msg) {{"),
        "}",
        |w| {
            let mut byte_offset = 0usize;
            for field in &msg.ordered_fields {
                byte_offset += render_parse_field(w, field, byte_offset)?;
            }
            Ok(())
        },
    )
}

fn render_parse_field(
    writer: &mut DartWriter,
    field: &DialectField,
    byte_offset: usize,
) -> Result<usize> {
    let parsed = field.parsed_type()?;
    let field_access = format!("msg.{}", field.name);

    if parsed.is_array() {
        if parsed.basic_type == BasicType::Int
            && parsed.bit == 8
            && field.field_type.starts_with("char")
        {
            writer.line(&format!(
                "mavlink_get_bytes(data, {byte_offset}, reinterpret_cast<uint8_t*>({field_access}), {}, {});",
                parsed.array_length, parsed.array_length
            ));
        } else {
            let get_fn = parse_get_fn(parsed.basic_type, parsed.bit);
            writer.block(
                &format!("for (size_t i = 0; i < {}; i++) {{", parsed.array_length),
                "}",
                |w| {
                    let assignment = if let Some(enum_name) =
                        field.enum_name.as_deref().filter(|_| !field.is_bitmask)
                    {
                        format!(
                            "{field_access}[i] = static_cast<{enum_name}>({get_fn}(data, {byte_offset} + i * {}));",
                            parsed.byte()
                        )
                    } else {
                        format!(
                            "{field_access}[i] = {get_fn}(data, {byte_offset} + i * {});",
                            parsed.byte()
                        )
                    };
                    w.line(&assignment);
                },
            );
        }
    } else {
        let get_fn = parse_get_fn(parsed.basic_type, parsed.bit);
        if let Some(enum_name) = field.enum_name.as_deref().filter(|_| !field.is_bitmask) {
            writer.line(&format!(
                "{field_access} = static_cast<{enum_name}>({get_fn}(data, {byte_offset}));"
            ));
        } else {
            writer.line(&format!("{field_access} = {get_fn}(data, {byte_offset});"));
        }
    }

    Ok((parsed.byte() * parsed.array_length) as usize)
}

fn parse_get_fn(basic_type: BasicType, bit: u32) -> &'static str {
    match (basic_type, bit) {
        (BasicType::Int, 8) => "mavlink_get_int8",
        (BasicType::Uint, 8) => "mavlink_get_uint8",
        (BasicType::Int, 16) => "mavlink_get_int16",
        (BasicType::Uint, 16) => "mavlink_get_uint16",
        (BasicType::Int, 32) => "mavlink_get_int32",
        (BasicType::Uint, 32) => "mavlink_get_uint32",
        (BasicType::Int, 64) => "mavlink_get_int64",
        (BasicType::Uint, 64) => "mavlink_get_uint64",
        (BasicType::Float, 32) => "mavlink_get_float",
        (BasicType::Float, 64) => "mavlink_get_double",
        _ => "mavlink_get_uint8",
    }
}

fn render_dialect(
    writer: &mut DartWriter,
    doc: &DialectDocument,
    dialect_name: &str,
    dialect_struct: &str,
) -> Result<()> {
    let dialect_var = format!("mavlink_dialect_{}", dialect_name.to_lowercase());

    writer.line(&format!("struct {dialect_struct} {{"));
    writer.indent();
    writer.line("dialect_t base;");
    writer.dedent();
    writer.line("};");
    writer.blank();

    writer.block(
        &format!(
            "inline int {dialect_var}_crc_extra(const dialect_t* dialect, uint32_t message_id) {{"
        ),
        "}",
        |w| {
            w.line("(void)dialect;");
            w.block("switch (message_id) {", "}", |w| {
                for msg in doc.messages.messages() {
                    let prefix = message_prefix(&msg.name);
                    w.line(&format!("case {prefix}_MSG_ID:"));
                    w.indent();
                    w.line(&format!("return {prefix}_CRC_EXTRA;"));
                    w.dedent();
                }
                w.line("default:");
                w.indent();
                w.line("return -1;");
                w.dedent();
            });
        },
    );

    writer.blank();
    writer.block(
        &format!(
            "inline bool {dialect_var}_parse(const dialect_t* dialect, uint32_t message_id, const uint8_t* payload, size_t payload_len, void* out_message) {{"
        ),
        "}",
        |w| {
            w.line("(void)dialect;");
            w.line("(void)payload_len;");
            w.block("switch (message_id) {", "}", |w| {
                for msg in doc.messages.messages() {
                    let prefix = message_prefix(&msg.name);
                    let struct_name = message_struct_name(&msg.name);
                    w.line(&format!("case {prefix}_MSG_ID:"));
                    w.indent();
                    w.line(&format!(
                        "{prefix}_parse(payload, *static_cast<{struct_name}*>(out_message));"
                    ));
                    w.line("return true;");
                    w.dedent();
                }
                w.line("default:");
                w.indent();
                w.line("return false;");
                w.dedent();
            });
        },
    );

    writer.blank();
    writer.block(
        &format!("inline void {dialect_var}_init({dialect_struct}& dialect) {{"),
        "}",
        |w| {
            w.line(&format!("dialect.base.version = {};", doc.version));
            w.line(&format!("dialect.base.parse = {dialect_var}_parse;"));
            w.line(&format!(
                "dialect.base.crc_extra = {dialect_var}_crc_extra;"
            ));
        },
    );

    Ok(())
}

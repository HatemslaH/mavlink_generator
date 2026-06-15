use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::c::util::{
    as_c_base_type, as_c_type, dialect_guard_name, dialect_name_from_path, dialect_struct_name,
    message_prefix, message_struct_name, unique_enum_entry_c_name,
};
use crate::generate::dart::writer::DartWriter;
use crate::xml::{BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage};

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let dialect_stem = src_dialect_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let dialect_name = dialect_name_from_path(src_dialect_path);
    let guard = dialect_guard_name(&dialect_stem);
    let dialect_struct = dialect_struct_name(&dialect_name);

    let mut writer = DartWriter::new();
    writer.line(&format!("#ifndef {guard}"));
    writer.line(&format!("#define {guard}"));
    writer.blank();
    writer.line("#include \"../types.h\"");
    writer.line("#include \"../mavlink_message.h\"");
    writer.line("#include \"../mavlink_dialect.h\"");
    writer.blank();

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            writer.documentation(description);
        }
        writer.line("///");
        writer.line(&format!("/// {}", enm.name));
        writer.line("typedef enum {");
        writer.indent();
        render_enum_entries(&mut writer, enm.entries.as_slice());
        writer.dedent();
        writer.line(&format!("}} {};", enm.name));
    }

    for msg in doc.messages.messages() {
        writer.blank();
        render_message(&mut writer, msg)?;
    }

    writer.blank();
    render_dialect(&mut writer, doc, &dialect_name, &dialect_struct)?;

    writer.blank();
    writer.line("#endif");
    writer.blank();

    Ok(writer.into_string())
}

fn render_enum_entries(writer: &mut DartWriter, entries: &[DialectEntry]) {
    let mut used_enum_entry_names = HashSet::new();

    for (index, entry) in entries.iter().enumerate() {
        let entry_name = unique_enum_entry_c_name(entry, &mut used_enum_entry_names);
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
    let struct_name = message_struct_name(&msg.name);
    let prefix = message_prefix(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    writer.documentation(&msg.description);
    writer.line("///");
    writer.line(&format!("/// {}", msg.name));
    writer.line(&format!("#define {prefix}_MSG_ID {}", msg.id));
    writer.line(&format!("#define {prefix}_CRC_EXTRA {crc_extra}"));
    writer.line(&format!("#define {prefix}_ENCODED_LENGTH {encoded_length}"));
    writer.line("typedef struct {");
    writer.indent();
    for field in &msg.ordered_fields {
        render_field(writer, field)?;
    }
    writer.dedent();
    writer.line(&format!("}} {struct_name};"));
    writer.blank();
    render_serialize(writer, msg, &struct_name, &prefix)?;
    writer.blank();
    render_parse(writer, msg, &struct_name, &prefix)?;
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
        let base_type = as_c_base_type(
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
            as_c_type(
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
    struct_name: &str,
    prefix: &str,
) -> Result<()> {
    writer.try_block(
        &format!(
            "static inline void {prefix}_serialize(const {struct_name} *msg, uint8_t *data) {{"
        ),
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
    let field_access = format!("msg->{}", field.name);
    let capacity = format!("{prefix}_ENCODED_LENGTH");

    if parsed.is_array() {
        if parsed.basic_type == BasicType::Int
            && parsed.bit == 8
            && field.field_type.starts_with("char")
        {
            writer.line(&format!(
                "mavlink_put_bytes(data, {capacity}, {byte_offset}, (const uint8_t *){field_access}, {});",
                parsed.array_length
            ));
        } else {
            let put_fn = serialize_put_fn(parsed.basic_type, parsed.bit);
            writer.block(
                &format!("for (size_t i = 0; i < {}; i++) {{", parsed.array_length),
                "}",
                |w| {
                    w.line(&format!(
                        "{put_fn}(data, {capacity}, {byte_offset} + i * {}, {}[i]);",
                        parsed.byte(),
                        field_access
                    ));
                },
            );
        }
    } else {
        let put_fn = serialize_put_fn(parsed.basic_type, parsed.bit);
        writer.line(&format!(
            "{put_fn}(data, {capacity}, {byte_offset}, {field_access});"
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

fn render_parse(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    struct_name: &str,
    prefix: &str,
) -> Result<()> {
    writer.try_block(
        &format!("static inline void {prefix}_parse(const uint8_t *data, {struct_name} *msg) {{"),
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
    let field_access = format!("msg->{}", field.name);

    if parsed.is_array() {
        if parsed.basic_type == BasicType::Int
            && parsed.bit == 8
            && field.field_type.starts_with("char")
        {
            writer.line(&format!(
                "mavlink_get_bytes(data, {byte_offset}, (uint8_t *){field_access}, {}, {});",
                parsed.array_length, parsed.array_length
            ));
        } else {
            let get_fn = parse_get_fn(parsed.basic_type, parsed.bit);
            writer.block(
                &format!("for (size_t i = 0; i < {}; i++) {{", parsed.array_length),
                "}",
                |w| {
                    w.line(&format!(
                        "{field_access}[i] = {get_fn}(data, {byte_offset} + i * {});",
                        parsed.byte()
                    ));
                },
            );
        }
    } else {
        let get_fn = parse_get_fn(parsed.basic_type, parsed.bit);
        writer.line(&format!("{field_access} = {get_fn}(data, {byte_offset});"));
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

    writer.line(&format!("typedef struct {dialect_struct} {{"));
    writer.indent();
    writer.line("mavlink_dialect_t base;");
    writer.dedent();
    writer.line(&format!("}} {dialect_struct};"));
    writer.blank();

    writer.block(
        &format!(
            "static inline int {dialect_var}_crc_extra(const mavlink_dialect_t *dialect, uint32_t message_id) {{"
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
            "static inline bool {dialect_var}_parse(const mavlink_dialect_t *dialect, uint32_t message_id, const uint8_t *payload, size_t payload_len, void *out_message) {{"
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
                        "{prefix}_parse(payload, ({struct_name} *)out_message);"
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
        &format!("static inline void {dialect_var}_init({dialect_struct} *dialect) {{"),
        "}",
        |w| {
            w.line(&format!("dialect->base.version = {};", doc.version));
            w.line(&format!("dialect->base.parse = {dialect_var}_parse;"));
            w.line(&format!(
                "dialect->base.crc_extra = {dialect_var}_crc_extra;"
            ));
        },
    );

    Ok(())
}

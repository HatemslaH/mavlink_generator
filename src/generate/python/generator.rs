use std::collections::HashSet;
use std::path::Path;

use crate::error::Result;
use crate::generate::dart::writer::DartWriter;
use crate::generate::python::util::{
    dialect_name_from_path, enum_class_name, message_class_name, unique_enum_entry_python_name,
};
use crate::xml::{
    BasicType, DialectDocument, DialectEntry, DialectField, DialectMessage, camel_case,
};

fn write_python_doc(writer: &mut DartWriter, text: &str) {
    for line in text.lines() {
        let trimmed = line.trim_start().trim_end();
        if !trimmed.is_empty() {
            writer.line(&format!("# {trimmed}"));
        }
    }
}

pub fn render(doc: &DialectDocument, src_dialect_path: &Path) -> Result<String> {
    let mut writer = DartWriter::new();
    writer.line("from __future__ import annotations");
    writer.blank();
    writer.line("import sys");
    writer.line("from dataclasses import dataclass, replace");
    writer.line("from enum import IntEnum");
    writer.line("from pathlib import Path");
    writer.line("from typing import ClassVar");
    writer.blank();
    writer.line("_ROOT = Path(__file__).resolve().parent.parent");
    writer.line("if str(_ROOT) not in sys.path:");
    writer.indent();
    writer.line("sys.path.insert(0, str(_ROOT))");
    writer.dedent();
    writer.blank();
    writer.line("from mavlink_dialect import MavlinkDialect");
    writer.line("from mavlink_message import MavlinkMessage");
    writer.line("from mavlink_types import *");

    for enm in doc.enums.enums() {
        writer.blank();
        if let Some(description) = &enm.description {
            write_python_doc(&mut writer, description);
        }
        writer.line(&format!("# {}", enm.name));
        let class_name = enum_class_name(&enm.name);
        writer.block(&format!("class {class_name}(IntEnum):"), "", |w| {
            render_enum_entries(w, enm.entries.as_slice());
            w.blank();
            render_enum_from_value(w, &class_name, enm.is_bitmask);
        });
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

    for entry in entries {
        let entry_name = unique_enum_entry_python_name(entry, &mut used_enum_entry_names);

        if entry.wip {
            writer.line("# WIP.");
        }
        if let Some(description) = &entry.description {
            write_python_doc(writer, description);
        }
        writer.line(&format!("# {}", entry.name));
        writer.line(&format!("{entry_name} = {}", entry.value));
    }
}

fn render_enum_from_value(writer: &mut DartWriter, enum_name: &str, is_bitmask: bool) {
    writer.line("@classmethod");
    writer.block(
        &format!("def from_value(cls, value: int) -> {enum_name}:"),
        "",
        |w| {
            if is_bitmask {
                w.line("# Exact match first");
                w.block("for member in cls:", "", |w| {
                    w.line("if member.value == value:");
                    w.indent();
                    w.line("return member");
                    w.dedent();
                });
                w.line("# For bitmasks, find the highest priority set bit");
                w.line(
                    "sorted_members = sorted((m for m in cls if m.value > 0), key=lambda m: m.value, reverse=True)",
                );
                w.block("for member in sorted_members:", "", |w| {
                    w.line("if (value & member.value) != 0:");
                    w.indent();
                    w.line("return member");
                    w.dedent();
                });
                w.line("return next((m for m in cls if m.value == 0), next(iter(cls)))");
            } else {
                w.block("for member in cls:", "", |w| {
                    w.line("if member.value == value:");
                    w.indent();
                    w.line("return member");
                    w.dedent();
                });
                w.line("return next(iter(cls))");
            }
        },
    );
}

fn render_message(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    let class_name = message_class_name(&msg.name);
    let crc_extra = msg.calculate_crc_extra()?;
    let encoded_length = msg.calculate_encoded_length()?;

    write_python_doc(writer, &msg.description);
    writer.line(&format!("# {}", msg.name));
    writer.line("@dataclass");
    writer.try_block(&format!("class {class_name}(MavlinkMessage):"), "", |w| {
        w.line(&format!("MSG_ID: ClassVar[int] = {}", msg.id));
        w.line(&format!("CRC_EXTRA: ClassVar[int] = {crc_extra}"));
        w.line(&format!(
            "MAVLINK_ENCODED_LENGTH: ClassVar[int] = {encoded_length}"
        ));
        w.blank();
        w.line("@property");
        w.line("def mavlink_message_id(self) -> int:");
        w.indent();
        w.line("return self.MSG_ID");
        w.dedent();
        w.blank();
        w.line("@property");
        w.line("def mavlink_crc_extra(self) -> int:");
        w.indent();
        w.line("return self.CRC_EXTRA");
        w.dedent();

        for field in &msg.ordered_fields {
            w.blank();
            render_field(w, field);
        }

        w.blank();
        render_copy_with(w, &class_name);
        w.blank();
        render_parse_factory(w, msg, &class_name)?;
        w.blank();
        render_serialize(w, msg)?;
        Ok(())
    })
}

fn render_field(writer: &mut DartWriter, field: &DialectField) {
    write_python_doc(writer, &field.description);
    writer.line(&format!("# MAVLink type: {}", field.field_type));
    if let Some(units) = &field.units {
        writer.line(&format!("# units: {units}"));
    }
    if let Some(enum_name) = &field.enum_name {
        writer.line(&format!("# enum: [{}]", camel_case(enum_name)));
    }
    if field.is_extension {
        writer.line("# Extensions field for MAVLink 2.");
    }
    writer.line(&format!("# {}", field.name));
    writer.line(&format!(
        "{}: {}",
        field.name,
        as_python_type(
            &field.field_type,
            field.enum_name.as_deref(),
            field.is_bitmask
        )
    ));
}

fn render_copy_with(writer: &mut DartWriter, class_name: &str) {
    writer.line(&format!("def copy_with(self, **kwargs) -> {class_name}:"));
    writer.indent();
    writer.line(&format!("return replace(self, **kwargs)"));
    writer.dedent();
}

fn render_parse_factory(
    writer: &mut DartWriter,
    msg: &DialectMessage,
    class_name: &str,
) -> Result<()> {
    writer.line("@classmethod");
    writer.try_block(
        &format!("def parse(cls, data_: bytes) -> {class_name}:"),
        "",
        |w| {
            w.block("if len(data_) < cls.MAVLINK_ENCODED_LENGTH:", "", |w| {
                w.line("data_ = data_ + bytes(cls.MAVLINK_ENCODED_LENGTH - len(data_))");
            });

            let mut byte_offset = 0;
            for field in &msg.ordered_fields {
                byte_offset += render_parse_field(w, field, byte_offset)?;
            }

            w.blank();
            w.line("return cls(");
            w.indent();
            for (index, field) in msg.ordered_fields.iter().enumerate() {
                let comma = if index + 1 == msg.ordered_fields.len() {
                    ""
                } else {
                    ","
                };
                w.line(&format!("{field}={field}{comma}", field = field.name));
            }
            w.dedent();
            w.line(")");
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
                    "{field}_raw = MavlinkMessage._get_int{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "{field}_raw = MavlinkMessage._get_uint{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "{field}_raw = MavlinkMessage._get_float{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
            }
            writer.line(&format!(
                "{field} = [{enum_type_name}.from_value(v) for v in {field}_raw]",
                field = field.name,
                enum_type_name = enum_type_name
            ));
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "{field} = MavlinkMessage._get_int{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "{field} = MavlinkMessage._get_uint{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "{field} = MavlinkMessage._get_float{}_list(data_, {byte_offset}, {})",
                    parsed.bit,
                    parsed.array_length,
                    field = field.name
                )),
            }
        }
    } else if is_enum {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "{field}_raw = MavlinkMessage._get_int{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
            BasicType::Uint => writer.line(&format!(
                "{field}_raw = MavlinkMessage._get_uint{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
            BasicType::Float => writer.line(&format!(
                "{field}_raw = MavlinkMessage._get_float{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
        }
        writer.line(&format!(
            "{field} = {enum_type_name}.from_value({field}_raw)",
            field = field.name,
            enum_type_name = enum_type_name
        ));
    } else {
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "{field} = MavlinkMessage._get_int{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
            BasicType::Uint => writer.line(&format!(
                "{field} = MavlinkMessage._get_uint{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
            BasicType::Float => writer.line(&format!(
                "{field} = MavlinkMessage._get_float{}(data_, {byte_offset})",
                parsed.bit,
                field = field.name
            )),
        }
    }

    Ok(parsed.byte() * parsed.array_length)
}

fn render_serialize(writer: &mut DartWriter, msg: &DialectMessage) -> Result<()> {
    writer.try_block("def serialize(self) -> bytes:", "", |w| {
        w.line("data_ = bytearray(self.MAVLINK_ENCODED_LENGTH)");

        let mut byte_offset = 0;
        for field in &msg.ordered_fields {
            byte_offset += render_serialize_field(w, field, byte_offset)?;
        }

        w.line("return bytes(data_)");
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
    let field_access = if is_enum {
        format!("[v.value for v in self.{}]", field.name)
    } else {
        format!("self.{}", field.name)
    };

    if parsed.is_array() {
        if is_enum {
            writer.line(&format!(
                "{field}_serialized = [v.value for v in self.{field}]",
                field = field.name
            ));
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage._set_int{}_list(data_, {byte_offset}, {field}_serialized)",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage._set_uint{}_list(data_, {byte_offset}, {field}_serialized)",
                    parsed.bit,
                    field = field.name
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage._set_float{}_list(data_, {byte_offset}, {field}_serialized)",
                    parsed.bit,
                    field = field.name
                )),
            }
        } else {
            match parsed.basic_type {
                BasicType::Int => writer.line(&format!(
                    "MavlinkMessage._set_int{}_list(data_, {byte_offset}, {field_access})",
                    parsed.bit
                )),
                BasicType::Uint => writer.line(&format!(
                    "MavlinkMessage._set_uint{}_list(data_, {byte_offset}, {field_access})",
                    parsed.bit
                )),
                BasicType::Float => writer.line(&format!(
                    "MavlinkMessage._set_float{}_list(data_, {byte_offset}, {field_access})",
                    parsed.bit
                )),
            }
        }
    } else {
        let access = if is_enum {
            format!("self.{}.value", field.name)
        } else {
            format!("self.{}", field.name)
        };
        match parsed.basic_type {
            BasicType::Int => writer.line(&format!(
                "MavlinkMessage._set_int{}(data_, {byte_offset}, {access})",
                parsed.bit
            )),
            BasicType::Uint => writer.line(&format!(
                "MavlinkMessage._set_uint{}(data_, {byte_offset}, {access})",
                parsed.bit
            )),
            BasicType::Float => writer.line(&format!(
                "MavlinkMessage._set_float{}(data_, {byte_offset}, {access})",
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
        &format!("class MavlinkDialect{dialect_name}(MavlinkDialect):"),
        "",
        |w| {
            w.line(&format!("MAVLINK_VERSION: ClassVar[int] = {}", doc.version));
            w.blank();
            w.line("@property");
            w.line("def version(self) -> int:");
            w.indent();
            w.line("return self.MAVLINK_VERSION");
            w.dedent();
            w.blank();
            w.line("def parse(self, message_id: int, data: bytes) -> MavlinkMessage | None:");
            w.indent();
            w.line("match message_id:");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_class_name(&msg.name);
                w.line(&format!("case {class_name}.MSG_ID:"));
                w.indent();
                w.line(&format!("return {class_name}.parse(data)"));
                w.dedent();
            }
            w.line("case _:");
            w.indent();
            w.line("return None");
            w.dedent();
            w.dedent();
            w.dedent();
            w.blank();
            w.line("def crc_extra(self, message_id: int) -> int:");
            w.indent();
            w.line("match message_id:");
            w.indent();
            for msg in doc.messages.messages() {
                let class_name = message_class_name(&msg.name);
                w.line(&format!("case {class_name}.MSG_ID:"));
                w.indent();
                w.line(&format!("return {class_name}.CRC_EXTRA"));
                w.dedent();
            }
            w.line("case _:");
            w.indent();
            w.line("return -1");
            w.dedent();
            w.dedent();
            w.dedent();
            Ok(())
        },
    )
}

pub fn as_python_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[&str] = &[
        "int8_t", "uint8_t", "int16_t", "uint16_t", "int32_t", "uint32_t", "int64_t", "uint64_t",
        "char", "float", "double",
    ];

    for basic_type in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_class_name(enum_name);
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
                return format!("list[{}]", enum_class_name(enum_name));
            }
            return format!("list[{basic_type}]");
        }
    }

    format!("object  # Unknown({mavlink_type})")
}

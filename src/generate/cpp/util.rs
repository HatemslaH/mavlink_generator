use std::collections::HashSet;
use std::path::Path;

use crate::xml::DialectEntry;

pub fn dialect_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    crate::xml::capitalize(stem)
}

pub fn message_struct_name(message_name: &str) -> String {
    format!("{}_t", message_name.to_lowercase())
}

pub fn message_type_name(message_name: &str) -> String {
    message_name.to_lowercase()
}

pub fn message_prefix(message_name: &str) -> String {
    message_name.to_lowercase()
}

pub fn dialect_struct_name(dialect_name: &str) -> String {
    format!("mavlink_dialect_{}_t", dialect_name.to_lowercase())
}

pub fn unique_enum_entry_cpp_name(
    entry: &DialectEntry,
    used_names: &mut HashSet<String>,
) -> String {
    let base = entry.name.clone();
    if used_names.insert(base.clone()) {
        return base;
    }

    let with_value = format!("{}_V{}", entry.name, entry.value);
    if used_names.insert(with_value.clone()) {
        return with_value;
    }

    let mut n = 2;
    loop {
        let candidate = format!("{with_value}_{n}");
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        n += 1;
    }
}

pub fn as_cpp_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    as_cpp_base_type(mavlink_type, enum_name, is_bitmask)
}

pub fn as_cpp_base_type(mavlink_type: &str, enum_name: Option<&str>, is_bitmask: bool) -> String {
    const BASIC_TYPES: &[&str] = &[
        "int8_t", "uint8_t", "int16_t", "uint16_t", "int32_t", "uint32_t", "int64_t", "uint64_t",
        "char", "float", "double",
    ];

    for basic_type in BASIC_TYPES {
        if *basic_type == mavlink_type {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_name.to_string();
            }
            return (*basic_type).to_string();
        }

        let prefix = format!("{basic_type}[");
        if mavlink_type.starts_with(&prefix) {
            if let Some(enum_name) = enum_name.filter(|_| !is_bitmask) {
                return enum_name.to_string();
            }
            return (*basic_type).to_string();
        }
    }

    format!("/* Unknown({mavlink_type}) */ uint8_t")
}

use std::collections::HashSet;
use std::path::Path;

use crate::xml::{DialectEntry, camel_case, capitalize};

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type",
    "unsafe", "use", "where", "while",
];

pub fn dialect_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    capitalize(stem)
}

pub fn dialect_struct_name(dialect_name: &str) -> String {
    format!("MavlinkDialect{}", camel_case(dialect_name))
}

pub fn unique_enum_entry_rust_name(
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

pub fn enum_type_name(enum_name: &str) -> String {
    camel_case(enum_name)
}

pub fn message_struct_name(message_name: &str) -> String {
    camel_case(message_name)
}

pub fn rust_field_name(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

pub fn rust_field_access(name: &str) -> String {
    format!("self.{}", rust_field_name(name))
}

pub fn parse_local_name(name: &str) -> String {
    if RUST_KEYWORDS.contains(&name) {
        format!("{name}_parsed")
    } else {
        name.to_string()
    }
}

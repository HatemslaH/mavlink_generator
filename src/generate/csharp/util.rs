use std::collections::HashSet;
use std::path::Path;

use crate::xml::{DialectEntry, camel_case, capitalize};

pub fn dialect_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    capitalize(stem)
}

pub fn unique_enum_entry_csharp_name(
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

pub fn enum_class_name(enum_name: &str) -> String {
    camel_case(enum_name)
}

pub fn message_class_name(message_name: &str) -> String {
    camel_case(message_name)
}

pub fn field_property_name(field_name: &str) -> String {
    escape_csharp_identifier(&camel_case(field_name))
}

pub fn field_property_name_for_message(field_name: &str, message_class: &str) -> String {
    let mut name = field_property_name(field_name);
    if name == message_class {
        let mut chars = name.chars();
        if let Some(first) = chars.next() {
            name = first.to_lowercase().to_string() + chars.as_str();
        }
    }
    name
}

pub fn escape_csharp_identifier(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char", "checked",
        "class", "const", "continue", "decimal", "default", "delegate", "do", "double", "else",
        "enum", "event", "explicit", "extern", "false", "finally", "fixed", "float", "for",
        "foreach", "goto", "if", "implicit", "in", "int", "interface", "internal", "is", "lock",
        "long", "namespace", "new", "null", "object", "operator", "out", "override", "params",
        "private", "protected", "public", "readonly", "ref", "return", "sbyte", "sealed",
        "short", "sizeof", "stackalloc", "static", "string", "struct", "switch", "this", "throw",
        "true", "try", "typeof", "uint", "ulong", "unchecked", "unsafe", "ushort", "using",
        "virtual", "void", "volatile", "while",
    ];

    if KEYWORDS.contains(&name) {
        format!("@{name}")
    } else {
        name.to_string()
    }
}

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

pub fn unique_enum_entry_js_name(entry: &DialectEntry, used_names: &mut HashSet<String>) -> String {
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

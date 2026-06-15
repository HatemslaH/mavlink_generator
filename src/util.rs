use std::collections::HashSet;
use std::path::Path;

use crate::dialect_entry::DialectEntry;

pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str().to_lowercase()),
    }
}

pub fn camel_case(s: &str) -> String {
    s.to_lowercase()
        .split('_')
        .map(capitalize)
        .collect::<Vec<_>>()
        .join("")
}

pub fn lower_camel_case(s: &str) -> String {
    let lowered = s.to_lowercase();
    let parts: Vec<&str> = lowered.split('_').collect();
    if parts.len() == 1 {
        return parts[0].to_string();
    }

    let head = parts[0];
    let tail: String = parts[1..].iter().map(|part| capitalize(part)).collect();
    format!("{head}{tail}")
}

pub fn dialect_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    capitalize(stem)
}

/// MAVLink entry names like `CR_75_M` and `CR_7_5_M` can map to the same Dart
/// identifier; disambiguate so generated enums stay valid.
pub fn unique_enum_entry_dart_name(
    entry: &DialectEntry,
    used_names: &mut HashSet<String>,
) -> String {
    let base = entry.name_for_dart.clone();
    if used_names.insert(base.clone()) {
        return base;
    }

    let with_value = format!("{}_v{}", entry.name_for_dart, entry.value);
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

pub fn generate_as_dart_documentation(text: &str) -> String {
    text.lines()
        .map(|line| format!("/// {}", line.trim_start()))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

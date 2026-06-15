use std::collections::HashSet;
use std::path::Path;

use crate::xml::{DialectEntry, capitalize};

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

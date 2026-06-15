use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct CRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.h", include_str!("../../../templates/c/crc.h")),
    ("types.h", include_str!("../../../templates/c/types.h")),
    (
        "mavlink_version.h",
        include_str!("../../../templates/c/mavlink_version.h"),
    ),
    (
        "mavlink_dialect.h",
        include_str!("../../../templates/c/mavlink_dialect.h"),
    ),
    (
        "mavlink_memory.h",
        include_str!("../../../templates/c/mavlink_memory.h"),
    ),
    (
        "mavlink_message.h",
        include_str!("../../../templates/c/mavlink_message.h"),
    ),
    (
        "mavlink_frame.h",
        include_str!("../../../templates/c/mavlink_frame.h"),
    ),
    (
        "mavlink_parser.h",
        include_str!("../../../templates/c/mavlink_parser.h"),
    ),
];

impl LanguageRuntimeGenerator for CRuntimeGenerator {
    fn static_files(&self) -> Vec<RuntimeFile> {
        STATIC_TEMPLATES
            .iter()
            .map(|(name, content)| RuntimeFile {
                relative_path: PathBuf::from(*name),
                content: (*content).to_string(),
            })
            .collect()
    }

    fn entry_point(&self, dialect_stems: &[String]) -> RuntimeFile {
        RuntimeFile {
            relative_path: PathBuf::from("mavlink.h"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("#ifndef MAVLINK_H".to_string());
    lines.push("#define MAVLINK_H".to_string());
    lines.push(String::new());
    lines.push("#include \"crc.h\"".to_string());
    lines.push("#include \"types.h\"".to_string());
    lines.push("#include \"mavlink_memory.h\"".to_string());
    lines.push("#include \"mavlink_version.h\"".to_string());
    lines.push("#include \"mavlink_message.h\"".to_string());
    lines.push("#include \"mavlink_dialect.h\"".to_string());
    lines.push("#include \"mavlink_frame.h\"".to_string());
    lines.push("#include \"mavlink_parser.h\"".to_string());
    for stem in dialect_stems {
        lines.push(format!("#include \"dialects/{stem}.h\""));
    }
    lines.push(String::new());
    lines.push("#endif".to_string());
    lines.push(String::new());

    lines.join("\n")
}

use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct JavaScriptRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    (
        "package.json",
        include_str!("../../../templates/js/package.json"),
    ),
    ("crc.js", include_str!("../../../templates/js/crc.js")),
    (
        "mavlink_types.js",
        include_str!("../../../templates/js/mavlink_types.js"),
    ),
    (
        "mavlink_version.js",
        include_str!("../../../templates/js/mavlink_version.js"),
    ),
    (
        "mavlink_dialect.js",
        include_str!("../../../templates/js/mavlink_dialect.js"),
    ),
    (
        "mavlink_message.js",
        include_str!("../../../templates/js/mavlink_message.js"),
    ),
    (
        "mavlink_frame.js",
        include_str!("../../../templates/js/mavlink_frame.js"),
    ),
    (
        "mavlink_parser.js",
        include_str!("../../../templates/js/mavlink_parser.js"),
    ),
];

impl LanguageRuntimeGenerator for JavaScriptRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.js"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("/** MAVLink JavaScript bindings. */".to_string());
    lines.push(String::new());
    lines.push("export { CrcX25 } from './crc.js';".to_string());
    lines.push("export * from './mavlink_types.js';".to_string());
    for stem in dialect_stems {
        lines.push(format!("export * from './dialects/{stem}.js';"));
    }
    lines.push("export { MavlinkDialect } from './mavlink_dialect.js';".to_string());
    lines.push("export { MavlinkFrame } from './mavlink_frame.js';".to_string());
    lines.push("export { MavlinkMessage } from './mavlink_message.js';".to_string());
    lines.push("export { MavlinkParser } from './mavlink_parser.js';".to_string());
    lines.push("export { MavlinkVersion } from './mavlink_version.js';".to_string());
    lines.push(String::new());

    format!("{}\n", lines.join("\n"))
}

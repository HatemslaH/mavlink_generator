use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct TypeScriptRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.ts", include_str!("../../../templates/ts/crc.ts")),
    (
        "mavlink_types.ts",
        include_str!("../../../templates/ts/mavlink_types.ts"),
    ),
    (
        "mavlink_version.ts",
        include_str!("../../../templates/ts/mavlink_version.ts"),
    ),
    (
        "mavlink_dialect.ts",
        include_str!("../../../templates/ts/mavlink_dialect.ts"),
    ),
    (
        "mavlink_message.ts",
        include_str!("../../../templates/ts/mavlink_message.ts"),
    ),
    (
        "mavlink_frame.ts",
        include_str!("../../../templates/ts/mavlink_frame.ts"),
    ),
    (
        "mavlink_parser.ts",
        include_str!("../../../templates/ts/mavlink_parser.ts"),
    ),
];

impl LanguageRuntimeGenerator for TypeScriptRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.ts"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("/** MAVLink TypeScript bindings. */".to_string());
    lines.push(String::new());
    lines.push("export { CrcX25 } from './crc';".to_string());
    lines.push("export * from './mavlink_types';".to_string());
    for stem in dialect_stems {
        lines.push(format!("export * from './dialects/{stem}';"));
    }
    lines.push("export type { MavlinkDialect } from './mavlink_dialect';".to_string());
    lines.push("export { MavlinkFrame } from './mavlink_frame';".to_string());
    lines.push("export { MavlinkMessage } from './mavlink_message';".to_string());
    lines.push("export { MavlinkParser } from './mavlink_parser';".to_string());
    lines.push("export { MavlinkVersion } from './mavlink_version';".to_string());
    lines.push(String::new());

    format!("{}\n", lines.join("\n"))
}

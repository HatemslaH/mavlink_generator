use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct DartRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.dart", include_str!("../../../templates/dart/crc.dart")),
    (
        "types.dart",
        include_str!("../../../templates/dart/types.dart"),
    ),
    (
        "mavlink_version.dart",
        include_str!("../../../templates/dart/mavlink_version.dart"),
    ),
    (
        "mavlink_dialect.dart",
        include_str!("../../../templates/dart/mavlink_dialect.dart"),
    ),
    (
        "mavlink_message.dart",
        include_str!("../../../templates/dart/mavlink_message.dart"),
    ),
    (
        "mavlink_frame.dart",
        include_str!("../../../templates/dart/mavlink_frame.dart"),
    ),
    (
        "mavlink_parser.dart",
        include_str!("../../../templates/dart/mavlink_parser.dart"),
    ),
    (
        "mavlink_protocols.dart",
        include_str!("../../../templates/dart/mavlink_protocols.dart"),
    ),
    (
        "protocols/mavlink_link.dart",
        include_str!("../../../templates/dart/protocols/mavlink_link.dart"),
    ),
    (
        "protocols/mavlink_session.dart",
        include_str!("../../../templates/dart/protocols/mavlink_session.dart"),
    ),
    (
        "protocols/param_codec.dart",
        include_str!("../../../templates/dart/protocols/param_codec.dart"),
    ),
    (
        "protocols/mission_protocol.dart",
        include_str!("../../../templates/dart/protocols/mission_protocol.dart"),
    ),
    (
        "protocols/parameter_protocol.dart",
        include_str!("../../../templates/dart/protocols/parameter_protocol.dart"),
    ),
    (
        "protocols/command_protocol.dart",
        include_str!("../../../templates/dart/protocols/command_protocol.dart"),
    ),
    (
        "protocols/heartbeat_protocol.dart",
        include_str!("../../../templates/dart/protocols/heartbeat_protocol.dart"),
    ),
    (
        "protocols/protocols.dart",
        include_str!("../../../templates/dart/protocols/protocols.dart"),
    ),
];

impl LanguageRuntimeGenerator for DartRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.dart"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("export 'crc.dart';".to_string());
    for stem in dialect_stems {
        lines.push(format!("export 'dialects/{stem}.dart';"));
    }
    lines.push("export 'mavlink_dialect.dart';".to_string());
    lines.push("export 'mavlink_frame.dart';".to_string());
    lines.push("export 'mavlink_message.dart';".to_string());
    lines.push("export 'mavlink_parser.dart';".to_string());
    lines.push("export 'mavlink_version.dart';".to_string());
    lines.push("export 'types.dart';".to_string());

    format!("{}\n", lines.join("\n"))
}

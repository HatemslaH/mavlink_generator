use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct PythonRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.py", include_str!("../../../templates/py/crc.py")),
    (
        "mavlink_types.py",
        include_str!("../../../templates/py/mavlink_types.py"),
    ),
    (
        "mavlink_version.py",
        include_str!("../../../templates/py/mavlink_version.py"),
    ),
    (
        "mavlink_dialect.py",
        include_str!("../../../templates/py/mavlink_dialect.py"),
    ),
    (
        "mavlink_message.py",
        include_str!("../../../templates/py/mavlink_message.py"),
    ),
    (
        "mavlink_frame.py",
        include_str!("../../../templates/py/mavlink_frame.py"),
    ),
    (
        "mavlink_parser.py",
        include_str!("../../../templates/py/mavlink_parser.py"),
    ),
];

impl LanguageRuntimeGenerator for PythonRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.py"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("\"\"\"MAVLink Python bindings.\"\"\"".to_string());
    lines.push(String::new());
    lines.push("from crc import CrcX25".to_string());
    lines.push("from mavlink_types import *".to_string());
    for stem in dialect_stems {
        lines.push(format!("from dialects.{stem} import *"));
    }
    lines.push("from mavlink_dialect import MavlinkDialect".to_string());
    lines.push("from mavlink_frame import MavlinkFrame".to_string());
    lines.push("from mavlink_message import MavlinkMessage".to_string());
    lines.push("from mavlink_parser import MavlinkParser".to_string());
    lines.push("from mavlink_version import MavlinkVersion".to_string());
    lines.push(String::new());

    format!("{}\n", lines.join("\n"))
}

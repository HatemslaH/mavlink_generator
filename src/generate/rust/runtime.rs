use std::path::PathBuf;

use crate::generate::examples::ALL_EXAMPLE_SUFFIXES;
use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct RustRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.rs", include_str!("../../../templates/rust/crc.rs")),
    (
        "mavlink_types.rs",
        include_str!("../../../templates/rust/mavlink_types.rs"),
    ),
    (
        "mavlink_version.rs",
        include_str!("../../../templates/rust/mavlink_version.rs"),
    ),
    (
        "mavlink_dialect.rs",
        include_str!("../../../templates/rust/mavlink_dialect.rs"),
    ),
    (
        "mavlink_message.rs",
        include_str!("../../../templates/rust/mavlink_message.rs"),
    ),
    (
        "mavlink_frame.rs",
        include_str!("../../../templates/rust/mavlink_frame.rs"),
    ),
    (
        "mavlink_parser.rs",
        include_str!("../../../templates/rust/mavlink_parser.rs"),
    ),
    (
        "mavlink_protocols.rs",
        include_str!("../../../templates/rust/mavlink_protocols.rs"),
    ),
    (
        "protocols/mod.rs",
        include_str!("../../../templates/rust/protocols/mod.rs"),
    ),
    (
        "protocols/mavlink_link.rs",
        include_str!("../../../templates/rust/protocols/mavlink_link.rs"),
    ),
    (
        "protocols/mavlink_session.rs",
        include_str!("../../../templates/rust/protocols/mavlink_session.rs"),
    ),
    (
        "protocols/mavlink_cancellation.rs",
        include_str!("../../../templates/rust/protocols/mavlink_cancellation.rs"),
    ),
    (
        "protocols/mavlink_vehicle_client.rs",
        include_str!("../../../templates/rust/protocols/mavlink_vehicle_client.rs"),
    ),
    (
        "protocols/param_codec.rs",
        include_str!("../../../templates/rust/protocols/param_codec.rs"),
    ),
    (
        "protocols/mission_protocol.rs",
        include_str!("../../../templates/rust/protocols/mission_protocol.rs"),
    ),
    (
        "protocols/parameter_protocol.rs",
        include_str!("../../../templates/rust/protocols/parameter_protocol.rs"),
    ),
    (
        "protocols/command_protocol.rs",
        include_str!("../../../templates/rust/protocols/command_protocol.rs"),
    ),
    (
        "protocols/heartbeat_protocol.rs",
        include_str!("../../../templates/rust/protocols/heartbeat_protocol.rs"),
    ),
];

const EXAMPLE_SUFFIXES: &[&str] = ALL_EXAMPLE_SUFFIXES;

impl LanguageRuntimeGenerator for RustRuntimeGenerator {
    fn static_files(&self) -> Vec<RuntimeFile> {
        STATIC_TEMPLATES
            .iter()
            .map(|(name, content)| RuntimeFile {
                relative_path: PathBuf::from(name),
                content: (*content).to_string(),
            })
            .collect()
    }

    fn entry_point(&self, dialect_stems: &[String]) -> RuntimeFile {
        RuntimeFile {
            relative_path: PathBuf::from("lib.rs"),
            content: render_lib_rs(dialect_stems),
        }
    }
}

pub fn render_dialects_mod(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();
    for stem in dialect_stems {
        lines.push(format!("pub mod {stem};"));
        lines.push(format!("pub use {stem}::*;"));
    }
    format!("{}\n", lines.join("\n"))
}

fn render_lib_rs(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push("//! MAVLink Rust bindings.".to_string());
    lines.push(String::new());
    lines.push("pub mod crc;".to_string());
    lines.push("pub mod dialects;".to_string());
    lines.push("pub mod mavlink_dialect;".to_string());
    lines.push("pub mod mavlink_frame;".to_string());
    lines.push("pub mod mavlink_message;".to_string());
    lines.push("pub mod mavlink_parser;".to_string());
    lines.push("pub mod mavlink_protocols;".to_string());
    lines.push("pub mod mavlink_types;".to_string());
    lines.push("pub mod mavlink_version;".to_string());
    lines.push("pub mod protocols;".to_string());
    lines.push(String::new());
    lines.push("pub use crc::CrcX25;".to_string());
    for stem in dialect_stems {
        lines.push(format!("pub use dialects::{stem}::*;"));
    }
    lines.push("pub use mavlink_dialect::MavlinkDialect;".to_string());
    lines.push("pub use mavlink_frame::MavlinkFrame;".to_string());
    lines.push("pub use mavlink_message::MavlinkMessage;".to_string());
    lines.push("pub use mavlink_parser::MavlinkParser;".to_string());
    lines.push("pub use mavlink_version::MavlinkVersion;".to_string());
    lines.push("pub use mavlink_types::*;".to_string());
    lines.push(String::new());
    format!("{}\n", lines.join("\n"))
}

pub fn render_cargo_toml(dialect_stems: &[String]) -> String {
    let mut lines = vec![
        "[package]".to_string(),
        "name = \"mavlink\"".to_string(),
        "version = \"0.1.0\"".to_string(),
        "edition = \"2021\"".to_string(),
        String::new(),
        "[lib]".to_string(),
        "path = \"lib.rs\"".to_string(),
        String::new(),
        "[dependencies]".to_string(),
        "tokio = { version = \"1\", features = [\"rt\", \"sync\", \"time\", \"macros\"] }"
            .to_string(),
        "tokio-stream = { version = \"0.1\", features = [\"sync\"] }".to_string(),
        "futures = \"0.3\"".to_string(),
        "async-trait = \"0.1\"".to_string(),
        String::new(),
        "[dev-dependencies]".to_string(),
        "tokio = { version = \"1\", features = [\"rt-multi-thread\", \"macros\"] }".to_string(),
        String::new(),
    ];

    for stem in dialect_stems {
        for suffix in EXAMPLE_SUFFIXES {
            lines.push("[[example]]".to_string());
            lines.push(format!("name = \"{stem}_{suffix}\""));
            lines.push(format!("path = \"examples/{stem}_{suffix}.rs\""));
            lines.push(String::new());
        }
    }

    format!("{}\n", lines.join("\n"))
}

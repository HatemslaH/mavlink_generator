use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct CppRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.hpp", include_str!("../../../templates/cpp/crc.hpp")),
    (
        "types.hpp",
        include_str!("../../../templates/cpp/types.hpp"),
    ),
    (
        "mavlink_version.hpp",
        include_str!("../../../templates/cpp/mavlink_version.hpp"),
    ),
    (
        "mavlink_dialect.hpp",
        include_str!("../../../templates/cpp/mavlink_dialect.hpp"),
    ),
    (
        "mavlink_memory.hpp",
        include_str!("../../../templates/cpp/mavlink_memory.hpp"),
    ),
    (
        "mavlink_message.hpp",
        include_str!("../../../templates/cpp/mavlink_message.hpp"),
    ),
    (
        "mavlink_frame.hpp",
        include_str!("../../../templates/cpp/mavlink_frame.hpp"),
    ),
    (
        "mavlink_parser.hpp",
        include_str!("../../../templates/cpp/mavlink_parser.hpp"),
    ),
    (
        "mavlink_protocols.hpp",
        include_str!("../../../templates/cpp/mavlink_protocols.hpp"),
    ),
    (
        "protocols/mavlink_link.hpp",
        include_str!("../../../templates/cpp/protocols/mavlink_link.hpp"),
    ),
    (
        "protocols/mavlink_link.cpp",
        include_str!("../../../templates/cpp/protocols/mavlink_link.cpp"),
    ),
    (
        "protocols/mavlink_cancellation.hpp",
        include_str!("../../../templates/cpp/protocols/mavlink_cancellation.hpp"),
    ),
    (
        "protocols/mavlink_cancellation.cpp",
        include_str!("../../../templates/cpp/protocols/mavlink_cancellation.cpp"),
    ),
    (
        "protocols/mavlink_session.hpp",
        include_str!("../../../templates/cpp/protocols/mavlink_session.hpp"),
    ),
    (
        "protocols/mavlink_session.cpp",
        include_str!("../../../templates/cpp/protocols/mavlink_session.cpp"),
    ),
    (
        "protocols/param_codec.hpp",
        include_str!("../../../templates/cpp/protocols/param_codec.hpp"),
    ),
    (
        "protocols/param_codec.cpp",
        include_str!("../../../templates/cpp/protocols/param_codec.cpp"),
    ),
    (
        "protocols/heartbeat_protocol.hpp",
        include_str!("../../../templates/cpp/protocols/heartbeat_protocol.hpp"),
    ),
    (
        "protocols/heartbeat_protocol.cpp",
        include_str!("../../../templates/cpp/protocols/heartbeat_protocol.cpp"),
    ),
    (
        "protocols/parameter_protocol.hpp",
        include_str!("../../../templates/cpp/protocols/parameter_protocol.hpp"),
    ),
    (
        "protocols/parameter_protocol.cpp",
        include_str!("../../../templates/cpp/protocols/parameter_protocol.cpp"),
    ),
    (
        "protocols/mission_protocol.hpp",
        include_str!("../../../templates/cpp/protocols/mission_protocol.hpp"),
    ),
    (
        "protocols/mission_protocol.cpp",
        include_str!("../../../templates/cpp/protocols/mission_protocol.cpp"),
    ),
    (
        "protocols/command_protocol.hpp",
        include_str!("../../../templates/cpp/protocols/command_protocol.hpp"),
    ),
    (
        "protocols/command_protocol.cpp",
        include_str!("../../../templates/cpp/protocols/command_protocol.cpp"),
    ),
    (
        "protocols/mavlink_vehicle_client.hpp",
        include_str!("../../../templates/cpp/protocols/mavlink_vehicle_client.hpp"),
    ),
    (
        "protocols/mavlink_vehicle_client.cpp",
        include_str!("../../../templates/cpp/protocols/mavlink_vehicle_client.cpp"),
    ),
    (
        "protocols/protocols.hpp",
        include_str!("../../../templates/cpp/protocols/protocols.hpp"),
    ),
];

impl LanguageRuntimeGenerator for CppRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.hpp"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("#pragma once".to_string());
    lines.push(String::new());
    lines.push("#include \"crc.hpp\"".to_string());
    lines.push("#include \"types.hpp\"".to_string());
    lines.push("#include \"mavlink_memory.hpp\"".to_string());
    lines.push("#include \"mavlink_version.hpp\"".to_string());
    lines.push("#include \"mavlink_message.hpp\"".to_string());
    lines.push("#include \"mavlink_dialect.hpp\"".to_string());
    lines.push("#include \"mavlink_frame.hpp\"".to_string());
    lines.push("#include \"mavlink_parser.hpp\"".to_string());
    for stem in dialect_stems {
        lines.push(format!("#include \"dialects/{stem}.hpp\""));
    }
    lines.push(String::new());

    lines.join("\n")
}

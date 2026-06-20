use std::path::PathBuf;

use crate::generate::runtime::{LanguageRuntimeGenerator, RuntimeFile};

pub struct CSharpRuntimeGenerator;

const STATIC_TEMPLATES: &[(&str, &str)] = &[
    ("crc.cs", include_str!("../../../templates/csharp/crc.cs")),
    (
        "mavlink_types.cs",
        include_str!("../../../templates/csharp/mavlink_types.cs"),
    ),
    (
        "mavlink_version.cs",
        include_str!("../../../templates/csharp/mavlink_version.cs"),
    ),
    (
        "mavlink_dialect.cs",
        include_str!("../../../templates/csharp/mavlink_dialect.cs"),
    ),
    (
        "mavlink_message.cs",
        include_str!("../../../templates/csharp/mavlink_message.cs"),
    ),
    (
        "mavlink_frame.cs",
        include_str!("../../../templates/csharp/mavlink_frame.cs"),
    ),
    (
        "mavlink_parser.cs",
        include_str!("../../../templates/csharp/mavlink_parser.cs"),
    ),
    (
        "mavlink_protocols.cs",
        include_str!("../../../templates/csharp/mavlink_protocols.cs"),
    ),
    (
        "protocols/mavlink_link.cs",
        include_str!("../../../templates/csharp/protocols/mavlink_link.cs"),
    ),
    (
        "protocols/mavlink_session.cs",
        include_str!("../../../templates/csharp/protocols/mavlink_session.cs"),
    ),
    (
        "protocols/mavlink_cancellation.cs",
        include_str!("../../../templates/csharp/protocols/mavlink_cancellation.cs"),
    ),
    (
        "protocols/mavlink_vehicle_client.cs",
        include_str!("../../../templates/csharp/protocols/mavlink_vehicle_client.cs"),
    ),
    (
        "protocols/param_codec.cs",
        include_str!("../../../templates/csharp/protocols/param_codec.cs"),
    ),
    (
        "protocols/mission_protocol.cs",
        include_str!("../../../templates/csharp/protocols/mission_protocol.cs"),
    ),
    (
        "protocols/parameter_protocol.cs",
        include_str!("../../../templates/csharp/protocols/parameter_protocol.cs"),
    ),
    (
        "protocols/command_protocol.cs",
        include_str!("../../../templates/csharp/protocols/command_protocol.cs"),
    ),
    (
        "protocols/heartbeat_protocol.cs",
        include_str!("../../../templates/csharp/protocols/heartbeat_protocol.cs"),
    ),
    (
        "protocols/protocols.cs",
        include_str!("../../../templates/csharp/protocols/protocols.cs"),
    ),
];

impl LanguageRuntimeGenerator for CSharpRuntimeGenerator {
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
            relative_path: PathBuf::from("mavlink.cs"),
            content: render_mavlink_entry_point(dialect_stems),
        }
    }
}

fn render_mavlink_entry_point(dialect_stems: &[String]) -> String {
    let mut lines = Vec::new();

    lines.push("/// <summary>MAVLink C# bindings.</summary>".to_string());
    lines.push("/// <remarks>".to_string());
    lines.push("/// Runtime types live in the <c>Mavlink</c> namespace.".to_string());
    lines.push(
        "/// Dialect-specific messages and enums live in <c>Mavlink.Dialects</c>.".to_string(),
    );
    if dialect_stems.is_empty() {
        lines.push("/// </remarks>".to_string());
    } else {
        lines.push("/// Generated dialects:".to_string());
        for stem in dialect_stems {
            lines.push(format!("/// - {stem} (dialects/{stem}.cs)"));
        }
        lines.push("/// </remarks>".to_string());
    }
    lines.push(String::new());
    lines.push("namespace Mavlink;".to_string());
    lines.push(String::new());
    lines.push(
        "/// <summary>Entry marker for the generated MAVLink C# library.</summary>".to_string(),
    );
    lines.push("public static class MavlinkBindings".to_string());
    lines.push("{".to_string());
    lines.push("}".to_string());
    lines.push(String::new());

    format!("{}\n", lines.join("\n"))
}

pub fn render_mavlink_csproj() -> String {
    r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <RootNamespace>Mavlink</RootNamespace>
  </PropertyGroup>
  <ItemGroup>
    <Compile Remove="examples/**" />
  </ItemGroup>
</Project>
"#
    .to_string()
}

pub fn render_example_csproj(stem: &str, suffix: &str) -> String {
    let source = format!("{stem}_{suffix}.cs");
    let static_source = if suffix.starts_with("protocol_") {
        "protocols_common.cs"
    } else {
        "common.cs"
    };
    format!(
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <OutputType>Exe</OutputType>
    <TargetFramework>net8.0</TargetFramework>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <EnableDefaultCompileItems>false</EnableDefaultCompileItems>
  </PropertyGroup>
  <ItemGroup>
    <ProjectReference Include="..\Mavlink.csproj" />
  </ItemGroup>
  <ItemGroup>
    <Compile Include="{static_source}" />
    <Compile Include="{source}" />
  </ItemGroup>
</Project>
"#
    )
}

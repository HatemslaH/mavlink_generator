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
    <Compile Include="common.cs" />
    <Compile Include="{source}" />
  </ItemGroup>
</Project>
"#
    )
}

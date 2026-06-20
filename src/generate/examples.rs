use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::generate::TargetLanguage;

/// A generated example file for a target language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExampleFile {
    pub relative_path: PathBuf,
    pub content: String,
}

/// Extension point for language-specific usage examples.
pub trait LanguageExampleGenerator {
    fn static_files(&self) -> Vec<ExampleFile>;
    fn generated_files(&self, dialect_stems: &[String]) -> Vec<ExampleFile>;
}

pub const EXAMPLES_DIR: &str = "examples";

/// Low-level virtual examples (message round-trip, no protocol classes).
pub const LOW_LEVEL_EXAMPLE_SUFFIXES: &[&str] = &[
    "heartbeat",
    "mission_upload",
    "request_telemetry",
    "request_parameters",
];

/// Protocol-class virtual examples (transport-agnostic clients over `VirtualMavlinkBus`).
pub const PROTOCOL_EXAMPLE_SUFFIXES: &[&str] = &[
    "protocol_mission",
    "protocol_parameters",
    "protocol_command",
    "protocol_heartbeat",
    "protocol_vehicle",
    "protocol_subscribe",
];

/// All generated example suffixes in order (low-level, then protocol).
pub const ALL_EXAMPLE_SUFFIXES: &[&str] = &[
    "heartbeat",
    "mission_upload",
    "request_telemetry",
    "request_parameters",
    "protocol_mission",
    "protocol_parameters",
    "protocol_command",
    "protocol_heartbeat",
    "protocol_vehicle",
    "protocol_subscribe",
];

pub fn examples_output_dir(language: TargetLanguage) -> PathBuf {
    crate::generate::runtime::language_output_dir(language).join(EXAMPLES_DIR)
}

pub fn generate_example_files(
    output_dir: impl AsRef<Path>,
    language: TargetLanguage,
    dialect_stems: &[String],
) -> Result<()> {
    let output_dir = output_dir.as_ref().join(EXAMPLES_DIR);
    let generator = example_generator(language)?;

    fs::create_dir_all(&output_dir)?;

    for file in generator
        .static_files()
        .into_iter()
        .chain(generator.generated_files(dialect_stems))
    {
        write_example_file(&output_dir, &file)?;
    }

    Ok(())
}

fn example_generator(language: TargetLanguage) -> Result<Box<dyn LanguageExampleGenerator>> {
    match language {
        TargetLanguage::Dart => Ok(Box::new(crate::generate::dart::DartExampleGenerator)),
        TargetLanguage::C => Ok(Box::new(crate::generate::c::CExampleGenerator)),
        TargetLanguage::Cpp => Ok(Box::new(crate::generate::cpp::CppExampleGenerator)),
        TargetLanguage::Python => Ok(Box::new(crate::generate::python::PythonExampleGenerator)),
        TargetLanguage::JavaScript => Ok(Box::new(
            crate::generate::javascript::JavaScriptExampleGenerator,
        )),
        TargetLanguage::TypeScript => Ok(Box::new(
            crate::generate::typescript::TypeScriptExampleGenerator,
        )),
        TargetLanguage::CSharp => Ok(Box::new(crate::generate::csharp::CSharpExampleGenerator)),
        TargetLanguage::Rust => Ok(Box::new(crate::generate::rust::RustExampleGenerator)),
    }
}

fn write_example_file(output_dir: &Path, file: &ExampleFile) -> Result<()> {
    let path = output_dir.join(&file.relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, &file.content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_example_suffixes_concatenate_low_level_and_protocol() {
        let expected: Vec<&str> = LOW_LEVEL_EXAMPLE_SUFFIXES
            .iter()
            .chain(PROTOCOL_EXAMPLE_SUFFIXES.iter())
            .copied()
            .collect();
        assert_eq!(expected, ALL_EXAMPLE_SUFFIXES);
    }
}

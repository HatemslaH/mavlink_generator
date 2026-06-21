use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::generate::TargetLanguage;

/// A generated support/runtime file (not dialect-specific).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeFile {
    pub relative_path: PathBuf,
    pub content: String,
}

/// Extension point for language-specific runtime/support file generation.
pub trait LanguageRuntimeGenerator {
    fn static_files(&self) -> Vec<RuntimeFile>;
    fn entry_point(&self, dialect_stems: &[String]) -> RuntimeFile;
}

pub const GENERATED_ROOT: &str = "generated";
pub const DIALECTS_DIR: &str = "dialects";

pub fn language_output_dir(language: TargetLanguage) -> PathBuf {
    PathBuf::from(GENERATED_ROOT).join(language.output_dir_name())
}

pub fn dialects_output_dir(language: TargetLanguage) -> PathBuf {
    let base = language_output_dir(language);
    match language {
        TargetLanguage::Dart => base.join("lib").join(DIALECTS_DIR),
        _ => base.join(DIALECTS_DIR),
    }
}

pub fn dialect_output_path(language: TargetLanguage, dialect_stem: &str) -> PathBuf {
    dialects_output_dir(language).join(format!("{dialect_stem}.{}", language.file_extension()))
}

pub fn generate_runtime_files(
    output_dir: impl AsRef<Path>,
    language: TargetLanguage,
    dialect_stems: &[String],
) -> Result<()> {
    let output_dir = output_dir.as_ref();
    let generator = runtime_generator(language)?;

    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join(dialects_relative_dir(language)))?;

    if language == TargetLanguage::Python {
        fs::write(output_dir.join(DIALECTS_DIR).join("__init__.py"), "")?;
    }

    for file in generator.static_files() {
        write_runtime_file(output_dir, &file)?;
    }

    let entry_point = generator.entry_point(dialect_stems);
    write_runtime_file(output_dir, &entry_point)?;

    if language == TargetLanguage::Rust {
        let dialects_mod = crate::generate::rust::render_dialects_mod(dialect_stems);
        write_runtime_file(
            output_dir,
            &RuntimeFile {
                relative_path: PathBuf::from("dialects/mod.rs"),
                content: dialects_mod,
            },
        )?;
        let cargo_toml = crate::generate::rust::render_cargo_toml(dialect_stems);
        write_runtime_file(
            output_dir,
            &RuntimeFile {
                relative_path: PathBuf::from("Cargo.toml"),
                content: cargo_toml,
            },
        )?;
    }

    if language == TargetLanguage::CSharp {
        let csproj = crate::generate::csharp::render_mavlink_csproj(dialect_stems);
        write_runtime_file(
            output_dir,
            &RuntimeFile {
                relative_path: PathBuf::from("Mavlink.csproj"),
                content: csproj,
            },
        )?;
    }

    Ok(())
}

pub fn dialects_relative_dir(language: TargetLanguage) -> PathBuf {
    match language {
        TargetLanguage::Dart => PathBuf::from("lib").join(DIALECTS_DIR),
        _ => PathBuf::from(DIALECTS_DIR),
    }
}

fn runtime_generator(language: TargetLanguage) -> Result<Box<dyn LanguageRuntimeGenerator>> {
    match language {
        TargetLanguage::Dart => Ok(Box::new(crate::generate::dart::DartRuntimeGenerator)),
        TargetLanguage::C => Ok(Box::new(crate::generate::c::CRuntimeGenerator)),
        TargetLanguage::Cpp => Ok(Box::new(crate::generate::cpp::CppRuntimeGenerator)),
        TargetLanguage::Python => Ok(Box::new(crate::generate::python::PythonRuntimeGenerator)),
        TargetLanguage::JavaScript => Ok(Box::new(
            crate::generate::javascript::JavaScriptRuntimeGenerator,
        )),
        TargetLanguage::TypeScript => Ok(Box::new(
            crate::generate::typescript::TypeScriptRuntimeGenerator,
        )),
        TargetLanguage::CSharp => Ok(Box::new(crate::generate::csharp::CSharpRuntimeGenerator)),
        TargetLanguage::Rust => Ok(Box::new(crate::generate::rust::RustRuntimeGenerator)),
    }
}

fn write_runtime_file(output_dir: &Path, file: &RuntimeFile) -> Result<()> {
    let path = output_dir.join(&file.relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, &file.content)?;
    Ok(())
}

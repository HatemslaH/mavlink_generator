pub mod c;
pub mod cpp;
pub mod csharp;
pub mod dart;
pub mod examples;
pub mod javascript;
pub mod python;
pub mod runtime;
pub mod rust;
pub mod typescript;

use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::xml::DialectDocument;

pub use examples::{EXAMPLES_DIR, ExampleFile, examples_output_dir, generate_example_files};
pub use runtime::{
    DIALECTS_DIR, GENERATED_ROOT, RuntimeFile, dialect_output_path, dialects_output_dir,
    generate_runtime_files, language_output_dir,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Dart,
    Python,
    C,
    Cpp,
    JavaScript,
    TypeScript,
    CSharp,
    Rust,
}

impl TargetLanguage {
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Dart => "dart",
            Self::Python => "py",
            Self::C => "h",
            Self::Cpp => "hpp",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::CSharp => "cs",
            Self::Rust => "rs",
        }
    }

    pub fn output_dir_name(self) -> &'static str {
        match self {
            Self::Dart => "dart",
            Self::Python => "py",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::JavaScript => "js",
            Self::TypeScript => "ts",
            Self::CSharp => "csharp",
            Self::Rust => "rust",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Dart => "Dart",
            Self::Python => "Python",
            Self::C => "C",
            Self::Cpp => "C++",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::CSharp => "C#",
            Self::Rust => "Rust",
        }
    }
}

/// Extension point for adding new target languages.
pub trait LanguageGenerator {
    fn render(&self, doc: &DialectDocument, src_dialect_path: &Path) -> Result<String>;
}

pub fn generate_code(
    dst_path: impl AsRef<Path>,
    src_dialect_path: impl AsRef<Path>,
    language: TargetLanguage,
) -> Result<()> {
    let dst_path = dst_path.as_ref();
    let src_dialect_path = src_dialect_path.as_ref();
    let doc = DialectDocument::parse(src_dialect_path)?;
    let content = render_dialect(&doc, src_dialect_path, language)?;

    if let Some(parent) = dst_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(dst_path, content)?;
    Ok(())
}

pub fn generate_dialect(
    src_dialect_path: impl AsRef<Path>,
    language: TargetLanguage,
    dialect_stem: &str,
) -> Result<()> {
    let dst_path = dialect_output_path(language, dialect_stem);
    generate_code(&dst_path, src_dialect_path, language)
}

fn render_dialect(
    doc: &DialectDocument,
    src_dialect_path: &Path,
    language: TargetLanguage,
) -> Result<String> {
    match language {
        TargetLanguage::Dart => dart::render(doc, src_dialect_path),
        TargetLanguage::Python => python::render(doc, src_dialect_path),
        TargetLanguage::C => c::render(doc, src_dialect_path),
        TargetLanguage::Cpp => cpp::render(doc, src_dialect_path),
        TargetLanguage::JavaScript => javascript::render(doc, src_dialect_path),
        TargetLanguage::TypeScript => typescript::render(doc, src_dialect_path),
        TargetLanguage::CSharp => csharp::render(doc, src_dialect_path),
        TargetLanguage::Rust => rust::render(doc, src_dialect_path),
    }
}

pub fn generate_dart_code(
    dst_path: impl AsRef<Path>,
    src_dialect_path: impl AsRef<Path>,
) -> Result<()> {
    generate_code(dst_path, src_dialect_path, TargetLanguage::Dart)
}

pub mod c;
pub mod dart;
pub mod python;

use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::xml::DialectDocument;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Dart,
    Python,
    C,
}

impl TargetLanguage {
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Dart => "dart",
            Self::Python => "py",
            Self::C => "h",
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

fn render_dialect(
    doc: &DialectDocument,
    src_dialect_path: &Path,
    language: TargetLanguage,
) -> Result<String> {
    match language {
        TargetLanguage::Dart => dart::render(doc, src_dialect_path),
        TargetLanguage::Python => python::render(doc, src_dialect_path),
        TargetLanguage::C => c::render(doc, src_dialect_path),
    }
}

pub fn generate_dart_code(
    dst_path: impl AsRef<Path>,
    src_dialect_path: impl AsRef<Path>,
) -> Result<()> {
    generate_code(dst_path, src_dialect_path, TargetLanguage::Dart)
}

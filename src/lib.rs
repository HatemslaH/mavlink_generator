pub mod error;
pub mod generate;
pub mod xml;

pub use error::{GeneratorError, Result};
pub use generate::{
    TargetLanguage, dialect_output_path, dialects_output_dir, generate_code, generate_dart_code,
    generate_dialect, generate_runtime_files, language_output_dir,
};
pub use xml::DialectDocument;

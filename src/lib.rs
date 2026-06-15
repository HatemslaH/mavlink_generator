pub mod driver;
pub mod error;
pub mod generate;
pub mod xml;

pub use driver::{
    DEFAULT_DEFINITIONS_DIR, DEFAULT_DIALECT_FILTER, DEFAULT_OUTPUT_ROOT, GenerateOptions,
    GenerateProgress, LanguageInfo, ValidateResult, list_languages, resolve_inputs, run_generate,
    validate_dialects,
};
pub use error::{GeneratorError, Result};
pub use generate::{
    TargetLanguage, dialect_output_path, dialects_output_dir, examples_output_dir, generate_code,
    generate_dart_code, generate_dialect, generate_example_files, generate_runtime_files,
    language_output_dir,
};
pub use xml::DialectDocument;

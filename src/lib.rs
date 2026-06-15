pub mod error;
pub mod generate;
pub mod xml;

pub use error::{GeneratorError, Result};
pub use generate::{TargetLanguage, generate_code, generate_dart_code};
pub use xml::DialectDocument;

use std::path::Path;

use crate::error::{GeneratorError, Result};
use crate::xml::DialectDocument;

pub fn render(_doc: &DialectDocument, _src_dialect_path: &Path) -> Result<String> {
    Err(GeneratorError::Format(
        "Python code generation is not implemented yet".into(),
    ))
}

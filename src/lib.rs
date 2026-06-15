pub mod crc;
pub mod dart_writer;
pub mod dialect_deprecated;
pub mod dialect_entry;
pub mod dialect_enum;
pub mod dialect_field;
pub mod dialect_message;
pub mod dialect_param;
pub mod document;
pub mod error;
pub mod generator;
pub mod mavlink_type;
pub mod util;
pub mod xml_util;

pub use document::DialectDocument;
pub use error::{GeneratorError, Result};
pub use generator::generate_code;

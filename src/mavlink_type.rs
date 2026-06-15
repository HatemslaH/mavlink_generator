use regex::Regex;
use std::sync::LazyLock;

use crate::error::{GeneratorError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BasicType {
    Int,
    Uint,
    Float,
}

#[derive(Debug, Clone)]
pub struct ParsedMavlinkType {
    pub basic_type: BasicType,
    pub bit: u32,
    pub array_length: u32,
    pub mavlink_type: String,
}

impl ParsedMavlinkType {
    pub fn parse(mavlink_type: &str) -> Result<Self> {
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(uint|int|char|float|double)(8|16|32|64|)(_t|_t_mavlink_version|)(\[(\d{1,3})\]|)")
                .expect("valid regex")
        });

        let caps = RE
            .captures(mavlink_type)
            .ok_or_else(|| GeneratorError::Format(format!("Unexpected type, {mavlink_type}")))?;

        let array_length = caps
            .get(5)
            .map(|m| m.as_str().parse::<u32>())
            .transpose()
            .map_err(|_| GeneratorError::Format(format!("Invalid array length in {mavlink_type}")))?
            .unwrap_or(1);

        let (basic_type, bit) = match caps.get(1).map(|m| m.as_str()) {
            Some("int") => (BasicType::Int, caps[2].parse().unwrap_or(8)),
            Some("uint") => (BasicType::Uint, caps[2].parse().unwrap_or(8)),
            Some("char") => (BasicType::Int, 8),
            Some("float") => (BasicType::Float, 32),
            Some("double") => (BasicType::Float, 64),
            Some(other) => {
                return Err(GeneratorError::Format(format!("Unexpected type, {other}")));
            }
            None => {
                return Err(GeneratorError::Format(format!(
                    "Unexpected type, {mavlink_type}"
                )));
            }
        };

        Ok(Self {
            basic_type,
            bit,
            array_length,
            mavlink_type: mavlink_type.to_string(),
        })
    }

    pub fn is_array(&self) -> bool {
        self.array_length > 1
    }

    pub fn byte(&self) -> u32 {
        self.bit / 8
    }

    pub fn unit_type(mavlink_type: &str) -> &str {
        mavlink_type
            .find('[')
            .map_or(mavlink_type, |idx| &mavlink_type[..idx])
    }
}

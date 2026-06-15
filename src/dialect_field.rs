use roxmltree::Node;

use crate::error::{GeneratorError, Result};
use crate::mavlink_type::ParsedMavlinkType;
use crate::util::lower_camel_case;
use crate::xml_util::{attr, element_text};

#[derive(Debug, Clone)]
pub struct DialectField {
    pub name: String,
    pub field_type: String,
    pub description: String,
    pub is_extension: bool,
    pub units: Option<String>,
    pub enum_name: Option<String>,
    pub is_bitmask: bool,
    pub name_for_dart: String,
}

impl DialectField {
    pub fn parsed_type(&self) -> Result<ParsedMavlinkType> {
        ParsedMavlinkType::parse(&self.field_type)
    }

    pub fn unit_type(&self) -> &str {
        ParsedMavlinkType::unit_type(&self.field_type)
    }

    pub fn parse_element(node: Node<'_, '_>, is_extension: bool) -> Result<Self> {
        let name = attr(node, "name").unwrap_or_default();
        if name.is_empty() {
            return Err(GeneratorError::Format(
                "The name of field element should not be empty.".into(),
            ));
        }

        let mut field_type = attr(node, "type").unwrap_or_default().to_string();
        if field_type.is_empty() {
            return Err(GeneratorError::Format(
                "The type of field element should not be empty.".into(),
            ));
        }
        if field_type == "uint8_t_mavlink_version" {
            field_type = "uint8_t".to_string();
        }

        let description = element_text(node);
        let units = attr(node, "units").map(str::to_string);
        let enum_name = attr(node, "enum").map(str::to_string);
        let is_bitmask = attr(node, "display") == Some("bitmask");

        Ok(Self {
            name_for_dart: lower_camel_case(&name),
            name: name.to_string(),
            field_type,
            description,
            is_extension,
            units,
            enum_name,
            is_bitmask,
        })
    }
}

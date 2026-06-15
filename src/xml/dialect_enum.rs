use roxmltree::Node;

use super::dialect_deprecated::DialectDeprecated;
use super::dialect_entry::DialectEntry;
use super::util::camel_case;
use super::xml_util::{attr, child_element, child_text, descendants_named};
use crate::error::{GeneratorError, Result};

#[derive(Debug, Clone)]
pub struct DialectEnum {
    pub name: String,
    pub description: Option<String>,
    pub deprecated: Option<DialectDeprecated>,
    pub entries: Vec<DialectEntry>,
    pub is_bitmask: bool,
    pub name_for_dart: String,
}

#[derive(Debug, Clone, Default)]
pub struct DialectEnums {
    enums: Vec<DialectEnum>,
}

impl DialectEnums {
    pub fn new(enums: Vec<DialectEnum>) -> Self {
        Self { enums }
    }

    pub fn enums(&self) -> &[DialectEnum] {
        &self.enums
    }

    pub fn add_all(&mut self, other: DialectEnums) {
        for new_enum in other.enums {
            if let Some(existing) = self.find_by_name_mut(&new_enum.name) {
                for new_entry in new_enum.entries {
                    if !existing
                        .entries
                        .iter()
                        .any(|entry| entry.value == new_entry.value)
                    {
                        existing.entries.push(new_entry);
                    }
                }

                if existing.description.is_none() {
                    existing.description = new_enum.description;
                }

                if !existing.is_bitmask && new_enum.is_bitmask {
                    existing.is_bitmask = new_enum.is_bitmask;
                }
            } else {
                self.enums.push(new_enum);
            }
        }
    }

    fn find_by_name_mut(&mut self, name: &str) -> Option<&mut DialectEnum> {
        self.enums.iter_mut().find(|e| e.name == name)
    }

    pub fn parse_element(node: Option<Node<'_, '_>>) -> Result<Self> {
        let Some(node) = node else {
            return Ok(Self::default());
        };

        let mut enums = Vec::new();
        for enum_node in descendants_named(node, "enum") {
            let name = attr(enum_node, "name").unwrap_or_default();
            if name.is_empty() {
                return Err(GeneratorError::Format(
                    "The name of enum element should not be empty.".into(),
                ));
            }

            let description = child_text(enum_node, "description");
            let deprecated =
                DialectDeprecated::parse_element(child_element(enum_node, "deprecated"))?;
            let is_bitmask = attr(enum_node, "bitmask") == Some("true");
            let enum_is_mav_cmd = name == "MAV_CMD";

            let entries = descendants_named(enum_node, "entry")
                .map(|entry_node| DialectEntry::parse_element(entry_node, enum_is_mav_cmd))
                .collect::<Result<Vec<_>>>()?;

            enums.push(DialectEnum {
                name_for_dart: camel_case(&name),
                name: name.to_string(),
                description,
                deprecated,
                entries,
                is_bitmask,
            });
        }

        Ok(Self::new(enums))
    }
}

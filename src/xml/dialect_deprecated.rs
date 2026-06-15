use roxmltree::Node;

use super::xml_util::{attr, element_text};
use crate::error::{GeneratorError, Result};

#[derive(Debug, Clone)]
pub struct DialectDeprecated {
    pub since: String,
    pub replaced_by: String,
    pub text: String,
}

impl DialectDeprecated {
    pub fn parse_element(node: Option<Node<'_, '_>>) -> Result<Option<Self>> {
        let Some(node) = node else {
            return Ok(None);
        };

        let since = attr(node, "since").unwrap_or_default().to_string();
        if since.is_empty() {
            return Err(GeneratorError::Format(
                "The since of deprecated element should not be empty.".into(),
            ));
        }

        Ok(Some(Self {
            since,
            replaced_by: attr(node, "replaced_by").unwrap_or_default().to_string(),
            text: element_text(node),
        }))
    }
}

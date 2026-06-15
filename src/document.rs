use std::fs;
use std::path::Path;

use roxmltree::Node;

use crate::dialect_deprecated::DialectDeprecated;
use crate::dialect_enum::DialectEnums;
use crate::dialect_field::DialectField;
use crate::dialect_message::{DialectMessage, DialectMessages};
use crate::error::{GeneratorError, Result};
use crate::xml_util::{child_element, child_text, descendants_named};

#[derive(Debug, Clone)]
pub struct DialectDocument {
    pub version: i32,
    pub dialect: i32,
    pub enums: DialectEnums,
    pub messages: DialectMessages,
}

impl DialectDocument {
    pub fn parse(dialect_path: impl AsRef<Path>) -> Result<Self> {
        let dialect_path = dialect_path.as_ref();
        if !dialect_path.exists() {
            return Err(GeneratorError::MissingFile(dialect_path.to_path_buf()));
        }

        let xml_str = fs::read_to_string(dialect_path)?;
        let document = roxmltree::Document::parse(&xml_str)?;

        let mavlink = document
            .descendants()
            .find(|node| node.has_tag_name("mavlink"))
            .ok_or_else(|| GeneratorError::Format("Missing <mavlink> root element".into()))?;

        let mut version = -1;
        let mut dialect = -1;
        let mut enums = DialectEnums::default();
        let mut messages = DialectMessages::default();

        for include_node in descendants_named(mavlink, "include") {
            let including_path = dialect_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(include_node.text().unwrap_or_default().trim());

            let including_doc = Self::parse(&including_path)?;
            enums.add_all(including_doc.enums);
            messages.add_all(including_doc.messages);

            if including_doc.version != -1 {
                version = including_doc.version;
            }
            if including_doc.dialect != -1 {
                dialect = including_doc.dialect;
            }
        }

        if let Some(value) = child_text(mavlink, "version") {
            version = value
                .parse()
                .map_err(|_| GeneratorError::Format("Invalid <version> value".into()))?;
        }

        if let Some(value) = child_text(mavlink, "dialect") {
            dialect = value
                .parse()
                .map_err(|_| GeneratorError::Format("Invalid <dialect> value".into()))?;
        }

        enums.add_all(DialectEnums::parse_element(child_element(
            mavlink, "enums",
        ))?);
        messages.add_all(Self::parse_messages(child_element(mavlink, "messages"))?);

        Ok(Self {
            version,
            dialect,
            enums,
            messages,
        })
    }

    fn parse_messages(node: Option<Node<'_, '_>>) -> Result<DialectMessages> {
        let Some(node) = node else {
            return Ok(DialectMessages::default());
        };

        let mut messages = Vec::new();
        for message_node in descendants_named(node, "message") {
            messages.push(parse_message(message_node)?);
        }

        Ok(DialectMessages::new(messages))
    }
}

fn parse_message(node: Node<'_, '_>) -> Result<DialectMessage> {
    let id = attr_or_error(node, "id")?.parse::<i32>().map_err(|_| {
        GeneratorError::Format("The id of message element should not be empty.".into())
    })?;

    let name = attr_or_error(node, "name")?;
    if name.is_empty() {
        return Err(GeneratorError::Format(
            "The name of message element should not be empty.".into(),
        ));
    }

    let description = child_text(node, "description").ok_or_else(|| {
        GeneratorError::Format("The description of message element should not be empty.".into())
    })?;

    let deprecated = DialectDeprecated::parse_element(child_element(node, "deprecated"))?;

    let mut fields = Vec::new();
    let mut is_extension = false;
    for child in node.children().filter(|child| child.is_element()) {
        match child.tag_name().name() {
            "field" => fields.push(DialectField::parse_element(child, is_extension)?),
            "extensions" => is_extension = true,
            _ => {}
        }
    }

    DialectMessage::new(id, name.to_string(), description, fields, deprecated)
}

fn attr_or_error<'a>(node: Node<'a, 'a>, name: &str) -> Result<&'a str> {
    node.attribute(name).ok_or_else(|| {
        GeneratorError::Format(format!(
            "Missing attribute '{name}' on <{}>",
            node.tag_name().name()
        ))
    })
}

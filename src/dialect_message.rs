use crate::crc::CrcX25;
use crate::dialect_deprecated::DialectDeprecated;
use crate::dialect_field::DialectField;
use crate::error::Result;
use crate::util::camel_case;

#[derive(Debug, Clone)]
pub struct DialectMessage {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub fields: Vec<DialectField>,
    pub deprecated: Option<DialectDeprecated>,
    pub name_for_dart: String,
    pub ordered_fields: Vec<DialectField>,
}

impl DialectMessage {
    pub fn new(
        id: i32,
        name: String,
        description: String,
        fields: Vec<DialectField>,
        deprecated: Option<DialectDeprecated>,
    ) -> Result<Self> {
        let mut ordered_fields: Vec<DialectField> = fields
            .iter()
            .filter(|field| !field.is_extension)
            .cloned()
            .collect();

        ordered_fields.sort_by(|left, right| {
            right
                .parsed_type()
                .map(|t| t.bit)
                .unwrap_or(0)
                .cmp(&left.parsed_type().map(|t| t.bit).unwrap_or(0))
        });
        ordered_fields.extend(fields.iter().filter(|field| field.is_extension).cloned());

        Ok(Self {
            name_for_dart: camel_case(&name),
            ordered_fields,
            id,
            name,
            description,
            fields,
            deprecated,
        })
    }

    pub fn calculate_crc_extra(&self) -> Result<u8> {
        let mut crc = CrcX25::new();
        crc.accumulate_str(&format!("{} ", self.name));

        for field in &self.ordered_fields {
            if field.is_extension {
                continue;
            }

            crc.accumulate_str(&format!("{} ", field.unit_type()));
            crc.accumulate_str(&format!("{} ", field.name));

            let parsed = field.parsed_type()?;
            if parsed.is_array() {
                crc.accumulate(parsed.array_length as u8);
            }
        }

        Ok(crc.crc_extra())
    }

    pub fn calculate_encoded_length(&self) -> Result<i32> {
        let mut length = 0;
        for field in &self.fields {
            let parsed = field.parsed_type()?;
            length += (parsed.byte() * parsed.array_length) as i32;
        }
        Ok(length)
    }
}

#[derive(Debug, Clone, Default)]
pub struct DialectMessages {
    messages: Vec<DialectMessage>,
}

impl DialectMessages {
    pub fn new(messages: Vec<DialectMessage>) -> Self {
        Self { messages }
    }

    pub fn messages(&self) -> &[DialectMessage] {
        &self.messages
    }

    pub fn add_all(&mut self, other: DialectMessages) {
        for message in other.messages {
            if !self.has_id(message.id) {
                self.messages.push(message);
            }
        }
    }

    fn has_id(&self, id: i32) -> bool {
        self.messages.iter().any(|message| message.id == id)
    }
}

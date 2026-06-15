use roxmltree::Node;

use crate::dialect_deprecated::DialectDeprecated;
use crate::dialect_param::DialectParam;
use crate::error::{GeneratorError, Result};
use crate::util::lower_camel_case;
use crate::xml_util::{
    attr, cast_as_bool, child_element, child_text, descendants_named, element_text,
};

#[derive(Debug, Clone)]
pub struct DialectEntry {
    pub name: String,
    pub value: i32,
    pub description: Option<String>,
    pub deprecated: Option<DialectDeprecated>,
    pub wip: bool,
    pub has_location: Option<bool>,
    pub is_destination: Option<bool>,
    pub params: Option<Vec<DialectParam>>,
    pub name_for_dart: String,
}

impl DialectEntry {
    pub fn parse_element(node: Node<'_, '_>, enum_is_mav_cmd: bool) -> Result<Self> {
        let name = attr(node, "name").unwrap_or_default();
        if name.is_empty() {
            return Err(GeneratorError::Format(
                "The name of entry element should not be empty.".into(),
            ));
        }

        let value_str = attr(node, "value").unwrap_or_default();
        let value = parse_entry_value(value_str)?;
        let description = child_text(node, "description");
        let deprecated = DialectDeprecated::parse_element(child_element(node, "deprecated"))?;
        let wip = child_element(node, "wip").is_some();

        let attr_has_location = attr(node, "hasLocation");
        let attr_is_destination = attr(node, "isDestination");

        let (has_location, is_destination, params) = if enum_is_mav_cmd {
            let mut params = (1..=7).map(DialectParam::empty).collect::<Vec<_>>();

            for param_node in descendants_named(node, "param") {
                let index = attr(param_node, "index")
                    .ok_or_else(|| GeneratorError::Format("param index is required".into()))?
                    .parse::<usize>()
                    .map_err(|_| GeneratorError::Format("param index must be an integer".into()))?;

                if index == 0 || index > params.len() {
                    return Err(GeneratorError::Format(format!(
                        "param index {index} is out of range"
                    )));
                }

                params[index - 1] = DialectParam::new(
                    index as u32,
                    element_text(param_node),
                    attr(param_node, "label").map(str::to_string),
                    attr(param_node, "units").map(str::to_string),
                    attr(param_node, "enum").map(str::to_string),
                    attr(param_node, "decimalPlaces").map(str::to_string),
                    attr(param_node, "increment").map(str::to_string),
                    attr(param_node, "minValue").map(str::to_string),
                    attr(param_node, "maxValue").map(str::to_string),
                    Some(cast_as_bool(attr(param_node, "reserved"), false)?),
                );
            }

            (
                Some(cast_as_bool(attr_has_location, true)?),
                Some(cast_as_bool(attr_is_destination, true)?),
                Some(params),
            )
        } else {
            if attr_has_location.is_some() || attr_is_destination.is_some() {
                return Err(GeneratorError::Format(
                    "The hasLocation attribute and isDestination must be child of MAV_CMD.".into(),
                ));
            }
            (None, None, None)
        };

        Ok(Self {
            name_for_dart: lower_camel_case(&name),
            name: name.to_string(),
            value,
            description,
            deprecated,
            wip,
            has_location,
            is_destination,
            params,
        })
    }
}

fn parse_entry_value(value_str: &str) -> Result<i32> {
    if let Some(exponent) = value_str.strip_prefix("2**") {
        let exponent: u32 = exponent
            .parse()
            .map_err(|_| GeneratorError::Format(format!("Invalid exponent in {value_str}")))?;
        Ok(1i32 << exponent)
    } else if let Some(bits) = value_str.strip_prefix("0b") {
        i32::from_str_radix(bits, 2)
            .map_err(|_| GeneratorError::Format(format!("Invalid binary value {value_str}")))
    } else if let Some(hex) = value_str
        .strip_prefix("0x")
        .or_else(|| value_str.strip_prefix("0X"))
    {
        i32::from_str_radix(hex, 16)
            .map_err(|_| GeneratorError::Format(format!("Invalid hex value {value_str}")))
    } else {
        value_str
            .parse()
            .map_err(|_| GeneratorError::Format(format!("Invalid integer value {value_str}")))
    }
}

#[cfg(test)]
mod tests {
    use super::parse_entry_value;

    #[test]
    fn parses_power_of_two() {
        assert_eq!(parse_entry_value("2**4").unwrap(), 16);
    }

    #[test]
    fn parses_binary() {
        assert_eq!(parse_entry_value("0b1010").unwrap(), 10);
    }
}

#[derive(Debug, Clone)]
pub struct DialectParam {
    pub index: u32,
    pub description: String,
    pub label: Option<String>,
    pub units: Option<String>,
    pub enum_name: Option<String>,
    pub decimal_places: Option<String>,
    pub increment: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub reserved: Option<bool>,
}

impl DialectParam {
    pub fn empty(index: u32) -> Self {
        Self {
            index,
            description: String::new(),
            label: None,
            units: None,
            enum_name: None,
            decimal_places: None,
            increment: None,
            min_value: None,
            max_value: None,
            reserved: None,
        }
    }

    pub fn new(
        index: u32,
        description: String,
        label: Option<String>,
        units: Option<String>,
        enum_name: Option<String>,
        decimal_places: Option<String>,
        increment: Option<String>,
        min_value: Option<String>,
        max_value: Option<String>,
        reserved: Option<bool>,
    ) -> Self {
        Self {
            index,
            description,
            label,
            units,
            enum_name,
            decimal_places,
            increment,
            min_value,
            max_value,
            reserved,
        }
    }
}

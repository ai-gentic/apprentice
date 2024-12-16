use serde::Serialize;

/// Tool parameter data types.
pub enum ParamType {
    /// String.
    String,
    /// Integer.
    Integer,
    /// Number.
    Number,
    /// Boolean.
    Boolean,
}

impl Serialize for ParamType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer {
        match &self {
            ParamType::String => serializer.serialize_str("string"),
            ParamType::Integer => serializer.serialize_str("integer"),
            ParamType::Number => serializer.serialize_str("number"),
            ParamType::Boolean => serializer.serialize_str("boolean"),
        }
    }
}

/// Tool parameter specification.
pub struct ToolParam {
    /// Parameter name.
    pub name: String,
    /// Parameter description.
    pub description: String,
    /// Parameter data type.
    pub data_type: ParamType,
    /// Value is required.
    pub required: bool,
}

/// Tool specification.
pub struct ToolSpec {
    /// Tool/function name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// Tool parameters.
    pub params: Vec<ToolParam>,
}

/// Tool choice settings.
pub enum ToolChoice {
    /// Do not use tools.
    None,
    /// LLM decide whether to call any of provided tools or not.
    Auto,
    /// LLM must use any one of the provided tools.
    CallOne,
    /// LLM must call specified tool (name).
    Force(String)
}
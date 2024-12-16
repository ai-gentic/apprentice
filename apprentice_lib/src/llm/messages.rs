use std::fmt::Display;
use serde_json::Value;


/// Logical roles (provider-independent).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Role {
    /// System.
    System = 0,
    /// Model.
    Model = 1,
    /// User.
    User = 2,
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let role = match self {
            Role::System => "system",
            Role::Model => "apprentice",
            Role::User => "user",
        };
        f.write_str(role)
    }
}

/// Chat message.
pub enum Message {
    /// Text message.
    Text(Text),
    /// Tool call.
    ToolCall(ToolCall),
    /// Tool call result.
    ToolResult(ToolResult),
}

impl Message {
    /// Create text message.
    pub fn text(role: Role, message: String) -> Self {
        Message::Text(Text {role, message})
    }

    /// Create tool result message.
    pub fn tool_result(call_id: String, name: String, result: String) -> Self {
        Message::ToolResult(ToolResult { call_id, name, result })
    }

    /// Create tool use message.
    #[cfg(test)]
    pub(crate) fn tool_use(call_id: String, name: String, params: Vec<ToolParam>) -> Self {
        Message::ToolCall(ToolCall { call_id, name, params })
    }
}

/// Chat message.
pub struct Text {
    /// Role.
    pub role: Role,
    /// Message content.
    pub message: String,
}

/// Tool call result.
pub struct ToolResult {
    /// Call id.
    pub call_id: String,
    /// Tool name.
    pub name: String,
    /// Call result.
    pub result: String
}

/// Tool call result.
pub struct ToolCall {
    /// Call id.
    pub call_id: String,
    /// Tool name.
    pub name: String,
    /// Call params.
    pub params: Vec<ToolParam>,
}

/// Tool parameters.
pub struct ToolParam {
    /// Parameter name.
    pub name: String,
    /// Value.
    pub value: Value,
}
use serde_json::{json, Number, Value};
use crate::{config::ModelProvider, error::Error, tools::ToolParam};
use super::Role;

/// Get model-specific role for the provider.
pub fn role_to_llm(provider: ModelProvider, role: Role) -> &'static str {
    const ROLES_FOR_OPENAI: [&str; 3] = ["system", "assistant", "user"];
    const ROLES_FOR_ANTHROPIC: [&str; 3] = ["", "assistant", "user"];
    const ROLES_FOR_GCP: [&str; 3] = ["system", "model", "user"];

    match provider {
        ModelProvider::OpenAI => ROLES_FOR_OPENAI[role as usize],
        ModelProvider::Anthropic => ROLES_FOR_ANTHROPIC[role as usize],
        ModelProvider::GCP => ROLES_FOR_GCP[role as usize],
    }
}

/// Get logical role by model role.
pub fn llm_to_role(role: &str) -> Result<Role, Error> {
    match role {
        "system" => Ok(Role::System),
        "model" | "assistant" => Ok(Role::Model),
        "user" => Ok(Role::User),
        _ => Err(Error::LLMResponseError("LLM returned message with an unknown role."))
    }
}

/// Interpret value as str
#[macro_export(local_inner_macros)]
macro_rules! val_as_str {
    ($val:expr, $element:literal) => {
        $val
            .as_str()
            .ok_or(Error::LLMResponseError(std::concat!("can't extract ", $element, " from LLM API response.")))?
    }
}

pub fn set_i64_param(payload: &mut Value, key: &str, val: &Option<i64>) {
    if let Some(v) = val {
        payload[key] = Value::Number(Number::from_i128(*v as i128).unwrap());
    }
}

pub fn set_f64_param(payload: &mut Value, key: &str, val: &Option<f64>) {
    if let Some(v) = val {
        if v.is_finite() {
            payload[key] = Value::Number(Number::from_f64(*v).unwrap());
        }
    }
}


pub fn tool_params_to_value(params: &[ToolParam], provider: ModelProvider) -> Value {
    let mut required = Vec::with_capacity(params.len());

    let mut result = json!({
        "type": "object",
        "properties": {},
    });

    for param in params {
        result["properties"][&param.name] = json!({
            "type": param.data_type,
            "description": param.description,
        });
        if param.required {
            required.push(Value::String(param.name.clone()));
        }
    }

    result["required"] = Value::Array(required);
    match provider {
        ModelProvider::OpenAI => {
            result["additionalProperties"] = Value::Bool(false);
        }
        ModelProvider::Anthropic => {
            result["additionalProperties"] = Value::Bool(false);
        }
        ModelProvider::GCP => {}
    }

    result
}
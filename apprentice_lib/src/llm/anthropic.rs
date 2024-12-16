use crate::llm::util::tool_params_to_value;
use crate::llm::{LLMChat, Role};
use crate::config::Config;
use crate::error::Error;
use crate::tools::{ToolChoice, ToolSpec};
use crate::val_as_str;
use serde_json::{json, Value};
use crate::request::Client;
use super::messages::Text;
use super::{Message, ToolCall, ToolParam};
use super::util::{self, llm_to_role, role_to_llm};

pub struct AnthropicChat {
    system_prompt: String,
    history: Vec<Value>,
    config: Config,
    client: Box<dyn Client>,
    tools: Vec<ToolSpec>,
}

impl AnthropicChat {
    pub(super) fn new(config: Config, client: Box<dyn Client>, tools: Vec<ToolSpec>) -> Result<Self, Error> {
        if config.api_version.is_none() {
            return Err(Error::MissingArgError("api-version is mandatory for anthropic."))
        }
        if config.max_tokens.is_none() {
            return Err(Error::MissingArgError("max-tokens is mandatory for anthropic."))
        }

        Ok(AnthropicChat {
            system_prompt: String::new(),
            history: vec![],
            config,
            client,
            tools,
        })
    }

    fn prep_payload(&mut self, messages: &[Message], tools: ToolChoice) -> Value {

        for message in messages {
            if let Message::Text(txt) = message {
                self.history.push(json!({
                    "role": role_to_llm(self.config.provider, txt.role),
                    "content": txt.message
                }));
            } else if let Message::ToolResult(res) = message {
                self.history.push(json!({
                    "role": role_to_llm(self.config.provider, Role::User),
                    "content": [
                        {
                          "type": "tool_result",
                          "tool_use_id": res.call_id,
                          "content": res.result
                        }
                    ]
                }));
            }
        }

        let mut payload = json!({
            "model": self.config.name,
            "system": self.system_prompt,
        });

        payload["messages"] = Value::Array(self.history.clone());

        util::set_i64_param(&mut payload, "max_tokens", &self.config.max_tokens);
        util::set_f64_param(&mut payload, "top_p", &self.config.top_p);
        util::set_i64_param(&mut payload, "top_k", &self.config.top_k);
        util::set_f64_param(&mut payload, "temperature", &self.config.temperature);

        if let Some(val) = &self.config.stop_sequence {
            payload["stop_sequences"] = Value::Array(vec![Value::String(val.clone())]);
        }

        self.prep_tool_use(&mut payload, tools);

        payload
    }

    fn prep_tool_use(&self, payload: &mut Value, tools: ToolChoice) {
        match tools {
            ToolChoice::None => {},
            ToolChoice::Auto => {
                payload["tool_choice"] = json!({
                    "type": "auto",
                    "disable_parallel_tool_use": true,
                });
                self.add_tools(payload);
            },
            ToolChoice::CallOne => {
                payload["tool_choice"] = json!({
                    "type": "any",
                    "disable_parallel_tool_use": true,
                });
                self.add_tools(payload);
            },
            ToolChoice::Force(tool) => {
                payload["tool_choice"] = json!({
                    "type": "tool",
                    "name": tool,
                    "disable_parallel_tool_use": true,
                });
                self.add_tools(payload);
            },
        };
    }

    fn add_tools(&self, payload: &mut Value) {
        let mut arr = Vec::with_capacity(self.tools.len());
        for spec in self.tools.iter() {
            arr.push(json!({
                "description": spec.description,
                "name": spec.name,
                "input_schema": tool_params_to_value(&spec.params, self.config.provider)
            }));
        }
        payload["tools"] = Value::Array(arr);
    }

    fn check_for_error(&self, response: &Value) -> Result<(), Error> {
        if let Some(error) = response.get("error") {
            let errmes = val_as_str!(error["message"], "error message").to_owned();
            return Err(Error::LLMErrorMessage(errmes));
        }
        Ok(())
    }

    fn process_response(&mut self, response: Value) -> Result<Vec<Message>, Error> {

        self.check_for_error(&response)?;

        let mut result = Vec::new();

        let role = val_as_str!(response["role"], "role");
        let role = llm_to_role(role)?;

        for msg in response["content"]
            .as_array()
            .ok_or(Error::LLMResponseError("can't enumerate messages in the response."))?
        {
            self.history.push(json!({
                "role": &response["role"],
                "content": [&msg]
            }));

            let msg_type = val_as_str!(msg["type"], "message type");

            if "text" == msg_type {

                let text = val_as_str!(msg["text"], "text").to_owned();

                result.push(Message::Text(Text{role, message: text}));

            } else if "tool_use" == msg_type {

                let call_id = val_as_str!(msg["id"], "tool call id").to_owned();
                let name = val_as_str!(msg["name"], "tool name").to_owned();
                let mut params = Vec::new();

                for (k, v) in msg["input"]
                    .as_object()
                    .ok_or(Error::LLMResponseError("can't enumerate tool call parameters."))?
                {
                    let name = k.clone();
                    let value = v.clone();
                    params.push(ToolParam {name, value});
                }

                result.push(Message::ToolCall(ToolCall{call_id, name, params}));

            } else {
                return Err(Error::LLMResponseError("unexpected message type."))
            }
        }

        Ok(result)
    }
}

impl LLMChat for AnthropicChat {

    fn get_inference(&mut self, messages: &[Message], tools: ToolChoice) -> Result<Vec<Message>, Error> {

        let payload = self.prep_payload(messages, tools);

        let api_ver: &str = self.config.api_version.as_ref().unwrap();
        let headers = &[
            ("x-api-key", self.config.api_key.as_ref()),
            ("anthropic-version", api_ver),
        ];

        let response = self.client.make_json_request(&self.config.api_url, payload, headers, &[])?;

        self.process_response(response)
    }

    fn clear_history(&mut self) {
        self.history.clear();
    }

    fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::stub::StubClient;
    use crate::llm::Role;
    use crate::tools::{ParamType, ToolParam};

    #[test]
    fn test_request_response_ok() {
        let config = Config {
            provider: "anthropic".try_into().expect("determine model provider"),
            name: "<model-name>".to_owned(),
            api_key: "<api-key>".to_owned(),
            api_url: "<api-uri>".to_owned(),
            api_version: Some("<api-ver>".to_owned()),
            max_tokens: Some(4096),
            n: Some(1),
            temperature: Some(0.123),
            top_p: Some(0.345),
            top_k: Some(5),
            frequency_penalty: Some(-0.11),
            presence_penalty: Some(0.22),
            stop_sequence: Some("<stop>".to_owned()),
        };

        let sys_msg = "test sys message";
        let user_msg = "test user message";
        let model_msg = "test resp message";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg.to_owned()));
        let mut expected_messages = Vec::new();
        expected_messages.push(Message::text(Role::Model, model_msg.to_owned()));

        let expected_headers = vec![
            ("x-api-key".to_owned(), config.api_key.clone()), 
            ("anthropic-version".to_owned(), config.api_version.clone().unwrap()),
        ];
        let expected_params = vec![];
        let expected_payload = json!({
            "model": config.name,
            "max_tokens": config.max_tokens.unwrap(),
            "messages": [
                {"role": "user", "content": user_msg}
            ],
            "system": sys_msg,
            "stop_sequences": [config.stop_sequence.as_ref().unwrap()],
            "temperature": config.temperature.unwrap(),
            "top_k": config.top_k.unwrap(),
            "top_p": config.top_p.unwrap(),
        });
        let response_body = json!({
            "content": [
              {
                "text": model_msg,
                "type": "text"
              }
            ],
            "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
            "model": config.name,
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {
              "input_tokens": 123,
              "output_tokens": 123
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = AnthropicChat::new(config, client, vec![]).expect("AnthropicChat initialization");
        
        chat.set_system_prompt(sys_msg.to_owned());

        let response = chat.get_inference(&messages, ToolChoice::None).expect("receive response");

        assert_eq!(expected_messages.len(), response.len());
        for (msg1, msg2) in response.iter().zip(expected_messages.iter()) {
            if let (Message::Text(txt1), Message::Text(txt2)) = (msg1, msg2) {
                assert_eq!(txt1.role, txt2.role);
                assert_eq!(txt1.message, txt2.message);    
            } else {
                panic!("type mismatch");
            }
        }
    }

    #[test]
    fn test_request_response_err() {
        let config = Config {
            provider: "anthropic".try_into().expect("determine model provider"),
            name: "<model-name>".to_owned(),
            api_key: "<api-key>".to_owned(),
            api_url: "<api-uri>".to_owned(),
            api_version: Some("<api-ver>".to_owned()),
            max_tokens: Some(4096),
            n: Some(1),
            temperature: Some(0.123),
            top_p: Some(0.345),
            top_k: Some(5),
            frequency_penalty: Some(-0.11),
            presence_penalty: Some(0.22),
            stop_sequence: Some("<stop>".to_owned()),
        };

        let sys_msg = "test sys message";
        let user_msg = "test user message";
        let model_msg = "test resp message";

        let mut messages = vec![];
        messages.push(Message::text(Role::User, user_msg.to_owned()));

        let expected_headers = vec![
            ("x-api-key".to_owned(), config.api_key.clone()), 
            ("anthropic-version".to_owned(), config.api_version.clone().unwrap()),
        ];
        let expected_params = vec![];
        let expected_payload = json!({
            "model": config.name,
            "max_tokens": config.max_tokens.unwrap(),
            "messages": [
                {"role": "user", "content": user_msg}
            ],
            "system": sys_msg,
            "stop_sequences": [config.stop_sequence.as_ref().unwrap()],
            "temperature": config.temperature.unwrap(),
            "top_k": config.top_k.unwrap(),
            "top_p": config.top_p.unwrap(),
        });
        let response_body = json!({
            "type": "error",
            "error": {
              "type": "invalid_request_error",
              "message": model_msg
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = AnthropicChat::new(config, client, vec![]).expect("AnthropicChat initialization");

        chat.set_system_prompt(sys_msg.to_owned());

        let response = chat.get_inference(&messages, ToolChoice::None);

        if let Err(Error::LLMErrorMessage(msg)) = response {
            assert_eq!(msg, model_msg);
        } else {
            panic!("type mismatch");
        }
    }


    #[test]
    fn test_request_response_tool_ok() {
        let config = Config {
            provider: "anthropic".try_into().expect("determine model provider"),
            name: "<model-name>".to_owned(),
            api_key: "<api-key>".to_owned(),
            api_url: "<api-uri>".to_owned(),
            api_version: Some("<api-ver>".to_owned()),
            max_tokens: Some(4096),
            n: Some(1),
            temperature: Some(0.123),
            top_p: Some(0.345),
            top_k: Some(5),
            frequency_penalty: Some(-0.11),
            presence_penalty: Some(0.22),
            stop_sequence: Some("<stop>".to_owned()),
        };

        let tools = vec![
            ToolSpec {
                name: "tool1".to_owned(),
                description: "tool desc 1".to_owned(),
                params: vec![
                    ToolParam {
                        name: "tool1_param1".to_string(),
                        description: "tool1_param1 desc".to_string(), 
                        data_type: ParamType::Integer, 
                        required: true
                    },
                    ToolParam {
                        name: "tool1_param2".to_string(),
                        description: "tool1_param2 desc".to_string(), 
                        data_type: ParamType::String, 
                        required: false
                    },
                ]
            },
            ToolSpec {
                name: "tool2".to_owned(),
                description: "tool desc 2".to_owned(),
                params: vec![
                    ToolParam {
                        name: "tool2_param1".to_string(),
                        description: "tool2_param1 desc".to_string(), 
                        data_type: ParamType::Boolean, 
                        required: false
                    },
                    ToolParam {
                        name: "tool2_param2".to_string(),
                        description: "tool2_param2 desc".to_string(), 
                        data_type: ParamType::Number, 
                        required: true
                    },
                ]
            },
        ];

        let call_id = "toolu_01A09q90qw90lq917835lq9";
        let call_tool = "tool2";
        let call_params = vec![
            super::ToolParam { name: "tool2_param2".to_owned(), value: Value::Bool(true) },
        ];

        let sys_msg = "test sys message";
        let user_msg = "test user message";
        let model_msg = "test resp message";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg.to_owned()));
        let mut expected_messages = Vec::new();
        expected_messages.push(Message::text(Role::Model, model_msg.to_owned()));
        expected_messages.push(Message::tool_use(call_id.to_owned(), call_tool.to_owned(), call_params));

        let expected_headers = vec![
            ("x-api-key".to_owned(), config.api_key.clone()), 
            ("anthropic-version".to_owned(), config.api_version.clone().unwrap()),
        ];
        let expected_params = vec![];
        let expected_payload = json!({
            "model": config.name,
            "max_tokens": config.max_tokens.unwrap(),
            "messages": [
                {"role": "user", "content": user_msg}
            ],
            "system": sys_msg,
            "stop_sequences": [config.stop_sequence.as_ref().unwrap()],
            "temperature": config.temperature.unwrap(),
            "top_k": config.top_k.unwrap(),
            "top_p": config.top_p.unwrap(),
            "tool_choice": {
                "type": "auto",
                "disable_parallel_tool_use": true,
            },
            "tools": [
                {
                    "name": "tool1",
                    "description": "tool desc 1",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "tool1_param1": {
                                "type": "integer",
                                "description": "tool1_param1 desc"
                            },
                            "tool1_param2": {
                                "type": "string",
                                "description": "tool1_param2 desc"
                            },
                        },
                        "required": ["tool1_param1"],
                        "additionalProperties": false,
                    }
                },
                {
                    "name": "tool2",
                    "description": "tool desc 2",
                    "input_schema": {
                        "type": "object",
                        "properties": {
                            "tool2_param1": {
                                "type": "boolean",
                                "description": "tool2_param1 desc"
                            },
                            "tool2_param2": {
                                "type": "number",
                                "description": "tool2_param2 desc"
                            },
                        },
                        "required": ["tool2_param2"],
                        "additionalProperties": false,
                    }
                }
            ]
        });
        let response_body = json!({
            "content": [
              {
                "text": model_msg,
                "type": "text"
              },
              {
                "type": "tool_use",
                "id": call_id,
                "name": call_tool,
                "input": {"tool2_param2": -1.2345, "tool2_param2": true}
              }
            ],
            "id": "msg_013Zva2CMHLNnXjNJJKqJ2EF",
            "model": config.name,
            "role": "assistant",
            "stop_reason": "end_turn",
            "stop_sequence": null,
            "type": "message",
            "usage": {
              "input_tokens": 123,
              "output_tokens": 123
            }
          });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = AnthropicChat::new(config, client, tools).expect("AnthropicChat initialization");
        
        chat.set_system_prompt(sys_msg.to_owned());

        let response = chat.get_inference(&messages, ToolChoice::Auto).expect("receive response");

        assert_eq!(expected_messages.len(), response.len());
        if let (Message::Text(txt1), Message::Text(txt2)) = (&expected_messages[0], &response[0]) {
            assert_eq!(txt1.role, txt2.role);
            assert_eq!(txt1.message, txt2.message);    
        } else {
            panic!("type mismatch");
        }

        if let (Message::ToolCall(call1), Message::ToolCall(call2)) = (&expected_messages[1], &response[1]) {
            assert_eq!(call1.call_id, call2.call_id);
            assert_eq!(call1.name, call2.name);
            for (param1, param2) in call1.params.iter().zip(call2.params.iter()) {
                assert_eq!(param1.name, param2.name);
                assert_eq!(param1.value, param2.value);
            }
        } else {
            panic!("type mismatch");
        }
    }

}
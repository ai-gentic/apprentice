use crate::llm::util::{role_to_llm, tool_params_to_value};
use crate::llm::LLMChat;
use crate::config::Config;
use crate::error::Error;
use crate::tools::{ToolChoice, ToolSpec};
use crate::val_as_str;
use serde_json::{json, Value};
use crate::request::Client;
use super::messages::Text;
use super::{Message, ToolCall, ToolParam};
use super::util::{self, llm_to_role};

pub struct OpenAIChat {
    system_prompt: String,
    history: Vec<Value>,
    config: Config,
    client: Box<dyn Client>,
    tools: Vec<ToolSpec>,
}

impl OpenAIChat {
    pub(super) fn new(config: Config, client: Box<dyn Client>, tools: Vec<ToolSpec>) -> Self {
        OpenAIChat {
            system_prompt: String::new(),
            history: vec![],
            config,
            client,
            tools,
        }
    }

    fn prep_payload(&mut self, messages: &[Message], tools: ToolChoice) -> Value {

        let mut payload = json!({
            "model": self.config.name
        });

        for message in messages {    
            if let Message::Text(txt) = message {
                self.history.push(json!({
                    "role": role_to_llm(self.config.provider, txt.role), 
                    "content": txt.message
                }));
            } else if let Message::ToolResult(res) = message {
                self.history.push(json!({
                    "role": "tool",
                    "content": res.result,
                    "tool_call_id": res.call_id
                }));
            }
        }

        payload["messages"] = Value::Array(self.history.clone());

        util::set_f64_param(&mut payload, "frequency_penalty", &self.config.frequency_penalty);
        util::set_f64_param(&mut payload, "presence_penalty", &self.config.presence_penalty);
        util::set_i64_param(&mut payload, "n", &self.config.n);
        util::set_f64_param(&mut payload, "top_p", &self.config.top_p);
        util::set_f64_param(&mut payload, "temperature", &self.config.temperature);
        util::set_i64_param(&mut payload, "max_completion_tokens", &self.config.max_tokens);

        if let Some(val) = &self.config.stop_sequence {
            payload["stop"] = Value::String(val.clone());
        }

        self.prep_tool_use(&mut payload, tools);

        payload
    }

    fn prep_tool_use(&self, payload: &mut Value, tools: ToolChoice) {
        match tools {
            ToolChoice::None => {},
            ToolChoice::Auto => {
                payload["tool_choice"] = Value::String("auto".to_owned());
                self.add_tools(payload);
            },
            ToolChoice::CallOne => {
                payload["tool_choice"] = Value::String("required".to_owned());
                self.add_tools(payload);
            },
            ToolChoice::Force(tool) => {
                payload["tool_choice"] = json!({
                    "type": "function", 
                    "function": {
                        "name": tool
                    }
                });
                self.add_tools(payload);
            },
        };
        payload["parallel_tool_calls"] = Value::Bool(false);
    }

    fn add_tools(&self, payload: &mut Value) {
        let mut arr = Vec::with_capacity(self.tools.len());
        for spec in self.tools.iter() {
            arr.push(json!({
                "type": "function",
                "function": {
                    "description": spec.description,
                    "name": spec.name,
                    "parameters": tool_params_to_value(&spec.params, self.config.provider),
                    "strict": true
                }
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

        for choice in response["choices"].as_array()
            .ok_or(Error::LLMResponseError("unexpected answer format, can't enumerate response messages."))?
        {
            let msg = &choice["message"];

            self.history.push(msg.clone());

            let role = llm_to_role(val_as_str!(msg["role"], "message role"))?;

            if !msg["content"].is_null() {
                let content = val_as_str!(msg["content"], "message content").to_owned();
                result.push(Message::Text(Text{role, message: content}));
            }

            if !msg["refusal"].is_null() {
                let content = val_as_str!(msg["refusal"], "refusal content").to_owned();
                result.push(Message::Text(Text{role, message: content}));
            }

            if !msg["tool_calls"].is_null() {
                for call in msg["tool_calls"].as_array()
                    .ok_or(Error::LLMResponseError("unexpected answer format, can't enumerate tool call requests."))?
                {
                    let call_id = val_as_str!(call["id"], "tool call id").to_owned();
                    let name = val_as_str!(call["function"]["name"], "tool name").to_owned();
                    let arguments = val_as_str!(call["function"]["arguments"], "tool arguments").to_owned();

                    let mut params = Vec::new();

                    let args_obj = serde_json::from_str::<Value>(&arguments)?;
                    
                    for (k, v) in args_obj
                        .as_object()
                        .ok_or(Error::LLMResponseError("can't enumerate arguments."))?
                    {
                        let name = k.clone();
                        let value = v.clone();
                        params.push(ToolParam {name, value});
                    }

                    result.push(Message::ToolCall(ToolCall{call_id, name, params}));
                }
            }
        }

        Ok(result)
    }
}

impl LLMChat for OpenAIChat {

    fn get_inference(&mut self, messages: &[Message], tools: ToolChoice) -> Result<Vec<Message>, Error> {
        let payload = self.prep_payload(messages, tools);

        let token = format!("Bearer {}", self.config.api_key);
        let headers = &[("Authorization", token.as_ref())];

        let response = self.client.make_json_request(&self.config.api_url, payload, headers, &[])?;

        self.process_response(response)
    }

    fn clear_history(&mut self) {
        self.history.clear();
    }

    fn set_system_prompt(&mut self, prompt: String) {
        self.system_prompt = prompt;
        let val = json!({
            "role": "system", 
            "content": self.system_prompt.clone(),
        });

        if self.history.is_empty() {
            self.history.push(val);
        } else {
            self.history[0] = val;
        }
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
            provider: "openai".try_into().expect("determine model provider"),
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
        let user_msg1 = "test user message 1";
        let user_msg2 = "test user message 2";
        let model_msg1 = "test resp message 1";
        let model_msg2 = "test resp message 2";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg1.to_owned()));
        messages.push(Message::text(Role::Model, model_msg1.to_owned()));
        messages.push(Message::text(Role::User, user_msg2.to_owned()));
        let mut expected_messages = Vec::new();
        expected_messages.push(Message::text(Role::Model, model_msg2.to_owned()));

        let expected_headers = vec![
            ("Authorization".to_owned(), format!("Bearer {}", config.api_key))
        ];
        let expected_params = vec![];
        let expected_payload = json!({
            "model": config.name,
            "messages": [
              {
                "role": "system",
                "content": sys_msg
              },
              {
                "role": "user",
                "content": user_msg1
              },
              {
                "role": "assistant",
                "content": model_msg1
              },
              {
                "role": "user",
                "content": user_msg2
              }
            ],
            "frequency_penalty": config.frequency_penalty.unwrap(),
            "max_completion_tokens": config.max_tokens.unwrap(),
            "n": config.n.unwrap(),
            "presence_penalty": config.presence_penalty.unwrap(),
            "stop": config.stop_sequence.as_ref().unwrap(),
            "temperature": config.temperature.unwrap(),
            "top_p": config.top_p.unwrap(),
            "parallel_tool_calls": false,
            "tool_choice": "auto", 
            "tools": [],
        });

        let response_body = json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "created": 1677652288,
            "model": config.name,
            "system_fingerprint": "fp_44709d6fcb",
            "choices": [{
              "index": 0,
              "message": {
                "role": "assistant",
                "content": model_msg2,
              },
              "logprobs": null,
              "finish_reason": "stop"
            }],
            "usage": {
              "prompt_tokens": 9,
              "completion_tokens": 12,
              "total_tokens": 21,
              "completion_tokens_details": {
                "reasoning_tokens": 0,
                "accepted_prediction_tokens": 0,
                "rejected_prediction_tokens": 0
              }
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = OpenAIChat::new(config, client, vec![]);
        
        chat.set_system_prompt(sys_msg.to_owned());
        
        let response = chat.get_inference(&messages, ToolChoice::Auto).expect("receive response");

        assert_eq!(expected_messages.len(), response.len());
        if let (Message::Text(txt1), Message::Text(txt2)) = (&expected_messages[0], &response[0]) {
            assert_eq!(txt1.role, txt2.role);
            assert_eq!(txt1.message, txt2.message);    
        } else {
            panic!("type mismatch");
        }
    }

    #[test]
    fn test_request_response_err() {
        let config = Config {
            provider: "openai".try_into().expect("determine model provider"),
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
        let user_msg1 = "test user message 1";
        let user_msg2 = "test user message 2";
        let model_msg1 = "test resp message 1";
        let model_msg2 = "You exceeded your current quota, please check your plan and billing details. For more information on this error, read the docs: https://platform.openai.com/docs/guides/error-codes/api-errors.";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg1.to_owned()));
        messages.push(Message::text(Role::Model, model_msg1.to_owned()));
        messages.push(Message::text(Role::User, user_msg2.to_owned()));

        let expected_headers = vec![
            ("Authorization".to_owned(), format!("Bearer {}", config.api_key))
        ];
        let expected_params = vec![];
        let expected_payload = json!({
            "model": config.name,
            "messages": [
              {
                "role": "system",
                "content": sys_msg
              },
              {
                "role": "user",
                "content": user_msg1
              },
              {
                "role": "assistant",
                "content": model_msg1
              },
              {
                "role": "user",
                "content": user_msg2
              }
            ],
            "frequency_penalty": config.frequency_penalty.unwrap(),
            "max_completion_tokens": config.max_tokens.unwrap(),
            "n": config.n.unwrap(),
            "presence_penalty": config.presence_penalty.unwrap(),
            "stop": config.stop_sequence.as_ref().unwrap(),
            "temperature": config.temperature.unwrap(),
            "top_p": config.top_p.unwrap(),
            "parallel_tool_calls": false,
            "tool_choice": "auto", 
            "tools": [],
        });

        let response_body = json!({
            "error": {
                "code": "insufficient_quota",
                "message": model_msg2,
                "param": null,
                "type": "insufficient_quota"
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = OpenAIChat::new(config, client, vec![]);
        
        chat.set_system_prompt(sys_msg.to_owned());
        
        let response = chat.get_inference(&messages, ToolChoice::Auto);

        if let Err(Error::LLMErrorMessage(msg)) = response {
            assert_eq!(msg, model_msg2);
        } else {
            panic!("type mismatch");
        }
    }

    #[test]
    fn test_request_response_tool_ok() {
        let config = Config {
            provider: "openai".try_into().expect("determine model provider"),
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

        let call_id = "call_id";
        let call_tool = "tool2";
        let call_params = vec![
            super::ToolParam { name: "tool2_param2".to_owned(), value: Value::Bool(true) },
        ];

        let sys_msg = "test sys message";
        let user_msg1 = "test user message 1";
        let user_msg2 = "test user message 2";
        let model_msg1 = "test resp message 1";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg1.to_owned()));
        messages.push(Message::text(Role::Model, model_msg1.to_owned()));
        messages.push(Message::text(Role::User, user_msg2.to_owned()));
        let mut expected_messages = Vec::new();
        expected_messages.push(Message::tool_use(call_id.to_owned(), call_tool.to_owned(), call_params));

        let expected_headers = vec![];
        let expected_params = vec![
            ("key".to_owned(), config.api_key.clone()), 
        ];
        let expected_payload = json!({
            "model": config.name,
            "messages": [
              {
                "role": "system",
                "content": sys_msg
              },
              {
                "role": "user",
                "content": user_msg1
              },
              {
                "role": "assistant",
                "content": model_msg1
              },
              {
                "role": "user",
                "content": user_msg2
              }
            ],
            "frequency_penalty": config.frequency_penalty.unwrap(),
            "max_completion_tokens": config.max_tokens.unwrap(),
            "n": config.n.unwrap(),
            "presence_penalty": config.presence_penalty.unwrap(),
            "stop": config.stop_sequence.as_ref().unwrap(),
            "temperature": config.temperature.unwrap(),
            "top_p": config.top_p.unwrap(),
            "parallel_tool_calls": false,
            "tool_choice": "auto", 
            "tools": [{
                "type": "function",
                "function": {
                    "description": "tool desc 1",
                    "name": "tool1",
                    "strict": true,
                    "parameters": {
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
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "tool2",
                    "description": "tool desc 2",
                    "strict": true,
                    "parameters": {
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
            }]
        });

        let response_body = json!({
          "id": "chatcmpl-123",
          "object": "chat.completion",
          "created": 1677652288,
          "model": config.name,
          "system_fingerprint": "fp_44709d6fcb",
          "choices": [{
            "index": 0,
            "message": {
              "role": "assistant",
              "tool_calls": [
                  {
                      "id": call_id,
                      "type": "function",
                      "function": {
                          "arguments": "{\"tool2_param2\": true}",
                          "name": call_tool
                      }
                  }
              ]
            },
            "logprobs": null,
            "finish_reason": "stop"
          }],
          "usage": {
            "prompt_tokens": 9,
            "completion_tokens": 12,
            "total_tokens": 21,
            "completion_tokens_details": {
              "reasoning_tokens": 0,
              "accepted_prediction_tokens": 0,
              "rejected_prediction_tokens": 0
            }
          }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = OpenAIChat::new(config, client, tools);

        chat.set_system_prompt(sys_msg.to_owned());

        let response = chat.get_inference(&messages, ToolChoice::Auto).expect("receive response");

        assert_eq!(expected_messages.len(), response.len());
        if let (Message::ToolCall(call1), Message::ToolCall(call2)) = (&expected_messages[0], &response[0]) {
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
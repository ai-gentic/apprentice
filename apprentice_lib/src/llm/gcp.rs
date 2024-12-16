
use crate::llm::util::{role_to_llm, tool_params_to_value};
use crate::llm::LLMChat;
use crate::config::Config;
use crate::error::Error;
use crate::tools::{ToolChoice, ToolSpec};
use crate::val_as_str;
use serde_json::{json, Value};
use crate::request::Client;
use super::{Message, ToolCall, ToolParam};
use super::util::{self, llm_to_role};

pub struct GcpChat {
    system_prompt: String,
    history: Vec<Value>,
    config: Config,
    client: Box<dyn Client>,
    tools: Vec<ToolSpec>,
}

impl GcpChat {
    pub(super) fn new(config: Config, client: Box<dyn Client>,  tools: Vec<ToolSpec>) -> Result<Self, Error> {
        Ok(GcpChat {
            system_prompt: String::new(),
            history: vec![],
            config,
            client,
            tools,
        })
    }

    fn prep_payload(&mut self, messages: &[Message], tools: ToolChoice) -> Value {

        let mut payload = json!({
            "systemInstruction": {
                "parts":
                  { "text": self.system_prompt }
            }
        });

        for message in messages {
            if let Message::Text(txt) = message {
                self.history.push(json!({
                    "role": role_to_llm(self.config.provider, txt.role),
                    "parts": [{"text": txt.message}]
                }));
            } else if let Message::ToolResult(res) = message {
                self.history.push(json!({
                    "role": "user",
                    "parts": [{
                        "functionResponse": {
                            "name": res.name,
                            "response": {
                                "name": res.name,
                                "content": res.result
                            }
                        }
                    }]
                }));
            }
        }

        payload["contents"] = Value::Array(self.history.clone());

        payload["generationConfig"] = json!({});

        util::set_i64_param(&mut payload["generationConfig"], "maxOutputTokens", &self.config.max_tokens);
        util::set_f64_param(&mut payload["generationConfig"], "topP", &self.config.top_p);
        util::set_i64_param(&mut payload["generationConfig"], "topK", &self.config.top_k);
        util::set_f64_param(&mut payload["generationConfig"], "temperature", &self.config.temperature);
        util::set_f64_param(&mut payload["generationConfig"], "presencePenalty", &self.config.presence_penalty);
        util::set_f64_param(&mut payload["generationConfig"], "frequencyPenalty", &self.config.frequency_penalty);

        if let Some(val) = &self.config.stop_sequence {
            payload["generationConfig"]["stopSequences"] = Value::Array(vec![Value::String(val.clone())]);
        }

        self.prep_tool_use(&mut payload, tools);

        payload
    }

    fn prep_tool_use(&self, payload: &mut Value, tools: ToolChoice) {
        match tools {
            ToolChoice::None => {},
            ToolChoice::Auto => {
                payload["tool_config"] = json!({
                    "function_calling_config": {
                        "mode": "AUTO"
                    }
                });
                self.add_tools(payload);
            },
            ToolChoice::CallOne => {
                payload["tool_config"] = json!({
                    "function_calling_config": {
                        "mode": "ANY"
                    }
                });
                self.add_tools(payload);
            },
            ToolChoice::Force(tool) => {
                payload["tool_config"] = json!({
                    "function_calling_config": {
                      "mode": "ANY",
                      "allowed_function_names": [tool]
                    },
                });
                self.add_tools(payload);
            },
        };
    }

    fn add_tools(&self, payload: &mut Value) {
        let mut arr = Vec::with_capacity(self.tools.len());
        for spec in self.tools.iter() {
            arr.push(json!({
                "name": spec.name,
                "description": spec.description,
                "parameters": tool_params_to_value(&spec.params, self.config.provider)
            }));
        }
        payload["tools"] = json!([{
            "function_declarations": arr
        }]);
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

        for candidate in response["candidates"]
            .as_array()
            .ok_or(Error::LLMResponseError("can't enumerate messages in the response."))?
        {
            self.history.push(candidate["content"].clone());

            let role = llm_to_role(val_as_str!(candidate["content"]["role"], "message role"))?;

            for part in candidate["content"]["parts"]
                .as_array()
                .ok_or(Error::LLMResponseError("unexpected answer format, can't enumerate message parts."))?
            {
                if part["functionCall"].is_object() {
                    let name = val_as_str!(part["functionCall"]["name"], "tool name").to_owned();
                    let mut params = Vec::new();

                    for (k, v) in part["functionCall"]["args"]
                        .as_object()
                        .ok_or(Error::LLMResponseError("can't enumerate tool call parameters."))?
                    {
                        let name = k.clone();
                        let value = v.clone();
                        params.push(ToolParam {name, value});
                    }

                    result.push(Message::ToolCall(ToolCall{call_id: String::new(), name, params}));

                } else if part["text"].is_string() {
                    let message = part["text"].as_str().unwrap().to_owned();
                    result.push(Message::text(role, message));
                } else {
                    return Err(Error::LLMResponseError("unexpected message type."))
                }
            }
        }

        Ok(result)
    }
}

impl LLMChat for GcpChat {

    fn get_inference(&mut self, messages: &[Message], tools: ToolChoice) -> Result<Vec<Message>, Error> {

        let payload = self.prep_payload(messages, tools);

        let params = &[("key", self.config.api_key.as_ref())];

        let response = self.client.make_json_request(&self.config.api_url, payload, &[], params)?;

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
            provider: "gcp".try_into().expect("determine model provider"),
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

        let expected_headers = vec![];
        let expected_params = vec![
            ("key".to_owned(), config.api_key.clone()), 
        ];
        let expected_payload = json!({
            "systemInstruction": {
                "parts": { "text": sys_msg }
            },
            "contents": [
              {"role":"user",
               "parts":[{
                 "text": user_msg1}]},
              {"role": "model",
               "parts":[{
                 "text": model_msg1}]},
              {"role":"user",
               "parts":[{
                 "text": user_msg2}]},
            ],
            "generationConfig": {
                "maxOutputTokens": config.max_tokens.unwrap(),
                "topP": config.top_p.unwrap(),
                "topK": config.top_k.unwrap(),
                "temperature": config.temperature.unwrap(),
                "presencePenalty": config.presence_penalty.unwrap(),
                "frequencyPenalty": config.frequency_penalty.unwrap(),
                "stopSequences": [
                    config.stop_sequence.as_ref().unwrap()
                ]
            },
            "tool_config": {
                "function_calling_config": {
                    "mode": "AUTO"
                }
            }, 
            "tools": [
                {
                    "function_declarations": []
                }
            ]
        });

        let response_body = json!({
            "candidates": [
              {
                "avgLogprobs": -0.024475134909152985,
                "content": {
                  "parts": [
                    {
                      "text": model_msg2
                    }
                  ],
                  "role": "model"
                },
                "finishReason": "STOP"
              }
            ],
            "modelVersion": config.name,
            "usageMetadata": {
              "candidatesTokenCount": 10,
              "promptTokenCount": 1744,
              "totalTokenCount": 1754
            }
          });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = GcpChat::new(config, client, vec![]).expect("Chat initialization");
        
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
            provider: "gcp".try_into().expect("determine model provider"),
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
        let model_msg2 = "API key not valid. Please pass a valid API key.";

        let mut messages = Vec::new();
        messages.push(Message::text(Role::User, user_msg1.to_owned()));
        messages.push(Message::text(Role::Model, model_msg1.to_owned()));
        messages.push(Message::text(Role::User, user_msg2.to_owned()));

        let expected_headers = vec![];
        let expected_params = vec![
            ("key".to_owned(), config.api_key.clone()), 
        ];
        let expected_payload = json!({
            "systemInstruction": {
                "parts": { "text": sys_msg }
            },
            "contents": [
              {"role":"user",
               "parts":[{
                 "text": user_msg1}]},
              {"role": "model",
               "parts":[{
                 "text": model_msg1}]},
              {"role":"user",
               "parts":[{
                 "text": user_msg2}]},
            ],
            "generationConfig": {
                "maxOutputTokens": config.max_tokens.unwrap(),
                "topP": config.top_p.unwrap(),
                "topK": config.top_k.unwrap(),
                "temperature": config.temperature.unwrap(),
                "presencePenalty": config.presence_penalty.unwrap(),
                "frequencyPenalty": config.frequency_penalty.unwrap(),
                "stopSequences": [
                    config.stop_sequence.as_ref().unwrap()
                ]
            },
            "tool_config": {
                "function_calling_config": {
                    "mode": "AUTO"
                }
            }, 
            "tools": [
                {
                    "function_declarations": []
                }
            ]
        });

        let response_body = json!({
            "error": {
                "code":400,
                "details": [
                    {
                        "@type": "type.googleapis.com/google.rpc.ErrorInfo",
                        "domain": "googleapis.com",
                        "metadata": {
                            "service": "generativelanguage.googleapis.com"
                        },
                        "reason": "API_KEY_INVALID"
                    },
                    {
                        "@type": "type.googleapis.com/google.rpc.LocalizedMessage",
                        "locale": "en-US",
                        "message": "API key not valid. Please pass a valid API key."
                    }
                ],
                "message": model_msg2,
                "status": "INVALID_ARGUMENT"
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = GcpChat::new(config, client, vec![]).expect("Chat initialization");
        
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
            provider: "gcp".try_into().expect("determine model provider"),
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

        let call_id = "";
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
            "systemInstruction": {
                "parts": { "text": sys_msg }
            },
            "contents": [
              {"role":"user",
               "parts":[{
                 "text": user_msg1}]},
              {"role": "model",
               "parts":[{
                 "text": model_msg1}]},
              {"role":"user",
               "parts":[{
                 "text": user_msg2}]},
            ],
            "generationConfig": {
                "maxOutputTokens": config.max_tokens.unwrap(),
                "topP": config.top_p.unwrap(),
                "topK": config.top_k.unwrap(),
                "temperature": config.temperature.unwrap(),
                "presencePenalty": config.presence_penalty.unwrap(),
                "frequencyPenalty": config.frequency_penalty.unwrap(),
                "stopSequences": [
                    config.stop_sequence.as_ref().unwrap()
                ]
            },
            "tool_config": {
                "function_calling_config": {
                    "mode": "AUTO"
                }
            },
            "tools": [{
                "function_declarations": [{
                    "description": "tool desc 1",
                    "name": "tool1",
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
                    }
                },
                {
                    "name": "tool2",
                    "description": "tool desc 2",
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
                    }
                }]
            }]
        });

        let response_body = json!({
            "candidates": [
              {
                "avgLogprobs": -0.024475134909152985,
                "content": {
                    "parts": [
                      {
                        "functionCall": {
                          "args": {
                            "tool2_param2": true
                          },
                          "name": "tool2"
                        }
                      }
                    ],
                    "role": "model"
                },
                "finishReason": "STOP"
              }
            ],
            "modelVersion": config.name,
            "usageMetadata": {
              "candidatesTokenCount": 10,
              "promptTokenCount": 1744,
              "totalTokenCount": 1754
            }
        });

        let client = Box::new(StubClient::new(expected_headers, expected_params, expected_payload, response_body));

        let mut chat = GcpChat::new(config, client, tools).expect("Chat initialization");
        
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
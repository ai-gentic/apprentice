use crate::config::ModelProvider;
use crate::llm::openai::OpenAIChat;
use crate::config::Config;
use crate::error::Error;
use crate::request::Client;
use crate::tools::{ToolChoice, ToolSpec};
use super::anthropic::AnthropicChat;
use super::gcp::GcpChat;
use super::Message;

/// Chat with LLM with storing history.
pub trait LLMChat {

    /// Add input messages to the message history.
    /// Input messages contains user message(s), and tool call results.
    /// Returns n messages as the result, and/or tool call requests.
    fn get_inference(&mut self, messages: &[Message], tools: ToolChoice) -> Result<Vec<Message>, Error>;

    /// Clear chat history.
    fn clear_history(&mut self);

    /// Update system prompt.
    fn set_system_prompt(&mut self, prompt: String);
}

/* TODO: split LLM and chat. Chat should keep history, LLm is stateless.
/// LLM
pub trait LLM {

    /// Call LLM and return the call result.
    /// Returns n messages as the result, and/or tool call requests.
    fn get_inference(&mut self, system: &str, messages: &Message, tools: ToolChoice) -> Result<Vec<Message>, Error>;
}
 */

/// Create LLMChat instance.
pub fn get_llm_chat(config: Config, client: Box<dyn Client>, tools: Vec<ToolSpec>) -> Result<Box<dyn LLMChat>, Error> {
    Ok(match config.provider {
        ModelProvider::OpenAI => Box::new(OpenAIChat::new(config, client, tools)),
        ModelProvider::Anthropic => Box::new(AnthropicChat::new(config, client, tools)?),
        ModelProvider::GCP => Box::new(GcpChat::new(config, client, tools)?),
    })
}
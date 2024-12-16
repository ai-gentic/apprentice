//! Apprentice-lib is a library that allows to create agent applications.
//! It allows to create a chat with an LLM and use tools/functions.
//! 
//! ### Features
//! 
//!  - several providers
//!  - light-weight
//!  - configurable
//!  - extensible
//! 
//! ### Providers
//! 
//! - Anthropic (Claude models)
//! - OpeanAI (GPT models)
//! - Google Cloud Platform (Gemini)
//! 
//! ### Examples
//! 
//! ```rust no_run
//! use apprentice_lib::llm::{get_llm_chat, Message, Role};
//! use apprentice_lib::tools::ToolChoice;
//! use apprentice_lib::request::get_reqwest_client;
//! use apprentice_lib::ModelProvider;
//! use apprentice_lib::Config;
//!
//! let config = Config::new(ModelProvider::OpenAI, "gpt-4".into(), "<api-key>".into(), "https://api.openai.com/v1/chat/completions".into());
//! 
//! let reqwest_client = get_reqwest_client().expect("transport created");
//! 
//! let mut chat = get_llm_chat(config, reqwest_client, vec![]).expect("chat created");
//! 
//! chat.set_system_prompt("You are a helpful assistant.".into());
//! 
//! let user_message = Message::text(Role::User, "Hi assistant!".into());
//! 
//! let response = chat.get_inference(&[user_message], ToolChoice::None).expect("LLM response");
//!
//! for message in response.iter() {
//!     match message {
//!         Message::Text(text) => { /* process text message */ }
//!         Message::ToolCall(tool_call) => { /* process tool use request */ }
//!         Message::ToolResult(_) => { panic!("LLM must not respond with tool result!") }
//!     };
//! }
//! ```

#![deny(missing_docs)]
#![deny(clippy::suspicious)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::collapsible_if)]

mod error;
mod config;
pub mod llm;
pub mod tools;
pub mod request;

pub use error::Error;
pub use config::Config;
pub use config::ModelProvider;
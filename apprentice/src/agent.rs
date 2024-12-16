use crate::config::Config;
use crate::prompts::Prompts;
use crate::tools::{Help, Shell};
use apprentice_lib::llm::{get_llm_chat, LLMChat, Message, Role, ToolCall};
use apprentice_lib::tools::ToolChoice;
use crate::error::AppError;
use crate::term::Term;
use apprentice_lib::Error;
use apprentice_lib::request::get_reqwest_client;
use rustyline::error::ReadlineError;

/// Agent.
pub struct Agent {
    config: Config,
    term: Term,
    shell: Shell,
    help: Help,
    chat: Box<dyn LLMChat>,
}

impl Agent {

    /// Create new agent.
    pub fn new(config: Config, prompts: Prompts) -> Result<Self, AppError> {
        let term = Term::new(&config)?;
        let shell = Shell::new();
        let help = Help::new(config.goal);

        let tools = vec![
            shell.get_tool_spec(),
            help.get_tool_spec()
        ];

        let reqwest_client = get_reqwest_client()?;
        let mut chat = get_llm_chat(config.model_params.clone(), reqwest_client, tools)?;
        chat.set_system_prompt(prompts.get(0)?.into());

        Ok(Agent {
            shell,
            help,
            config,
            term,
            chat,
        })
    }

    /// Run agent.
    pub fn run(&mut self) -> Result<(), AppError> {
        self.term.print_into();

        let mut next_message = if let Some(first_message) = &self.config.message {
            let user_message = Message::text(Role::User, first_message.clone());

            let response = self.chat.get_inference(&[user_message], ToolChoice::Auto)
                .map_err(AppError::LibError);

            if let Some(msg) = self.process_response(response)? {
                msg
            } else {
                return Ok(());
            }
        } else if let Some(msg) = self.get_user_message()? {
            msg
        } else {
            return Ok(());
        };

        loop {
            let response = self.chat.get_inference(&[next_message], ToolChoice::Auto)
            .map_err(AppError::LibError);

            next_message = if let Some(message) = self.process_response(response)? {
                message
            } else {
                break;
            }
        }

        Ok(())
    }

    fn get_user_message(&mut self) -> Result<Option<Message>, AppError> {
        loop {
            let user_input = self.term.user_input();

            if let Ok(user_msg) = user_input {
                let user_msg = user_msg.trim();
                if !user_msg.is_empty() {
                    if user_msg == "?" {
                        self.term.print_help();
                    } else {
                        let msg = Message::text(Role::User, user_msg.trim().to_owned());
                        return Ok(Some(msg));
                    }
                }
            } else if self.process_user_input_errors(user_input.unwrap_err())? {
                return Ok(None);
            }
        }
    }

    fn process_response(&mut self, response: Result<Vec<Message>, AppError>) -> Result<Option<Message>, AppError> {
        if let Ok(results) = response {
            if results.len() > 1 || results.is_empty() {
                let mut tool_msg = None;
                for message in results.iter() {
                    match message {
                        Message::Text(text) => { 
                            self.term.apprentice_print(&text.message);
                        },
                        Message::ToolCall(tool_call) => {
                            if tool_msg.replace(tool_call).is_some() {
                                return Err(AppError::ApplicationError("Unexpected LLM response: parallel tool call is requested."))
                            }
                        },
                        Message::ToolResult(_) => {
                            return Err(AppError::ApplicationError("Unexpected \"tool result\" message from LLM."))
                        }
                    }
                }

                if let Some(tool_call) = tool_msg {
                    self.process_tool_call(tool_call)
                } else {
                    self.get_user_message()
                }
                
            } else {
                let message = &results[0];
                match message {
                    Message::Text(text) => { 
                        self.term.apprentice_print(&text.message);
                        self.get_user_message()
                    }
                    Message::ToolCall(tool_call) => {
                        self.process_tool_call(tool_call)
                    }
                    Message::ToolResult(_) => {
                        Err(AppError::ApplicationError("Unexpected message type from the LLM."))
                    }
                }
            }
        } else if let Err(AppError::LibError(llmerr)) = response {
            if let Error::LLMErrorMessage(msg) = llmerr {
                self.term.apprentice_print(&format!("{}", AppError::LibError(Error::LLMErrorMessage(msg))));
            } else if let Error::LLMCallError(msg) = llmerr {
                self.term.apprentice_print(&format!("{}", AppError::LibError(Error::LLMCallError(msg))));
            }
            self.get_user_message()
        } else {
            Err(response.err().unwrap())
        }
    }

    fn process_tool_call(&mut self, tool_call: &ToolCall) -> Result<Option<Message>, AppError> {
        let tool_result = if tool_call.name == "SHELL" {
            match self.shell.call_tool(&tool_call.params, &mut self.term) {
                Ok(result) => result,
                Err(err) => return Err(err),
            }
        } else if tool_call.name == "HELP" {
            match self.help.call_tool(&tool_call.params) {
                Ok(result) => result,
                Err(err) => return Err(err),
            }
        } else {
            format!("Unknown tool \"{}\" was requested.", tool_call.name)
        };

        Ok(Some(Message::tool_result(
            tool_call.call_id.clone(), 
            tool_call.name.clone(), 
            tool_result)))
    }

    fn process_user_input_errors(&self, err: AppError) -> Result<bool, AppError> {
        match err {
            AppError::Rustyline(re) => {
                match re {
                    ReadlineError::Interrupted | ReadlineError::Eof => Ok(true),
                    _ => Err(AppError::Rustyline(re))
                }                
            },
            _ => Err(err)
        }
    }
}
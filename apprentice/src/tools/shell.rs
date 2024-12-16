use apprentice_lib::tools::{ParamType, ToolParam, ToolSpec};
use apprentice_lib::llm::ToolParam as InputParam;
use crate::error::AppError;
use crate::term::Term;
use crate::util::exec_pipe;

/// Ask user something.
pub struct Shell {}

impl Shell {

    /// Return tool specification.
    pub fn get_tool_spec(&self) -> ToolSpec {
        let mut description = if cfg!(target_os = "windows") {
            "Executes an arbitrary command in Windows shell (cmd) environment and returns its stdout and stderr."
        } else {
            "Executes an arbitrary command in a Unix/Linux shell (sh) environment and returns its stdout and stderr."
        }.to_owned();

        description += " User may cancel execution of the command and will provide reason.";

        ToolSpec {
            name: "SHELL".to_owned(),
            description,
            params: vec![
                ToolParam {
                    name: "command".to_string(), 
                    description: "command to execute".to_string(), 
                    data_type: ParamType::String, 
                    required: true
                }
            ]
        }
    }

    /// Create an instance.
    pub fn new() -> Self {
        Shell {}
    }

    /// Ask user and get reply.
    pub fn exec(&self, command: &str, term: &mut Term) -> Result<String, AppError> {
        term.print_tool_message("SHELL", command);

        loop {
            let user_input = term.tool_input("SHELL", "Execute command? (y - yes / n - no): ")?;
            let user_input = user_input.trim();

            if user_input.len() == 1 {
                let ret = match user_input {
                    "y" => {
                        term.begin_tool_format();
                        let ret = exec_pipe(command);
                        term.end_tool_format();
                        ret
                    },
                    "n" => {
                        let reason = term.tool_input("SHELL", "reason: ")?;
                        Ok(format!("User cancelled the operation with the reason: {}", reason))
                    },
                    _ => continue
                };

                return ret
            }
        }
    }

    pub fn call_tool(&self, params: &[InputParam], term: &mut Term) -> Result<String, AppError> {
        if params.len() == 1 {
            let param = &params[0];
            if param.name == "command" {
                if let Some(command) = param.value.as_str() {
                    self.exec(command, term)
                } else {
                    Ok("wrong parameter value type, expect 1 parameter called \"command\" of type string.".to_owned())
                }
            } else {
                Ok("wrong parameter name, expect 1 parameter called \"command\" of type string.".to_owned())
            }
        } else {
            Ok("wrong number of input parameters, expect 1 parameter called \"command\" of type string.".to_owned())
        }
    }

}
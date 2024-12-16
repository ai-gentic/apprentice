use apprentice_lib::tools::{ParamType, ToolParam, ToolSpec};
use apprentice_lib::llm::ToolParam as InputParam;
use crate::config::Goal;
use crate::error::AppError;
use crate::util::exec_pipe;

/// Ask user something.
pub struct Help {
    goal: Goal,
}

impl Help {

    /// Return tool specification.
    pub fn get_tool_spec(&self) -> ToolSpec {
        let description = 
            "Returns a help page for a specific CLI tool subcommand.".to_owned();

        ToolSpec {
            name: "HELP".to_owned(),
            description,
            params: vec![
                ToolParam {
                    name: "command".to_string(), 
                    description: "command for which the help is required".to_string(), 
                    data_type: ParamType::String, 
                    required: true
                }
            ]
        }
    }

    /// Create an instance.
    pub fn new(goal: Goal) -> Self {
        Help {
            goal,
        }
    }

    /// Check params and execute tool.
    pub fn call_tool(&self, params: &[InputParam]) -> Result<String, AppError> {
        if params.len() == 1 {
            let param = &params[0];
            if param.name == "command" {
                if let Some(command) = param.value.as_str() {
                    let full_cmd = match self.goal {
                        Goal::Gcp => {
                            if command.starts_with("gcloud ") || command.starts_with("bq ") || command.starts_with("gsutil ") {
                                command.to_owned() + " --help"
                            } else {
                                return Ok("command must start with \"gcloud \",  \"bq  \", or  \"gsutil  \".".to_owned());
                            }
                        },
                        Goal::Aws => if command.starts_with("aws ") {
                            command.to_owned() + " help"
                        } else {
                            return Ok("command must start with \"aws \".".to_owned());
                        }
                        Goal::Azure => if command.starts_with("az ") {
                            command.to_owned() + " --help"
                        } else {
                            return Ok("command must start with \"az \".".to_owned());
                        }
                    };
                    exec_pipe(&full_cmd)
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
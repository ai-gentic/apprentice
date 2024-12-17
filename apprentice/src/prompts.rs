use crate::{config::Goal, error::AppError};

const PROMPTS: [&str;3] = [
"You are an assistant called \"Apprentice\" that helps translate a user request into a valid call to the ",
".
You are in dialogue with the user. 
After each response from the user, you think and ALWAYS do one of the following actions:
1. Produce the resulting command (use the SHELL tool).
2. Ask the user a clarifying question.
3. Request help page for a specific subcommand (use HELP tool).
4. Reject the user request and specify the reason why it cannot be fulfilled.
The user can ask questions. You understand from the context that the user is asking a question and not giving you an answer, then you are doing one of the actions defined above.
You form your resulting command based on the information from your dialogue with the user.
You reflect in the resulting command ALL that the user specifid in the request and important/common attributes, even if the user did not specify them in the request.

",
"

Below is your actual dialogue with the user."
];

/// System prompts.
pub struct Prompts {
    prompts: Vec<String>,
}

impl Prompts {

    /// Create a new instance.
    pub fn new(sys_add: &Option<String>, goal: Goal) -> Self {
        let mut sys = PROMPTS[0].to_owned();

        sys += match goal {
            Goal::Gcp => "Google Cloud CLI tools gcloud, bq, gsutil",
            Goal::Aws => "AWS CLI aws",
            Goal::Azure => "Azure CLI az",
        };

        sys += PROMPTS[1];

        if let Some(instr) = sys_add {
            sys += "In addition, consider using the following information from the user:\n-----\n";
            sys += instr;
            sys += "\n-----";
        }

        sys += PROMPTS[2];

        Prompts {
            prompts: vec![sys],
        }
    }

    /// Get prompt by id.
    pub fn get(&self, id: usize) -> Result<&str, AppError> {
        let len = if self.prompts.is_empty() {PROMPTS.len()} else {self.prompts.len()};

        if len <= id {
            return Err(AppError::ConfigParseError("requested prompt does not exist."));
        }

        Ok(&self.prompts[id])
    }
}
use thiserror::Error as ThisError;

/// App errors.
#[derive(ThisError, Debug)]
pub enum Error {
    /// Missing arguments.
    #[error("Missing mandatory arguments: {0}\nTry `apprentice --help` for more information.")]
    MissingArgError(&'static str),

    /// LLM call error.
    #[error("Failed to call LLM: {0}")]
    LLMCallError(#[from] reqwest::Error),

    /// LLM call error.
    #[error("Failed to process LLM call: {0}")]
    LLMJsonError(#[from] serde_json::Error),

    /// LLM call error.
    #[error("Failed to parse LLM response: {0}")]
    LLMResponseError(&'static str),

    /// General error.
    #[error("{0}")]
    Error(String),

    /// LLM response error message.
    #[error("LLM provider responded with error: {0}")]
    LLMErrorMessage(String),

    /// LLM response error message.
    #[cfg(test)]
    #[error("Test error: {0}")]
    ForTests(&'static str),
}
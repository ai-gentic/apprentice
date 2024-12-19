use thiserror::Error;

/// App errors
#[derive(Error, Debug)]
pub enum AppError {

    /// Toml parsing error
    #[error("Failed to parse config file: {0}")]
    TomlError(#[from] toml::de::Error),

    /// Config parsing error
    #[error("Failed to parse config file: {0}")]
    ConfigParseError(&'static str),

    /// Missing arguments
    #[error("Missing mandatory arguments: {0}\nTry `apprentice --help` for more information.")]
    MissingArgError(&'static str),

    /// Missing arguments
    #[error("Incorrect argument value: {0}")]
    InvalidArgError(&'static str),

    /// Missing arguments
    #[error("Incorrect argument value: {0}")]
    LibError(#[from] apprentice_lib::Error),

    /// Other errors
    #[error("Reading user input: {0}")]
    Rustyline(#[from] rustyline::error::ReadlineError),

    /// Unknown/unexpected error
    #[error("Unknown error")]
    Unknown,

    /// Config parsing error
    #[error("The format of the color value is incorrect")]
    ColorParseError,

    /// Embedding with candle error.
    #[error("Candle core error: {0}")]
    CandleCoreError(#[from] candle_core::Error),

    /// Hf API error.
    #[error("Huggingface hub API call: {0}")]
    HfApiCall(#[from] hf_hub::api::sync::ApiError),

    /// Application logic error.
    #[error("Application error: {0}")]
    ApplicationError(&'static str),

    /// General error.
    #[error("{0}")]
    Error(String),
}
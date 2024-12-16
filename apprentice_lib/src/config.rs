use crate::error::Error;

/// Model providers.
#[derive(Debug, Clone, Copy)]
pub enum ModelProvider {
    /// Open AI.
    OpenAI,
    /// Anthropic.
    Anthropic,
    /// GCP.
    GCP,
}

impl TryFrom<&str> for ModelProvider {
    type Error = Error;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        match val {
            "openai" => Ok(ModelProvider::OpenAI),
            "anthropic" => Ok(ModelProvider::Anthropic),
            "gcp" => Ok(ModelProvider::GCP),
            _ => Err(Error::Error(format!("unknown provider: {val}"))),
        }
    }
}

/// Model parameters.
#[derive(Clone, Debug)]
pub struct Config {
    /// Model name.
    pub provider: ModelProvider,
    /// Model name.
    pub name: String,
    /// API key.
    pub api_key: String,
    /// Model API URL.
    pub api_url: String,
    /// Model API version.
    pub api_version: Option<String>,
    /// Maximum number of tokens that will be generated.
    pub max_tokens: Option<i64>,
    /// Number of variants to generate.
    pub n: Option<i64>,
    /// Level of randomization when choosing tokens.
    pub temperature: Option<f64>,
    /// Only the tokens comprising the top_p probability mass will be considered.
    pub top_p: Option<f64>,
    /// Only k tokens with the most probability will be considered.
    pub top_k: Option<i64>,
    /// Penalize new tokens based on their existing frequency.
    pub frequency_penalty: Option<f64>,
    /// Penalize new tokens based on whether they appear in the text so far.
    pub presence_penalty: Option<f64>,
    /// Sequences at which model will stop generating.
    pub stop_sequence: Option<String>,
}


impl Config {

    /// Create minimal config using provider, model name, API key, and API URL.
    pub fn new(provider: ModelProvider, name: String, api_key: String, api_url: String) -> Self {
        Config {
            provider,
            name,
            api_key,
            api_url,
            api_version: None,
            max_tokens: None,
            n: None,
            temperature: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequence: None
        }
    }
}
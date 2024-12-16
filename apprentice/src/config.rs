use apprentice_lib::Config as ModelParams;

use crate::{error::AppError, options::Options, util::api_url_for_provider};

/// Goal the agent will pursue
#[derive(Debug, Clone, Copy)]
pub enum Goal {
    /// Google cloud platform
    Gcp,
    /// Amazon web services
    Aws,
    /// Microsoft Azure
    Azure,
}

impl TryFrom<&str> for Goal {
    type Error = AppError;

    fn try_from(val: &str) -> Result<Self, Self::Error> {
        match val {
            "gcp" => Ok(Goal::Gcp),
            "aws" => Ok(Goal::Aws),
            "azure" => Ok(Goal::Azure),
            _ => Err(AppError::ConfigParseError("unknown goal")),
        }
    }
}

/// Application settings.
#[derive(Clone, Debug)]
pub struct Settings {
    /// User messages color.
    pub user_color: (Option<[u8;3]>, Option<[u8;3]>),
    /// Apprentice messages color.
    pub apprentice_color: (Option<[u8;3]>, Option<[u8;3]>),
    /// Tool stdout and stderr output color.
    pub tool_color: (Option<[u8;3]>, Option<[u8;3]>),
}

/// App config
#[derive(Clone, Debug)]
pub struct Config {
    /// Agent goal
    pub goal: Goal,
    /// Model name
    pub model_params: ModelParams,
    /// Message
    pub message: Option<String>,
    /// Settings
    pub settings: Settings,
    /// Custom instructions to add to system prompt.
    pub prompt: Option<String>,
}

impl TryFrom<Options> for Config {
    type Error = AppError;

    fn try_from(options: Options) -> Result<Self, AppError> {
        let model = options.model.unwrap();
        let provider = options.model_provider.unwrap().as_str().try_into()?;
        let default_url = api_url_for_provider(provider, &model);

        let model_params = ModelParams {
            provider,
            name: model.clone(),
            api_key: options.api_key.unwrap(),
            api_url: options.api_url.unwrap_or(default_url),
            api_version: options.api_version,
            max_tokens: options.max_tokens,
            n: options.n,
            temperature: options.temperature,
            top_p: options.top_p,
            top_k: options.top_k,
            frequency_penalty: options.frequency_penalty,
            presence_penalty: options.presence_penalty,
            stop_sequence: options.stop_sequence,
        };

        let settings = Settings {
            user_color: options.user_color,
            apprentice_color: options.apprentice_color,
            tool_color: options.tool_color,
        };

        Ok(Config {
            goal: options.goal.unwrap().as_str().try_into()?,
            model_params,
            message: options.message,
            settings,
            prompt: options.prompt,
        })
    }
}

#[cfg(test)]
mod test {
    use apprentice_lib::ModelProvider;

    use super::*;

    #[test]
    fn test_config_try_from() {
        let mut options = Options {
            goal: Some("aws".into()),
            model_provider: Some("anthropic".into()),
            model: Some("mdl".into()),
            api_key: Some("apk".into()),
            api_url: Some("apr".into()),
            api_version: Some("apv".into()),
            max_tokens: Some(1024),
            n: Some(34),
            temperature: Some(7.44),
            top_p: Some(0.94),
            top_k: Some(7),
            frequency_penalty: Some(0.222),
            presence_penalty: Some(0.111),
            stop_sequence: Some("ssq".into()),
            message: Some("msg".into()),
            user_color: (Some([255,0,123]), Some([0,123,255])),
            apprentice_color: (Some([255,0,124]), Some([0,124,255])),
            tool_color: (Some([255,0,125]), Some([0,125,255])),
            prompt: Some("prm".into()),
        };

        let config = Config::try_from(options.clone()).expect("create from options");

        assert!(matches!(config.goal, Goal::Aws));
        assert_eq!(config.message, Some("msg".into()));
        assert_eq!(config.prompt, Some("prm".into()));
        assert!(matches!(config.model_params.provider, ModelProvider::Anthropic));
        assert_eq!(config.model_params.name, "mdl".to_owned());
        assert_eq!(config.model_params.api_key, "apk".to_owned());
        assert_eq!(config.model_params.api_url, "apr".to_owned());
        assert_eq!(config.model_params.api_version, Some("apv".into()));
        assert_eq!(config.model_params.max_tokens, Some(1024));
        assert_eq!(config.model_params.n, Some(34));
        assert_eq!(config.model_params.temperature, Some(7.44));
        assert_eq!(config.model_params.top_p, Some(0.94));
        assert_eq!(config.model_params.top_k, Some(7));
        assert_eq!(config.model_params.frequency_penalty, Some(0.222));
        assert_eq!(config.model_params.presence_penalty, Some(0.111));
        assert_eq!(config.model_params.stop_sequence, Some("ssq".into()));

        assert_eq!(config.settings.user_color, (Some([255,0,123]), Some([0,123,255])));
        assert_eq!(config.settings.apprentice_color, (Some([255,0,124]), Some([0,124,255])));
        assert_eq!(config.settings.tool_color, (Some([255,0,125]), Some([0,125,255])));

        options.api_url = None;

        let config = Config::try_from(options.clone()).expect("create from options");
        assert_eq!(config.model_params.api_url, "https://api.anthropic.com/v1/messages");

        options.model_provider = Some("gcp".into());

        let config = Config::try_from(options.clone()).expect("create from options");
        assert_eq!(config.model_params.api_url, "https://generativelanguage.googleapis.com/v1beta/models/mdl:generateContent");

        options.model_provider = Some("openai".into());

        let config = Config::try_from(options.clone()).expect("create from options");
        assert_eq!(config.model_params.api_url, "https://api.openai.com/v1/chat/completions");
    }
}
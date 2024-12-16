//! App initialization functions.

use anstyle::Style;
use clap::Arg;
use clap::ArgMatches;
use clap::Command;
use std::ffi::OsString;
use std::str::FromStr;
use crate::error::AppError;
use crate::toml_parser::parse_toml_config;
use dirs::home_dir;
use crate::util::parse_colors;

/// App options.
#[derive(Debug, Clone)]
pub struct Options {
    /// Goal.
    pub goal: Option<String>,
    /// Model provider.
    pub model_provider: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// API key.
    pub api_key: Option<String>,
    /// Model API URL.
    pub api_url: Option<String>,
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
    /// Sequence at which model will stop generating.
    pub stop_sequence: Option<String>,
    /// User message.
    pub message: Option<String>,
    /// User message color and prompt background.
    pub user_color: (Option<[u8;3]>, Option<[u8;3]>),
    /// Apprentice message color and prompt background.
    pub apprentice_color: (Option<[u8;3]>, Option<[u8;3]>),
    /// Apprentice message color and prompt background.
    pub tool_color: (Option<[u8;3]>, Option<[u8;3]>),
    /// Custom instructions to add to system prompt.
    pub prompt: Option<String>,
}


macro_rules! check_and_set_float_arg {
    ($arg:literal, $m:ident, $option:expr) => {
        if let Some(x) = $m.get_one::<String>($arg) {
            if let Ok(val) = f64::from_str(x) {
                $option.replace(val);  
            } else {
                return Err(AppError::InvalidArgError(concat!($arg, " must be floating point number")));
            }
        }
    }
}

macro_rules! check_and_set_color_arg {
    ($arg:literal, $m:ident, $option:expr) => {
        if let Some(x) = $m.get_one::<String>($arg) {
            if let Ok(colors) = parse_colors(&x) {
                $option = colors;
            } else {
                return Err(AppError::InvalidArgError(
                    concat!($arg, " must have valid format, e.g. 'fg(255,0,123);bg(0,123,255)'.")
                ));
            }
        }
    }
}

impl Options {

    /// Create new unfilled options.
    pub fn new() -> Self {
        Options {
            goal: None,
            model_provider: None,
            model: None,
            api_key: None,
            api_url: None,
            api_version: None,
            max_tokens: None,
            n: None,
            temperature: None,
            top_p: None,
            top_k: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop_sequence: None,
            message: None,
            user_color: (None, None),
            apprentice_color: (None, None),
            tool_color: (None, None),
            prompt: None,
        }
    }
    
    fn argument_parser<T>(args: impl IntoIterator<Item = T>) -> ArgMatches where T: Into<OsString> + Clone {
        let bold_underline = Style::new().underline().bold();
        let bold = Style::new().bold();

        Command::new("Apprentice")
            .about("Apprentice is an assistant tool that helps to translate and execute human commands using cloud CLI tools.")
            .version(env!("CARGO_PKG_VERSION"))
            .arg(
                Arg::new("goal")
                .long("goal")
                .help("One of: gcp, aws, azure")
                .short('g')
                .env("APPRENTICE_GOAL")
                .required(false)
            ).arg(
                Arg::new("model")
                .long("model")
                .help("Inference model name")
                .short('m')
                .env("APPRENTICE_MODEL")
                .required(false)
            ).arg(
                Arg::new("model-provider")
                .long("model-provider")
                .help("Model provider, one of: openai, anthropic, gcp, azure, custom")
                .short('p')
                .env("APPRENTICE_MODEL_PROVIDER")
                .required(false)
            ).arg(
                Arg::new("api-key")
                .long("api-key")
                .help("LLM model API key")
                .short('k')
                .env("APPRENTICE_API_KEY")
                .required(false)
            ).arg(
                Arg::new("api-url")
                .long("api-url")
                .help("Model API URL")
                .short('u')
                .env("APPRENTICE_API_URL")
                .required(false)
            ).arg(
                Arg::new("config")
                .long("config")
                .help("Config file path")
                .short('c')
                .env("APPRENTICE_CONFIG")
                .required(false)
            ).arg(
                Arg::new("message")
                .long("message")
                .help("User's request")
                .short('e')
                .env("APPRENTICE_MESSAGE")
                .required(false)
            ).arg(
                Arg::new("api-version")
                .long("api-version")
                .help("Model API version")
                .env("APPRENTICE_API_VERSION")
                .required(false)
            ).arg(
                Arg::new("max-tokens")
                .long("max-tokens")
                .help("Maximum number of tokens that will be generated")
                .env("APPRENTICE_MAX_TOKENS")
                .required(false)
            ).arg(
                Arg::new("n")
                .long("n")
                .help("Number of variants to generate per one LLM call")
                .env("APPRENTICE_N")
                .required(false)
            ).arg(
                Arg::new("temperature")
                .long("temperature")
                .help("Level of randomization when LLM choose tokens")
                .env("APPRENTICE_TEMPERATURE")
                .required(false)
            ).arg(
                Arg::new("top-p")
                .long("top-p")
                .help("Only the tokens comprising the top_p probability mass will be considered")
                .env("APPRENTICE_TOP_P")
                .required(false)
            ).arg(
                Arg::new("top-k")
                .long("top-k")
                .help("Only k tokens with the most probability will be considered")
                .env("APPRENTICE_TOP_K")
                .required(false)
            ).arg(
                Arg::new("frequency-penalty")
                .long("frequency-penalty")
                .help("Penalize new tokens based on their existing frequency")
                .env("APPRENTICE_FREQUENCY_PENALTY")
                .required(false)
            ).arg(
                Arg::new("presence-penalty")
                .long("presence-penalty")
                .help("Penalize new tokens based on whether they appear in the text so far")
                .env("APPRENTICE_PRESENCE_PENALTY")
                .required(false)
            ).arg(
                Arg::new("stop-sequence")
                .long("stop-sequence")
                .help("Sequence at which model will stop generating")
                .env("APPRENTICE_STOP_SEQUENCE")
                .required(false)
            ).arg(
                Arg::new("prompt")
                .long("prompt")
                .help("Custom instructions to use in the system prompt.")
                .env("APPRENTICE_PROMPT")
                .required(false)
            ).arg(
                Arg::new("apprentice-color")
                .long("apprentice-color")
                .help("Apprentice messages and prompt background colors, rgb (e.g. 'fg(255,0,123);bg(0,123,255)').")
                .env("APPRENTICE_APPRENTICE_COLOR")
                .required(false)
            ).arg(
                Arg::new("user-color")
                .long("user-color")
                .help("User messages and prompt background colors, rgb (e.g. 'fg(255,0,123);bg(0,123,255)').")
                .env("APPRENTICE_USER_COLOR")
                .required(false)
            ).arg(
                Arg::new("tool-color")
                .long("tool-color")
                .help("Tool stdout and stderr and prompt background colors, rgb (e.g. 'fg(255,0,123);bg(0,123,255)').")
                .env("APPRENTICE_TOOL_COLOR")
                .required(false)
            )
            .after_help(format!("{bold_underline}Example:{bold_underline:#} {bold}

    apprentice --goal=gcp --model=gemini-1.5-pro-002 --model-provider=gcp --api-key=<your-key> --message='Create a VM instance with 4 CPU cores, 16GB RAM, 100GB disk, Debian OS, public IP address'{bold:#}

To start using the application you need to specify at least goal (--goal), API provider (--model-provider), model name (--model), and API key (--api-key).
Apprentice uses the configuration file .apprentice.toml from user's home directory, or the one specified with -c option (see the sample_config.toml for the reference).
If it finds the configuration file it uses configuration options from the file.
The configuration options can be overridden with the command line arguments or environment variables."))
            .get_matches_from(args)
    }

    fn load_config_file(path: Option<&str>) -> Result<Option<String>, std::io::Error> {
        Ok(if let Some(p) = path {
            Some(std::fs::read_to_string(p)?)
        } else if let Some(mut p) = home_dir() {
            p.push(".apprentice.toml");
            if std::fs::exists(p.as_path())? {
                Some(std::fs::read_to_string(p.as_path())?)
            } else {
                None
            }
        } else {
            None
        })
    }

    fn validate_mandatory_options(options: &Options) -> Result<(), AppError> {
        if options.goal.is_none() {
            return Err(AppError::MissingArgError("goal is not specified."));
        }
        if options.model.is_none() {
            return Err(AppError::MissingArgError("inference model is not specified."));
        }
        if options.model_provider.is_none() {
            return Err(AppError::MissingArgError("model provider is not specified."));
        }
        if options.api_key.is_none() {
            return Err(AppError::MissingArgError("API key is not specified."));
        }
        if let Some(n) = options.n {
            if n != 1 {
                return Err(AppError::InvalidArgError("Currently only n=1 is uspported."));
            }
        }

        Ok(())
    }

    /// Load and validate options from env, command line arguments, config file.
    pub fn load<T>(args: impl IntoIterator<Item = T>) -> Result<Self, AppError> 
        where T: Into<OsString> + Clone 
    {
        let m = Self::argument_parser(args);

        let mut options = Options::new();

        let config_path = m.get_one("config").map(|s: &String| s.as_ref());

        if let Some(content) = Self::load_config_file(config_path)
            .map_err(|err| AppError::Error(format!("Error loading config file: {}", err)))?
        {
            parse_toml_config(&content, &mut options)?;
        }

        if let Some(x) = m.get_one::<String>("goal") {
            options.goal.replace(x.as_str().to_owned());
        }
        if let Some(x) = m.get_one::<String>("model") {
            options.model.replace(x.clone());
        }
        if let Some(x) = m.get_one::<String>("model-provider") {
            options.model_provider.replace(x.clone());
        }
        if let Some(x) = m.get_one::<String>("api-key") {
            options.api_key.replace(x.clone());
        }
        if let Some(x) = m.get_one::<String>("api-url") {
            options.api_url.replace(x.clone());
        }
        if let Some(x) = m.get_one::<String>("api-version") {
            options.api_version.replace(x.clone());
        }
        if let Some(x) = m.get_one::<String>("max-tokens") {
            if let Ok(val) = x.parse::<i64>() {
                if val < 0 { return Err(AppError::InvalidArgError("max-tokens must be non-negative")) };
                options.max_tokens.replace(val);
            } else {
                return Err(AppError::InvalidArgError("max-tokens must be integer"));
            }
        }
        if let Some(x) = m.get_one::<String>("n") {
            if let Ok(val) = x.parse::<i64>() {
                if val <= 0 { return Err(AppError::InvalidArgError("n must be greater than zero")) };
                options.n.replace(val);
            } else {
                return Err(AppError::InvalidArgError("n must be integer"));
            }
        }
        if let Some(x) = m.get_one::<String>("top-k") {
            if let Ok(val) = x.parse::<i64>() {
                if val <= 0 { return Err(AppError::InvalidArgError("top-k must be greater than zero")) };
                options.top_k.replace(val);
            } else {
                return Err(AppError::InvalidArgError("top-k must be integer"));
            }
        }

        check_and_set_float_arg!("temperature", m, options.temperature);
        check_and_set_float_arg!("top-p", m, options.top_p);
        check_and_set_float_arg!("frequency-penalty", m, options.frequency_penalty);
        check_and_set_float_arg!("presence-penalty", m, options.presence_penalty);

        if let Some(x) = m.get_one::<String>("stop-sequence") {
            options.stop_sequence.replace(x.clone());
        }

        if let Some(x) = m.get_one::<String>("prompt") {
            options.prompt.replace(x.clone());
        }

        check_and_set_color_arg!("apprentice-color", m, options.apprentice_color);
        check_and_set_color_arg!("user-color", m, options.user_color);
        check_and_set_color_arg!("tool-color", m, options.tool_color);

        options.message = m.get_one::<String>("message").cloned();

        Self::validate_mandatory_options(&options)?;

        Ok(options)
    }
}


#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_load_options() {
 
        let mut args = vec![
            OsString::from("/bin/path"),
            OsString::from("--goal=<goal>"),
            OsString::from("--model=<model>"),
            OsString::from("--model-provider=<model-provider>"),
            OsString::from("--api-key=<api-key>"),
            OsString::from("--api-url=<api-url>"),
            OsString::from("--message=<message>"),
            OsString::from("--api-version=<api-version>"),
            OsString::from("--max-tokens=789"),
            OsString::from("--n=1"),
            OsString::from("--temperature=0.456"),
            OsString::from("--top-p=0.123"),
            OsString::from("--top-k=123"),
            OsString::from("--frequency-penalty=1.234"),
            OsString::from("--presence-penalty=2.345"),
            OsString::from("--stop-sequence=<stop-sequence>"),
            OsString::from("--prompt=<prompt>"),
            OsString::from("--apprentice-color=fg(255,0,124);bg(0,124,255)"),
            OsString::from("--user-color='fg(255,0,125);bg(0,125,255)'"),
            OsString::from("--tool-color=\"fg(255,0,123);bg(0,123,255)\""),
        ];

        let options = Options::load(args.clone()).expect("load options");

        assert_eq!(options.goal, Some("<goal>".into()));
        assert_eq!(options.model_provider, Some("<model-provider>".into()));
        assert_eq!(options.model, Some("<model>".into()));
        assert_eq!(options.api_key, Some("<api-key>".into()));
        assert_eq!(options.api_url, Some("<api-url>".into()));
        assert_eq!(options.api_version, Some("<api-version>".into()));
        assert_eq!(options.max_tokens, Some(789));
        assert_eq!(options.n, Some(1));
        assert_eq!(options.temperature, Some(0.456));
        assert_eq!(options.top_p, Some(0.123));
        assert_eq!(options.top_k, Some(123));
        assert_eq!(options.frequency_penalty, Some(1.234));
        assert_eq!(options.presence_penalty, Some(2.345));
        assert_eq!(options.stop_sequence, Some("<stop-sequence>".into()));
        assert_eq!(options.message, Some("<message>".into()));
        assert_eq!(options.apprentice_color, (Some([255,0,124]), Some([0,124,255])));
        assert_eq!(options.user_color, (Some([255,0,125]), Some([0,125,255])));
        assert_eq!(options.tool_color, (Some([255,0,123]), Some([0,123,255])));
        assert_eq!(options.prompt, Some("<prompt>".into()));

        let mut args2 = args.clone();
        args2.remove(1);
        assert!(matches!(Options::load(args2), Err(AppError::MissingArgError(_))));

        let mut args2 = args.clone();
        args2.remove(2);
        assert!(matches!(Options::load(args2), Err(AppError::MissingArgError(_))));

        let mut args2 = args.clone();
        args2.remove(3);
        assert!(matches!(Options::load(args2), Err(AppError::MissingArgError(_))));

        let mut args2 = args.clone();
        args2.remove(4);
        assert!(matches!(Options::load(args2), Err(AppError::MissingArgError(_))));

        args[9] = "--n=2".into();
        assert!(matches!(Options::load(args), Err(AppError::InvalidArgError(_))));

    }
}
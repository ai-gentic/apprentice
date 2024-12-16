use toml::Table;
use toml::Value;
use crate::options::Options;
use crate::error::AppError;
use crate::util::parse_colors;

fn get_str_val<'a>(val: &'a Value, err: &'static str) -> Result<&'a str, AppError> {
    if !val.is_str() {
        return Err(AppError::ConfigParseError(err));
    }
    val.as_str().ok_or(AppError::Unknown)
}

fn get_int_val(val: &Value, err: &'static str) -> Result<i64, AppError> {
    if !val.is_integer() {
        return Err(AppError::ConfigParseError(err));
    }
    val.as_integer().ok_or(AppError::Unknown)
}

fn get_float_val(val: &Value, err: &'static str) -> Result<f64, AppError> {
    if !val.is_float() {
        return Err(AppError::ConfigParseError(err));
    }
    val.as_float().ok_or(AppError::Unknown)
}

fn get_color_val(val: &Value, err: &'static str) -> Result<(Option<[u8;3]>, Option<[u8;3]>), AppError> {
    let s = get_str_val(val, err)?;
    parse_colors(s).map_err(|_| AppError::ConfigParseError(err))
}

pub fn parse_toml_config(content: &str, options: &mut Options) -> Result<(), AppError> {

    let toml_config: Table = toml::from_str(content)?;

    if let Some(default_context) = toml_config.get("default_context") {

        let context_name = get_str_val(default_context, "default_context must be a string value")?;

        let context_value = toml_config.get(context_name)
            .ok_or(AppError::ConfigParseError("configuration for the default context is not specified"))?;

        let ct = context_value.as_table().ok_or(AppError::Unknown)?;

        if let Some(val) = ct.get("goal") {
            options.goal.replace(get_str_val(val, "goal must be a string value")?.to_owned());
        }

        if let Some(val) = ct.get("model") {
            options.model.replace(get_str_val(val,"model must be a string value")?.to_owned());
        }

        if let Some(val) = ct.get("model_provider") {
            options.model_provider.replace(get_str_val(val, "model_provider must be a string value")?.to_owned());
        }

        if let Some(val) = ct.get("api_key") {
            options.api_key.replace(get_str_val(val,"api_key must be a string value")?.to_owned());
        }
        
        if let Some(val) = ct.get("api_url") {
            options.api_url.replace(get_str_val(val,"api_url must be a string value")?.to_owned());
        }
        
        if let Some(val) = ct.get("api_version") {
            options.api_version.replace(get_str_val(val,"api_version must be a string value")?.to_owned());
        }

        if let Some(val) = ct.get("max_tokens") {
            options.max_tokens.replace(get_int_val(val,"max_tokens must be an integer value")?);
        }

        if let Some(val) = ct.get("n") {
            options.n.replace(get_int_val(val,"n must be an integer value")?);
        }

        if let Some(val) = ct.get("temperature") {
            options.temperature.replace(get_float_val(val,"temperature must be a float value")?);
        }

        if let Some(val) = ct.get("top_p") {
            options.top_p.replace(get_float_val(val,"top_p must be a float value")?);
        }

        if let Some(val) = ct.get("top_k") {
            options.top_k.replace(get_int_val(val,"top_k must be an integer value")?);
        }

        if let Some(val) = ct.get("frequency_penalty") {
            options.frequency_penalty.replace(get_float_val(val,"frequency_penalty must be a float value")?);
        }

        if let Some(val) = ct.get("presence_penalty") {
            options.presence_penalty.replace(get_float_val(val,"presence_penalty must be a float value")?);
        }

        if let Some(val) = ct.get("stop_sequence") {
            options.stop_sequence.replace(get_str_val(val,"stop_sequence must be a string value")?.to_owned());
        }

        if let Some(val) = ct.get("prompt") {
            options.prompt.replace(get_str_val(val, "prompt must be a string value")?.to_owned());
        }
    }

    if let Some(settings_section) = toml_config.get("settings") {
        if let Some(settings) = settings_section.as_table() {
            if let Some(user_color) = settings.get("user_color") {
                options.user_color = get_color_val(user_color, "user_color value must have valid format, e.g. 'fg(255,0,123);bg(0,123,255)'.")?;
            }
            if let Some(apprentice_color) = settings.get("apprentice_color") {
                options.apprentice_color = get_color_val(apprentice_color, "apprentice_color value must have valid format, e.g. 'fg(255,0,123);bg(0,123,255)'.")?;
            }
            if let Some(tool_color) = settings.get("tool_color") {
                options.tool_color = get_color_val(tool_color, "tool_color value must have valid format, e.g. 'fg(255,0,123);bg(0,123,255)'.")?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_parser() {
        const SAMPLE_CONTENT: &str = "
default_context = \"google_cloud\"

# Context contains a set of configuration parameters for the agent
[google_cloud]
goal = \"gcp\"                # The cloud provider agent will work with, one of gcp, aws, or azure
model_provider = \"openai\"   # Model provider, one of: openai, anthropic, gcp
model = \"gpt-4\"             # Model name
api_url = \"https://api.openai.com/v1/chat/completions\"  # Model API URL
api_key = \"<your-api-key>\"  # Model API key
api_version = \"v1.1\"        # Other parameters (depending on provider some of the parameters may be required)
max_tokens = 8192
n = 4
temperature = 0.5
top_p = 1.0
top_k = 10
frequency_penalty = 2.0
presence_penalty = 3.0
stop_sequence = \"seq\"
prompt = \"sample_prompt\"

# Second context
[google_cloud_gemini]
goal = \"gcp\"
model_provider = \"gcp\"
model = \"gemini-1.5-pro-002\"
api_url = \"https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-pro-002:generateContent\"
api_key = \"<your-api-key>\"      
prompt = \"sample_prompt\"  
        
[settings]
user_color = \"fg(1,2,3);bg(4,5,6)\"
apprentice_color = \"fg(7,8,9);bg(10,11,12)\"
tool_color = \"fg(13,14,15);bg(16,17,18)\"
";

        let mut options = Options::new();
        assert!(parse_toml_config(SAMPLE_CONTENT, &mut options).is_ok());

        assert_eq!(options.goal, Some("gcp".into()));
        assert_eq!(options.model_provider, Some("openai".into()));
        assert_eq!(options.model, Some("gpt-4".into()));
        assert_eq!(options.api_key, Some("<your-api-key>".into()));
        assert_eq!(options.api_url, Some("https://api.openai.com/v1/chat/completions".into()));
        assert_eq!(options.api_version, Some("v1.1".into()));
        assert_eq!(options.max_tokens, Some(8192));
        assert_eq!(options.n, Some(4));
        assert_eq!(options.temperature, Some(0.5));
        assert_eq!(options.top_p, Some(1.0));
        assert_eq!(options.top_k, Some(10));
        assert_eq!(options.frequency_penalty, Some(2.0));
        assert_eq!(options.presence_penalty, Some(3.0));
        assert_eq!(options.stop_sequence, Some("seq".into()));
        assert_eq!(options.message, None);
        assert_eq!(options.apprentice_color, (Some([7,8,9]), Some([10,11,12])));
        assert_eq!(options.user_color, (Some([1,2,3]), Some([4,5,6])));
        assert_eq!(options.tool_color, (Some([13,14,15]), Some([16,17,18])));
        assert_eq!(options.prompt, Some("sample_prompt".into()));
    }
}
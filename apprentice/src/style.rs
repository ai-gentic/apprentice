//! Terminal styles.
use anstyle::Color;
use anstyle::RgbColor;
use anstyle::Style;
use crate::Config;

/// Styles.
pub struct Styles {
    /// User prompt style.
    pub user_prompt: Style,
    /// User prompt arrow style.
    pub user_prompt_arrow: Style,
    /// User message style.
    pub user_text: Style,
    /// Apprentice prompt style.
    pub apprentice_prompt: Style,
    /// Apprentice prompt arrow style.
    pub apprentice_prompt_arrow: Style,
    /// Apprentice message style.
    pub apprentice_text: Style,
    /// Tool prompt style.
    pub tool_prompt: Style,
    /// Tool prompt arrow style.
    pub tool_prompt_arrow: Style,
    /// Tool output style.
    pub tool_text: Style,
}

impl Styles {

    /// Load styles.
    pub fn new(config: &Config) -> Self {
        let mut fg_user_color = Color::Rgb(RgbColor(128, 64, 64));
        let mut fg_apprentice_color = Color::Rgb(RgbColor(64, 128, 64));
        let mut fg_tool_color = Color::Rgb(RgbColor(128, 128, 0));

        let mut bg_user_color = Color::Rgb(RgbColor(128, 0, 0));
        let mut bg_apprentice_color = Color::Rgb(RgbColor(0, 128, 0));
        let mut bg_tool_color = Color::Rgb(RgbColor(64, 64, 0));

        if let (Some([r1,g1,b1]), Some([r2,g2,b2])) = config.settings.user_color {
            fg_user_color = Color::Rgb(RgbColor(r1,g1,b1));
            bg_user_color = Color::Rgb(RgbColor(r2,g2,b2));
        }
        if let (Some([r1,g1,b1]), Some([r2,g2,b2])) = config.settings.apprentice_color {
            fg_apprentice_color = Color::Rgb(RgbColor(r1,g1,b1));
            bg_apprentice_color = Color::Rgb(RgbColor(r2,g2,b2));
        }
        if let (Some([r1,g1,b1]), Some([r2,g2,b2])) = config.settings.tool_color {
            fg_tool_color = Color::Rgb(RgbColor(r1,g1,b1));
            bg_tool_color = Color::Rgb(RgbColor(r2,g2,b2));
        }
        
        let white = Color::Rgb(RgbColor(255,255,255));

        let user_prompt = Style::new().bold().bg_color(Some(bg_user_color)).fg_color(Some(white));
        let user_prompt_arrow = Style::new().bold().fg_color(Some(bg_user_color));
        let user_text = Style::new().fg_color(Some(fg_user_color));

        let apprentice_prompt = Style::new().bold().bg_color(Some(bg_apprentice_color)).fg_color(Some(white));
        let apprentice_prompt_arrow = Style::new().bold().fg_color(Some(bg_apprentice_color));
        let apprentice_text = Style::new().fg_color(Some(fg_apprentice_color));

        let tool_prompt = Style::new().bold().bg_color(Some(bg_tool_color)).fg_color(Some(white));
        let tool_prompt_arrow = Style::new().bold().fg_color(Some(bg_tool_color));
        let tool_text = Style::new().fg_color(Some(fg_tool_color));

        Self {
            user_prompt,
            user_prompt_arrow,
            user_text,
            apprentice_prompt,
            apprentice_prompt_arrow,
            apprentice_text,
            tool_prompt,
            tool_prompt_arrow,
            tool_text,
        }
    }
}
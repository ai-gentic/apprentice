use std::borrow::Cow;

use crate::{config::Config, style::Styles, error::AppError};
use rustyline::{config::BellStyle, highlight::{CmdKind, Highlighter}, history::MemHistory, Completer, CompletionType, EditMode, Editor, Helper, Hinter, Validator};

const LOGO: &str = r"
    ___    ___   ___   ___   ____ _  __ ______ ____ _____ ____
   / _ |  / _ \ / _ \ / _ \ / __// |/ //_  __//  _// ___// __/
  / __ | / ___// ___// , _// _/ /    /  / /  _/ / / /__ / _/  
 /_/ |_|/_/   /_/   /_/|_|/___//_/|_/  /_/  /___/ \___//___/";

const INSTRUCTIONS: &str = "For help use ?, to exit use Ctrl+C";

const HELP: &str = "You are in a dialogue with Apprentice, please enter your request. 
Apprentice can ask clarifying questions, use tools, for example, 
execute a shell command (each time it will ask for user confirmation), etc.
It is not recommended to trust the application blindly.";

/// Terminal stuff.
pub struct Term {
    user_prompt: String, 
    apprentice_prompt: String, 
    styles: Styles,
    dumb: bool,
    editor: Editor<RlineHelper, MemHistory>,
}

impl Term {
    /// New instance.
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let styles = Styles::new(config);

        let rline_config = rustyline::Config::builder()
            .history_ignore_space(true)
            .auto_add_history(true)
            .bell_style(BellStyle::None)
            .check_cursor_position(true)
            .completion_type(CompletionType::List)
            .edit_mode(EditMode::Emacs)
            .build();

        let (user_prompt, apprentice_prompt, dumb) = 
        if Ok("dumb") == std::env::var("TERM").as_deref() {
            (
                "USER> ".to_owned(),
                "APPRENTICE> ".to_owned(),
                true,
            )
        } else {
            (
                format!("{} USER {:#}{} {:#}", styles.user_prompt, styles.user_prompt, styles.user_prompt_arrow, styles.user_prompt_arrow),
                format!("{} APPRENTICE {:#}{} {:#}", styles.apprentice_prompt, styles.apprentice_prompt, styles.apprentice_prompt_arrow, styles.apprentice_prompt_arrow),
                false,
            )
        };

        let mut editor: Editor<RlineHelper, MemHistory> = Editor::with_config(rline_config)?;
        let h = RlineHelper {
            colored_prompt: String::new()
        };
        editor.set_helper(Some(h));

        Ok(Term {
            user_prompt,
            apprentice_prompt, 
            styles,
            dumb,
            editor,
        })
    }

    /// Get input from user.
    pub fn user_input(&mut self) -> Result<String, AppError> {
        if self.dumb {
            self.editor.readline(&self.user_prompt).map_err(|e| e.into())
        } else {
            self.editor.helper_mut().unwrap().colored_prompt = format!("{}{}", &self.user_prompt, self.styles.user_text);
            let ret = self.editor.readline(" USER > ");
            print!("{:#}", self.styles.user_text);
            ret.map_err(|e| e.into())
        }
    }

    /// Print as apprentice.
    pub fn apprentice_print(&self, s: &str) {
        if self.dumb {
            println!("{}{}", self.apprentice_prompt, s);
        } else {
            println!("{}{}{}{:#}", self.apprentice_prompt, self.styles.apprentice_text, s, self.styles.apprentice_text);
        }
    }

    /// Print logo and instructions.
    pub fn print_into(&self) {
        if self.dumb {
            println!("{}\n (ver. {})\n\n{}", LOGO, env!("CARGO_PKG_VERSION"), INSTRUCTIONS);
        } else {
            println!("{}{}\n (ver. {})\n\n{}{:#}", self.styles.apprentice_text, LOGO,  env!("CARGO_PKG_VERSION"), INSTRUCTIONS, self.styles.apprentice_text);
        }
    }

    /// Print command suggested for execution.
    pub fn print_tool_message(&self, tool: &str, message: &str) {
        if self.dumb {
            println!("{}> {}", tool, message);
        } else {
            println!("{} {} {:#}{} {:#}{}{}{:#}", 
                self.styles.tool_prompt, 
                tool, 
                self.styles.tool_prompt, 
                self.styles.tool_prompt_arrow, 
                self.styles.tool_prompt_arrow, 
                self.styles.tool_text,
                message,
                self.styles.tool_text
            );
        }
    }

    /// Tool request input from user.
    pub fn tool_input(&mut self, tool: &str, text: &str) -> Result<String, AppError> {
        if self.dumb {
            self.editor.readline(&format!("{}> {}", tool, text)).map_err(|e| e.into())
        } else {
            self.editor.helper_mut().unwrap().colored_prompt = format!("{} {} {:#}{} {:#}{}{}", 
                self.styles.tool_prompt,
                tool, 
                self.styles.tool_prompt,
                self.styles.tool_prompt_arrow,
                self.styles.tool_prompt_arrow,
                self.styles.tool_text,
                text
            );
            let ret = self.editor.readline(&format!(" {} > {}", tool, text));
            print!("{:#}", self.styles.tool_text);
            ret.map_err(|e| e.into())
        }
    }

    /// Begin formatting with tool ouput style.
    pub fn begin_tool_format(&self) {
        print!("{}", self.styles.tool_text);
    }

    /// End formatting with tool ouput style.
    pub fn end_tool_format(&self) {
        print!("{:#}", self.styles.tool_text);
    }

    /// Print help information.
    pub fn print_help(&self) {
        if !self.dumb { print!("{}", self.styles.apprentice_text); }
        print!("{}", HELP);
        if !self.dumb { println!("{:#}", self.styles.apprentice_text); }
    }
}


#[derive(Helper, Validator, Hinter, Completer)]
struct RlineHelper {
    colored_prompt: String,
}

impl Highlighter for RlineHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        let _ = pos;
        Cow::Borrowed(line)
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        default: bool,
    ) -> Cow<'b, str> {
        if default {
            Cow::Borrowed(&self.colored_prompt)
        } else {
            Cow::Borrowed(prompt)
        }
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Borrowed(hint)
    }

    fn highlight_candidate<'c>(
        &self,
        candidate: &'c str, // FIXME should be Completer::Candidate
        completion: rustyline::CompletionType,
    ) -> Cow<'c, str> {
        let _ = completion;
        Cow::Borrowed(candidate)
    }

    fn highlight_char(&self, line: &str, pos: usize, kind: CmdKind) -> bool {
        let _ = (line, pos, kind);
        false
    }
}
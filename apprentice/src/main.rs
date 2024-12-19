mod agent;
mod config;
mod error;
mod options;
mod prompts;
mod style;
mod term;
mod toml_parser;
mod util;
mod tools;
mod rag;

use agent::Agent;
use error::AppError;
use options::Options;
use config::Config;
use prompts::Prompts;

fn run_agent() -> Result<(), AppError> {
    let options = Options::load(std::env::args())?;
    let config: Config = options.try_into()?;
    let prompts = Prompts::new(&config.prompt, config.goal);

    Agent::new(config, prompts)?.run()
}

fn main() {
    if let Err(e) = run_agent() { 
        eprintln!("ERROR: {e}");
        std::process::exit(1);
    }
}

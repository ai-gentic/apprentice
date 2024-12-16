use std::process::{Command, ExitStatus, Stdio};
use crate::error::Error;

/// Execute command in shell environment.
pub fn exec_pipe(command: &str) -> Result<ExitStatus, Error> {
    let child = if cfg!(target_os = "windows") {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    let output = child.wait_with_output()?;

    Ok(output.status)
}

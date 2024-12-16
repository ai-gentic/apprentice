use apprentice_lib::ModelProvider;
use std::thread;
use crate::AppError;
use std::{io::Write, process::{Child, Command, Stdio}};
use std::io;

/// API URL by provider.
pub fn api_url_for_provider(provider: ModelProvider, model: &str) -> String {
    match provider {
        ModelProvider::OpenAI => "https://api.openai.com/v1/chat/completions".into(),
        ModelProvider::Anthropic => "https://api.anthropic.com/v1/messages".into(),
        ModelProvider::GCP => format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model),
    }
}

/// Execute command in shell environment.
pub fn exec_pipe(command: &str) -> Result<String, AppError> {
    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .arg("/C")
            .arg(command)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdin(Stdio::inherit())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }.map_err(|err| AppError::Error(format!("Failed to run {}\nError: {}", command, err)))?;

    let (output1, output2) = stream_and_capture_stdio(&mut child).map_err(|err| AppError::Error(format!("Failed to capture stdio of {}\nError: {}", command, err)))?;

    let _exit_code = child.wait().map_err(|err| AppError::Error(format!("Failed to terminate {}\nError: {}", command, err)))?;

    let output = format!("STDOUT:\n{}\nSTDERR:\n{}", String::from_utf8_lossy(&output1), String::from_utf8_lossy(&output2));

    Ok(output)
}

// Write to stdio and buffer at the same time.
struct StreamBufferWriter<T: Write> {
    buf: Vec<u8>,
    stdstream: T,
}

impl<T: Write> Write for StreamBufferWriter<T> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.stdstream.write(buf)?;
        self.buf.write_all(&buf[..len])?;
        Ok(len)
        
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdstream.flush()?;
        self.buf.flush()
    }
}

// Capture and return stdout and stderr of the child process.
fn stream_and_capture_stdio(child: &mut Child) -> std::io::Result<(Vec<u8>, Vec<u8>)> {

    let thread1 = child.stdout.take()
        .map(|mut stdout| thread::spawn(move || -> Result<Vec<u8>, io::Error> {
            let writer = io::stdout().lock();
            let mut sbw = StreamBufferWriter { buf: vec![], stdstream: writer, };
            io::copy(&mut stdout, &mut sbw)?;
            Ok(sbw.buf)
        }));

    let thread2 = child.stderr.take()
        .map(|mut stderr| thread::spawn(move || -> Result<Vec<u8>, io::Error> {
            let writer = io::stderr().lock();
            let mut sbw = StreamBufferWriter { buf: vec![], stdstream: writer, };
            io::copy(&mut stderr, &mut sbw)?;
            Ok(sbw.buf)
        }));

    let output1 = if let Some(jh) = thread1 {
        jh.join().unwrap()?
    } else {
        vec![]
    };

    let output2 = if let Some(jh) = thread2 {
        jh.join().unwrap()?
    } else {
        vec![]
    };

    Ok((output1, output2))
}

/// Parse foragroud and background colors from string.
pub fn parse_colors(s: &str) -> Result<(Option<[u8;3]>, Option<[u8;3]>), AppError> {
    let mut fg = None;
    let mut bg = None;
    let s = s.trim();
    let s = s.trim_matches(['\'', '"']);

    for part in s.split(";") {
        let part = part.trim();

        if let Some(rgb) = part.strip_prefix("bg") {
            bg.replace(parse_color(rgb.trim())?);
        } else if let Some(rgb) = part.strip_prefix("fg") {
            fg.replace(parse_color(rgb.trim())?);
        } else {
            return Err(AppError::ColorParseError);
        };
    }

    Ok((fg, bg))
}

fn parse_color(s: &str) -> Result<[u8;3], AppError> {
    let mut color = [0u8;3];
    if !(s.starts_with('(') && s.ends_with(')')) {
        return Err(AppError::ColorParseError)
    }

    let mut i = 0;
    let s = &s[1..s.len()-1];
    for s in s.split(',') {
        if i > 2 {
            return Err(AppError::ColorParseError);
        }
        color[i] = s.trim().parse::<u8>().map_err(|_| AppError::ColorParseError)?;
        i+=1;
    }
    if i < 3 {
        return Err(AppError::ColorParseError);
    }

    Ok(color)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("( 0, 123, 255 )").unwrap(), [0,123,255]);
        assert_eq!(parse_color("(0,123,255)").unwrap(), [0,123,255]);
        assert!(matches!(parse_color("( 256, 123, 123 )").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_color("( 256, 123, -1 )").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_color("( 123, 123, 123, 123 )").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_color("(123, 123)").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_color("asdfg").unwrap_err(), AppError::ColorParseError));
    }

    #[test]
    fn test_parse_colors() {
        assert_eq!(parse_colors(" bg ( 0, 123, 255 ) ").unwrap(), (None, Some([0,123,255])));
        assert_eq!(parse_colors("fg(0,123,255)").unwrap(), (Some([0,123,255]), None));
        assert_eq!(parse_colors(" bg ( 255, 0, 123 ) ; fg ( 0, 123, 255 ) ").unwrap(), (Some([0,123,255]), Some([255,0,123])));
        assert_eq!(parse_colors("fg(255,0,123);bg(0,123,255)").unwrap(), (Some([255,0,123]), Some([0,123,255])));
        assert_eq!(parse_colors("fg(255,0,123);bg(0,123,255);bg(123,255,0)").unwrap(), (Some([255,0,123]), Some([123,255,0])));
        assert!(matches!(parse_colors("fg(255,0,123);gg(0,123,255)").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_colors("fg(255,0,123)gg(0,123,255)").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_colors("fg(255,0,123)bg(0,123,255)").unwrap_err(), AppError::ColorParseError));
        assert!(matches!(parse_colors("fg(255,0,123);bg(0,123,255);(123,255,0)").unwrap_err(), AppError::ColorParseError));
    }
}
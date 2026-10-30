use std::fmt;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[derive(Debug)]
pub enum ClipboardError {
    ToolNotFound,
    SpawnFailed(String),
    Io(String),
    CommandFailed(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClipboardError::ToolNotFound => write!(
                f,
                "Clipboard tool 'wl-copy' was not found in PATH. Please install 'wl-clipboard'."
            ),
            ClipboardError::SpawnFailed(err) => {
                write!(f, "Failed to start 'wl-copy': {}", err)
            }
            ClipboardError::Io(err) => {
                write!(f, "Failed to send image data to clipboard: {}", err)
            }
            ClipboardError::CommandFailed(err) => {
                if err.is_empty() {
                    write!(f, "Clipboard tool 'wl-copy' exited with an error.")
                } else {
                    write!(f, "Clipboard tool 'wl-copy' failed: {}", err)
                }
            }
        }
    }
}

impl std::error::Error for ClipboardError {}

pub fn copy_to_clipboard(png_bytes: &[u8]) -> Result<(), ClipboardError> {
    let mut cmd = Command::new("wl-copy");
    cmd.arg("--type")
        .arg("image/png")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .process_group(0);

    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(ClipboardError::ToolNotFound);
        }
        Err(e) => {
            return Err(ClipboardError::SpawnFailed(e.to_string()));
        }
    };

    if let Some(mut stdin) = child.stdin.take() {
        if let Err(e) = stdin.write_all(png_bytes) {
            return Err(ClipboardError::Io(e.to_string()));
        }
        if let Err(e) = stdin.flush() {
            return Err(ClipboardError::Io(e.to_string()));
        }
    }

    let status = match child.wait() {
        Ok(s) => s,
        Err(e) => return Err(ClipboardError::Io(e.to_string())),
    };

    if !status.success() {
        let mut err_msg = String::new();
        if let Some(mut err_pipe) = child.stderr.take() {
            let _ = err_pipe.read_to_string(&mut err_msg);
        }
        return Err(ClipboardError::CommandFailed(err_msg.trim().to_string()));
    }

    Ok(())
}

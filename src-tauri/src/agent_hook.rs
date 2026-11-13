//! `rice.exe --agent-hook <agent>`: the process a coding agent starts as its
//! hook (docs/BACKEND.md §4 "Coding-Agents"). It forwards the hook payload to
//! the running rice and prints rice's answer in the agent's format.
//!
//! It always exits with 0 and prints nothing when in doubt (rice not running,
//! another program behind the pipe, any error), so the agent asks in the
//! terminal as usual: on Windows a failing permission hook can block the call.

use rice_core::agents::{self, MAX_MESSAGE, PipeAnswer, PipeMessage};
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::Path;
use std::time::Duration;

/// `SECURITY_IDENTIFICATION`: the server may learn who calls, not act as the caller.
const SECURITY_IDENTIFICATION: u32 = 0x0001_0000;
/// All pipe instances are busy for a moment.
const ERROR_PIPE_BUSY: i32 = 231;

/// Runs the hook; never panics past this point.
pub fn run(agent: &str) {
    let _ = std::panic::catch_unwind(|| forward(agent));
}

fn forward(agent: &str) -> Option<()> {
    let mut input = Vec::new();
    io::stdin().lock().take(MAX_MESSAGE).read_to_end(&mut input).ok()?;
    let payload: serde_json::Value = serde_json::from_slice(&input).ok()?;
    let message = match agent {
        "claude" => agents::claude_message(&payload)?,
        _ => return None,
    };
    let answer = exchange(&rice_win::pipes::agent_pipe_name(), &message)?;
    let output = agents::claude_answer(answer.decision)?;
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{output}").ok()?;
    stdout.flush().ok()
}

/// Sends `message` to rice behind the pipe `name`; rice's answer to a
/// permission request (`None` for a status).
pub fn exchange(name: &str, message: &PipeMessage) -> Option<PipeAnswer> {
    let mut pipe = connect(name)?;
    let mut line = serde_json::to_vec(message).ok()?;
    line.push(b'\n');
    pipe.write_all(&line).ok()?;
    match message {
        PipeMessage::Status { .. } | PipeMessage::Settled { .. } => {
            // Returns once rice has read the message (FlushFileBuffers on a pipe).
            let _ = pipe.sync_all();
            None
        }
        PipeMessage::Permission { .. } => {
            let mut answer = String::new();
            BufReader::new(pipe.take(4096)).read_line(&mut answer).ok()?;
            serde_json::from_str(&answer).ok()
        }
    }
}

/// The pipe `name` of the running rice, if it is this very program.
fn connect(name: &str) -> Option<File> {
    for _ in 0..40 {
        match OpenOptions::new().read(true).write(true).security_qos_flags(SECURITY_IDENTIFICATION).open(name) {
            Ok(pipe) => return is_rice(&pipe).then_some(pipe),
            Err(error) if error.raw_os_error() == Some(ERROR_PIPE_BUSY) => std::thread::sleep(Duration::from_millis(50)),
            // Not found: rice is not running.
            Err(_) => return None,
        }
    }
    None
}

/// Tool inputs can contain secrets: only the same rice.exe may read them,
/// not a program that took the pipe's name first.
fn is_rice(pipe: &File) -> bool {
    match (rice_win::pipes::server_executable(pipe), std::env::current_exe()) {
        (Some(server), Ok(own)) => same_file(&server, &own),
        _ => false,
    }
}

fn same_file(a: &Path, b: &Path) -> bool {
    let normal = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_owned()).to_string_lossy().to_lowercase();
    normal(a) == normal(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_paths_like_windows() {
        let own = std::env::current_exe().unwrap();
        let upper = std::path::PathBuf::from(own.to_string_lossy().to_uppercase());
        assert!(same_file(&own, &upper));
        assert!(!same_file(&own, &own.with_file_name("other.exe")));
    }
}

use crate::{allowlist, VerifierError};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// The result of running a verified command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub command: String,
    pub exit_code: Option<i32>,
    pub stdout_tail: String,
    pub stderr_tail: String,
    pub timed_out: bool,
    pub success: bool,
}

/// Maximum number of bytes of stdout/stderr to retain.
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

/// Run an allowlisted command in `working_dir` with a timeout.
///
/// The command string is split on whitespace; the first token is the program
/// and the rest are arguments.
pub async fn run_command(
    command: &str,
    working_dir: impl AsRef<Path>,
    timeout_secs: u64,
) -> Result<CommandResult, VerifierError> {
    // Validate against the allowlist.
    allowlist::is_allowed(command).map_err(|e| VerifierError::NotAllowed(e))?;

    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return Err(VerifierError::NotAllowed("empty command".to_string()));
    }

    let mut cmd = Command::new(parts[0]);
    cmd.args(&parts[1..]);
    cmd.current_dir(working_dir.as_ref());

    // Strip sensitive environment variables; only forward safe ones.
    cmd.env_clear();
    for key in &[
        "PATH",
        "HOME",
        "RUSTUP_HOME",
        "CARGO_HOME",
        "GOPATH",
        "GOROOT",
    ] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }

    let run = timeout(Duration::from_secs(timeout_secs), cmd.output()).await;

    match run {
        Err(_) => Ok(CommandResult {
            command: command.to_string(),
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: format!("Command timed out after {timeout_secs}s"),
            timed_out: true,
            success: false,
        }),
        Ok(Err(e)) => Err(VerifierError::Io(e)),
        Ok(Ok(output)) => {
            let stdout = tail_bytes(&output.stdout, MAX_OUTPUT_BYTES);
            let stderr = tail_bytes(&output.stderr, MAX_OUTPUT_BYTES);
            let exit_code = output.status.code();
            let success = output.status.success();
            Ok(CommandResult {
                command: command.to_string(),
                exit_code,
                stdout_tail: stdout,
                stderr_tail: stderr,
                timed_out: false,
                success,
            })
        }
    }
}

/// Convert `bytes` to a UTF-8 string, keeping at most `max_bytes` from the
/// tail of the output.
fn tail_bytes(bytes: &[u8], max_bytes: usize) -> String {
    let slice = if bytes.len() > max_bytes {
        &bytes[bytes.len() - max_bytes..]
    } else {
        bytes
    };
    String::from_utf8_lossy(slice).to_string()
}

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during policy enforcement.
#[derive(Debug, Error)]
pub enum PolicyError {
    #[error("command not in allowlist: {0}")]
    NotAllowed(String),

    #[error("command contains a destructive pattern: {0}")]
    Destructive(String),

    #[error("command output exceeds maximum size")]
    OutputTooLarge,

    #[error("command timed out")]
    Timeout,
}

/// Allowlisted safe commands and their canonical prefix forms.
static ALLOWED_COMMANDS: &[&str] = &[
    "cargo test",
    "cargo check",
    "cargo fmt",
    "cargo clippy",
    "cargo build",
    "npm test",
    "npm run test",
    "npm run typecheck",
    "npm run lint",
    "npm run build",
    "pytest",
    "ruff check",
    "mypy",
    "go test",
    "python -m pytest",
    "python -m mypy",
];

/// Destructive command patterns that must never be executed.
static DESTRUCTIVE_PATTERNS: &[&str] = &[
    "rm -rf",
    "rm -r",
    "sudo",
    "curl | sh",
    "curl |sh",
    "wget | sh",
    "wget |sh",
    "chmod -R",
    "chown -R",
    "docker system prune",
    "git reset --hard",
    "git clean -fd",
    "git clean -f",
    "> /dev/sda",
    "mkfs",
    "dd if=",
    ":(){ :|:& };:",
];

/// Execution configuration for a command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandConfig {
    /// Maximum execution time in seconds.
    pub timeout_secs: u64,

    /// Maximum size of combined stdout + stderr output in bytes.
    pub max_output_bytes: usize,

    /// The working directory (should be inside the workspace).
    pub working_directory: std::path::PathBuf,

    /// Environment variable allowlist (only these keys are forwarded).
    pub allowed_env_keys: Vec<String>,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 60,
            max_output_bytes: 256 * 1024, // 256 KB
            working_directory: std::path::PathBuf::from("."),
            allowed_env_keys: vec![
                "PATH".into(),
                "HOME".into(),
                "RUSTUP_HOME".into(),
                "CARGO_HOME".into(),
                "GOPATH".into(),
                "GOROOT".into(),
                "VIRTUAL_ENV".into(),
                "NODE_PATH".into(),
            ],
        }
    }
}

/// Enforces the command allowlist and destructive-pattern denylist.
#[derive(Debug, Clone)]
pub struct CommandPolicy {
    config: CommandConfig,
}

impl CommandPolicy {
    pub fn new(config: CommandConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: CommandConfig::default(),
        }
    }

    pub fn config(&self) -> &CommandConfig {
        &self.config
    }

    /// Validate that `command` is allowed.
    ///
    /// Returns `Ok(())` if the command passes all checks, or a `PolicyError`
    /// describing the violation.
    pub fn validate_command(&self, command: &str) -> Result<(), PolicyError> {
        let trimmed = command.trim();

        // Check for destructive patterns first.
        for pattern in DESTRUCTIVE_PATTERNS {
            if trimmed.contains(pattern) {
                return Err(PolicyError::Destructive(command.to_string()));
            }
        }

        // Check against allowlist.
        let is_allowed = ALLOWED_COMMANDS
            .iter()
            .any(|allowed| trimmed.starts_with(allowed));

        if !is_allowed {
            return Err(PolicyError::NotAllowed(command.to_string()));
        }

        Ok(())
    }

    /// Return a copy of the execution configuration.
    pub fn execution_config(&self) -> &CommandConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> CommandPolicy {
        CommandPolicy::with_defaults()
    }

    #[test]
    fn test_allowed_cargo_test() {
        assert!(policy().validate_command("cargo test").is_ok());
    }

    #[test]
    fn test_allowed_cargo_test_with_args() {
        assert!(policy().validate_command("cargo test auth").is_ok());
    }

    #[test]
    fn test_allowed_npm_test() {
        assert!(policy().validate_command("npm test").is_ok());
    }

    #[test]
    fn test_rejected_arbitrary_command() {
        assert!(matches!(
            policy().validate_command("ls -la"),
            Err(PolicyError::NotAllowed(_))
        ));
    }

    #[test]
    fn test_rejected_destructive_rm() {
        assert!(matches!(
            policy().validate_command("rm -rf /"),
            Err(PolicyError::Destructive(_))
        ));
    }

    #[test]
    fn test_rejected_sudo() {
        assert!(matches!(
            policy().validate_command("sudo cargo test"),
            Err(PolicyError::Destructive(_))
        ));
    }

    #[test]
    fn test_rejected_git_reset_hard() {
        assert!(matches!(
            policy().validate_command("git reset --hard HEAD~1"),
            Err(PolicyError::Destructive(_))
        ));
    }
}

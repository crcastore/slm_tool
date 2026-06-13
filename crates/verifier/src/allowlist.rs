/// Commands that the verifier is permitted to run.
///
/// Each entry is a command prefix; a request must start with one of these
/// (after trimming leading whitespace) to be allowed.
pub static ALLOWED_COMMANDS: &[&str] = &[
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
    "npx tsc",
    "pytest",
    "python -m pytest",
    "ruff check",
    "ruff format",
    "mypy",
    "python -m mypy",
    "go test",
    "go build",
    "go vet",
];

/// Destructive patterns that must never be executed.
pub static DESTRUCTIVE_PATTERNS: &[&str] = &[
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
    "git clean",
    "mkfs",
    "dd if=",
    "> /dev/",
];

/// Check whether `command` is allowed.
///
/// Returns `Ok(())` if allowed, or an error string describing why it was
/// rejected.
pub fn is_allowed(command: &str) -> Result<(), String> {
    let trimmed = command.trim();

    for pattern in DESTRUCTIVE_PATTERNS {
        if trimmed.contains(pattern) {
            return Err(format!("destructive pattern detected: {pattern}"));
        }
    }

    let allowed = ALLOWED_COMMANDS
        .iter()
        .any(|prefix| trimmed.starts_with(prefix));

    if !allowed {
        return Err(format!("command not in allowlist: {trimmed}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allowed_cargo_test() {
        assert!(is_allowed("cargo test").is_ok());
        assert!(is_allowed("cargo test --release").is_ok());
    }

    #[test]
    fn test_rejected_shell() {
        assert!(is_allowed("ls -la").is_err());
    }

    #[test]
    fn test_rejected_rm_rf() {
        assert!(is_allowed("cargo test && rm -rf /").is_err());
    }

    #[test]
    fn test_allowed_pytest() {
        assert!(is_allowed("pytest tests/").is_ok());
    }
}

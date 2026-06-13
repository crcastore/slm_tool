use crate::commands::CommandResult;

/// Extract a short human-readable summary from a `CommandResult`.
///
/// This helps the model quickly understand whether an operation succeeded
/// without parsing the full stdout/stderr.
pub fn summarize(result: &CommandResult) -> String {
    if result.timed_out {
        return format!("Command timed out: {}", result.command);
    }

    let status = match result.exit_code {
        Some(0) => "PASS",
        Some(code) => return format!("FAIL (exit {}): {}", code, extract_first_error(&result.stderr_tail)),
        None => "FAIL (no exit code)",
    };

    format!("{status}: {}", result.command)
}

/// Extract the first non-empty line from stderr that looks like an error.
fn extract_first_error(stderr: &str) -> String {
    for line in stderr.lines() {
        let lower = line.to_lowercase();
        if lower.contains("error")
            || lower.contains("failed")
            || lower.contains("panicked")
            || lower.contains("FAILED")
        {
            return line.trim().to_string();
        }
    }
    // Fall back to first non-empty line.
    stderr
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("(no output)")
        .trim()
        .to_string()
}

/// Format the full test output for display.
pub fn format_test_output(result: &CommandResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("$ {}\n", result.command));
    if !result.stdout_tail.is_empty() {
        out.push_str("--- stdout ---\n");
        out.push_str(&result.stdout_tail);
        out.push('\n');
    }
    if !result.stderr_tail.is_empty() {
        out.push_str("--- stderr ---\n");
        out.push_str(&result.stderr_tail);
        out.push('\n');
    }
    out.push_str(&format!(
        "--- exit code: {:?} ---\n",
        result.exit_code
    ));
    out
}

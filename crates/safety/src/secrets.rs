use serde::{Deserialize, Serialize};
use std::path::Path;

/// Result of scanning a file for potential secrets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretScanResult {
    pub path: String,
    pub findings: Vec<SecretFinding>,
    pub is_clean: bool,
}

/// A single potential-secret finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretFinding {
    pub line: usize,
    pub kind: String,
    pub snippet: String,
}

/// Patterns that suggest a line may contain a secret or credential.
///
/// These are intentionally conservative — they look for common assignment
/// patterns rather than trying to match every possible secret format.
static SECRET_PATTERNS: &[(&str, &str)] = &[
    (r#"(?i)(api[_-]?key|apikey)\s*=\s*['"][^'"]{8,}"#, "api_key"),
    (
        r#"(?i)(secret[_-]?key|secretkey)\s*=\s*['"][^'"]{8,}"#,
        "secret_key",
    ),
    (
        r#"(?i)(password|passwd|pwd)\s*=\s*['"][^'"]{4,}"#,
        "password",
    ),
    (
        r#"(?i)(access[_-]?token|auth[_-]?token)\s*=\s*['"][^'"]{8,}"#,
        "token",
    ),
    (
        r#"(?i)(private[_-]?key|privkey)\s*=\s*['"][^'"]{8,}"#,
        "private_key",
    ),
    (
        r"-----BEGIN (RSA|EC|DSA|OPENSSH) PRIVATE KEY-----",
        "private_key_block",
    ),
    (
        r"(?i)aws[_-]?access[_-]?key[_-]?id\s*=\s*[A-Z0-9]{16,}",
        "aws_key",
    ),
    (r"AKIA[0-9A-Z]{16}", "aws_access_key_id"),
    (r"(?i)bearer\s+[a-zA-Z0-9\-._~+/]{32,}", "bearer_token"),
];

/// Scans files and strings for potential secrets.
#[derive(Debug)]
pub struct SecretScanner {
    patterns: Vec<(regex::Regex, String)>,
}

impl SecretScanner {
    /// Create a new scanner with the built-in patterns.
    pub fn new() -> Self {
        let patterns = SECRET_PATTERNS
            .iter()
            .filter_map(|(pat, kind)| regex::Regex::new(pat).ok().map(|re| (re, kind.to_string())))
            .collect();
        Self { patterns }
    }

    /// Scan `content` for potential secrets and return any findings.
    pub fn scan_content(&self, path: impl AsRef<Path>, content: &str) -> SecretScanResult {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let mut findings = Vec::new();

        for (line_number, line) in content.lines().enumerate() {
            for (re, kind) in &self.patterns {
                if re.is_match(line) {
                    // Truncate the line to avoid leaking the full secret.
                    let snippet = if line.len() > 80 {
                        format!("{}…", &line[..80])
                    } else {
                        line.to_string()
                    };
                    findings.push(SecretFinding {
                        line: line_number + 1,
                        kind: kind.clone(),
                        snippet,
                    });
                    break; // Only report one finding per line.
                }
            }
        }

        let is_clean = findings.is_empty();
        SecretScanResult {
            path: path_str,
            findings,
            is_clean,
        }
    }
}

impl Default for SecretScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanner() -> SecretScanner {
        SecretScanner::new()
    }

    #[test]
    fn test_clean_content() {
        let result = scanner().scan_content("test.rs", "fn main() { println!(\"hello\"); }");
        assert!(result.is_clean);
    }

    #[test]
    fn test_api_key_detected() {
        let content = r#"api_key = "supersecretkey12345""#;
        let result = scanner().scan_content("config.py", content);
        assert!(!result.is_clean);
        assert_eq!(result.findings[0].kind, "api_key");
    }

    #[test]
    fn test_private_key_block_detected() {
        let content = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAK...";
        let result = scanner().scan_content("key.pem", content);
        assert!(!result.is_clean);
        assert_eq!(result.findings[0].kind, "private_key_block");
    }

    #[test]
    fn test_aws_access_key_detected() {
        let content = "AKIAIOSFODNN7EXAMPLE";
        let result = scanner().scan_content("credentials", content);
        assert!(!result.is_clean);
    }
}

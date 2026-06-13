use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during path validation.
#[derive(Debug, Error)]
pub enum PathValidatorError {
    #[error("path is outside the workspace root: {0}")]
    OutsideWorkspace(PathBuf),

    #[error("path traversal detected in: {0}")]
    PathTraversal(PathBuf),

    #[error("path references a sensitive file: {0}")]
    SensitiveFile(PathBuf),

    #[error("I/O error resolving path: {0}")]
    Io(#[from] std::io::Error),
}

/// Sensitive file patterns that must never be read or written.
static SENSITIVE_PATTERNS: &[&str] = &[
    ".env",
    "id_rsa",
    "id_ed25519",
    "id_ecdsa",
    "id_dsa",
    ".aws",
    ".ssh",
    "secrets",
    ".gnupg",
    ".pgp",
];

/// Sensitive file extensions.
static SENSITIVE_EXTENSIONS: &[&str] = &["pem", "key", "p12", "pfx", "jks", "keystore"];

/// Validates file system paths against the workspace root boundary.
///
/// All file access is restricted to the workspace root. Paths outside the
/// workspace, path traversal attempts, and accesses to sensitive files are
/// all rejected.
#[derive(Debug, Clone)]
pub struct PathValidator {
    workspace_root: PathBuf,
}

impl PathValidator {
    /// Create a new validator anchored at `workspace_root`.
    ///
    /// The root is canonicalized at construction time so that relative roots
    /// work correctly.
    pub fn new(workspace_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let canonical = std::fs::canonicalize(workspace_root.as_ref())?;
        Ok(Self {
            workspace_root: canonical,
        })
    }

    /// Create a new validator without canonicalizing the root (useful in tests
    /// where the directory may not exist yet).
    pub fn new_unchecked(workspace_root: impl AsRef<Path>) -> Self {
        Self {
            workspace_root: workspace_root.as_ref().to_path_buf(),
        }
    }

    /// Return the workspace root.
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Validate `path` and return its absolute form inside the workspace.
    ///
    /// Returns `Err` if the path is outside the workspace, attempts traversal,
    /// or refers to a sensitive file.
    pub fn validate(&self, path: impl AsRef<Path>) -> Result<PathBuf, PathValidatorError> {
        let path = path.as_ref();

        // Resolve absolute path relative to workspace root when relative.
        let abs = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.workspace_root.join(path)
        };

        // Normalise without requiring the path to exist (no canonicalize).
        let normalized = normalize_path(&abs);

        // Check for path traversal components before resolution.
        if path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(PathValidatorError::PathTraversal(path.to_path_buf()));
        }

        // Ensure the normalised path is inside the workspace.
        if !normalized.starts_with(&self.workspace_root) {
            return Err(PathValidatorError::OutsideWorkspace(normalized));
        }

        // Check for sensitive files.
        self.check_sensitive(&normalized)?;

        Ok(normalized)
    }

    /// Check whether a normalised absolute path refers to a sensitive file.
    fn check_sensitive(&self, path: &Path) -> Result<(), PathValidatorError> {
        // Check each component of the path.
        for component in path.components() {
            if let std::path::Component::Normal(name) = component {
                let name_lower = name.to_string_lossy().to_lowercase();
                for pattern in SENSITIVE_PATTERNS {
                    if name_lower == *pattern || name_lower.starts_with(&format!("{pattern}.")) {
                        return Err(PathValidatorError::SensitiveFile(path.to_path_buf()));
                    }
                }
            }
        }

        // Check the file extension.
        if let Some(ext) = path.extension() {
            let ext_lower = ext.to_string_lossy().to_lowercase();
            if SENSITIVE_EXTENSIONS.contains(&ext_lower.as_str()) {
                return Err(PathValidatorError::SensitiveFile(path.to_path_buf()));
            }
        }

        Ok(())
    }
}

/// Normalize a path by resolving `.` and `..` components without hitting the
/// filesystem (so it works even for paths that do not exist yet).
pub fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut components: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if matches!(components.last(), Some(Component::Normal(_))) {
                    components.pop();
                } else {
                    components.push(component);
                }
            }
            other => components.push(other),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_validator() -> PathValidator {
        PathValidator::new_unchecked("/workspace")
    }

    #[test]
    fn test_valid_path_inside_workspace() {
        let v = make_validator();
        let result = v.validate("src/main.rs");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("/workspace/src/main.rs"));
    }

    #[test]
    fn test_path_traversal_rejected() {
        let v = make_validator();
        let result = v.validate("../etc/passwd");
        assert!(matches!(result, Err(PathValidatorError::PathTraversal(_))));
    }

    #[test]
    fn test_absolute_path_outside_workspace_rejected() {
        let v = make_validator();
        let result = v.validate("/etc/passwd");
        assert!(matches!(
            result,
            Err(PathValidatorError::OutsideWorkspace(_))
        ));
    }

    #[test]
    fn test_sensitive_env_file_rejected() {
        let v = make_validator();
        let result = v.validate(".env");
        assert!(matches!(result, Err(PathValidatorError::SensitiveFile(_))));
    }

    #[test]
    fn test_sensitive_env_local_file_rejected() {
        let v = make_validator();
        let result = v.validate(".env.local");
        assert!(matches!(result, Err(PathValidatorError::SensitiveFile(_))));
    }

    #[test]
    fn test_sensitive_pem_file_rejected() {
        let v = make_validator();
        let result = v.validate("certs/server.pem");
        assert!(matches!(result, Err(PathValidatorError::SensitiveFile(_))));
    }

    #[test]
    fn test_sensitive_ssh_key_rejected() {
        let v = make_validator();
        let result = v.validate(".ssh/id_rsa");
        assert!(matches!(result, Err(PathValidatorError::SensitiveFile(_))));
    }
}

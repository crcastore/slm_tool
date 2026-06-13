use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The category of an evaluation task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Find where a feature is implemented.
    FindImplementation,
    /// Explain a module or function.
    ExplainCode,
    /// Add a unit test for a function.
    AddTest,
    /// Fix a failing test.
    FixTest,
    /// Refactor a small function.
    Refactor,
    /// Update an API usage.
    UpdateApiUsage,
    /// Find all callers of a function.
    FindCallers,
    /// Add validation to a function.
    AddValidation,
}

impl std::fmt::Display for TaskKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        write!(f, "{s}")
    }
}

/// A single evaluation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalTask {
    pub id: String,
    pub kind: TaskKind,
    pub description: String,
    /// The repository path or context used for this task.
    pub repo_path: String,
    /// Expected answer or keywords that should appear in a correct response.
    pub expected_keywords: Vec<String>,
    /// Files that a correct solution must touch or reference.
    pub expected_files: Vec<String>,
}

impl EvalTask {
    pub fn new(
        kind: TaskKind,
        description: impl Into<String>,
        repo_path: impl Into<String>,
        expected_keywords: Vec<String>,
        expected_files: Vec<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            description: description.into(),
            repo_path: repo_path.into(),
            expected_keywords,
            expected_files,
        }
    }
}

/// A curated set of evaluation tasks for testing the assistant.
pub fn sample_tasks(repo_path: &str) -> Vec<EvalTask> {
    vec![
        EvalTask::new(
            TaskKind::FindImplementation,
            "Find where the file reading tool is implemented",
            repo_path,
            vec!["read_file".to_string(), "file".to_string()],
            vec![],
        ),
        EvalTask::new(
            TaskKind::ExplainCode,
            "Explain the workspace crawler module",
            repo_path,
            vec![
                "crawl".to_string(),
                "index".to_string(),
                "gitignore".to_string(),
            ],
            vec!["crates/workspace-index/src/crawler.rs".to_string()],
        ),
        EvalTask::new(
            TaskKind::FindCallers,
            "Find all callers of grep_workspace",
            repo_path,
            vec!["grep_workspace".to_string()],
            vec![],
        ),
        EvalTask::new(
            TaskKind::AddTest,
            "Add a unit test for the PathValidator",
            repo_path,
            vec!["test".to_string(), "PathValidator".to_string()],
            vec!["crates/safety/src/paths.rs".to_string()],
        ),
    ]
}

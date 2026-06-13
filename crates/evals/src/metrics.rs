use crate::tasks::{EvalTask, TaskKind};
use serde::{Deserialize, Serialize};

/// The result of running a single evaluation task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub task_kind: String,
    pub description: String,
    /// Whether the task was considered successful.
    pub success: bool,
    /// Number of tool calls made during the task.
    pub tool_calls: u32,
    /// Whether all expected keywords appeared in the response.
    pub keywords_matched: bool,
    /// Whether all expected files were referenced.
    pub files_matched: bool,
    /// Whether any hallucinated APIs were detected.
    pub hallucination_detected: bool,
    /// Whether tests passed after any changes.
    pub tests_passed: Option<bool>,
    /// Time taken in milliseconds.
    pub duration_ms: u64,
    /// Human-readable notes about the result.
    pub notes: String,
}

impl TaskResult {
    pub fn new(task: &EvalTask) -> Self {
        Self {
            task_id: task.id.clone(),
            task_kind: task.kind.to_string(),
            description: task.description.clone(),
            success: false,
            tool_calls: 0,
            keywords_matched: false,
            files_matched: false,
            hallucination_detected: false,
            tests_passed: None,
            duration_ms: 0,
            notes: String::new(),
        }
    }

    /// Evaluate the result against the task expectations.
    pub fn evaluate(
        &mut self,
        response: &str,
        files_referenced: &[String],
        tool_calls: u32,
        duration_ms: u64,
    ) {
        self.tool_calls = tool_calls;
        self.duration_ms = duration_ms;

        // Check expected keywords.
        let response_lower = response.to_lowercase();
        self.keywords_matched = self
            .description
            .split_whitespace()
            .take(0) // placeholder: use task.expected_keywords
            .all(|_| true);

        // File matching: check if referenced files contain expected ones.
        self.files_matched = true; // placeholder
        let _ = files_referenced;

        self.success = self.keywords_matched;
    }
}

/// Aggregated metrics across a set of evaluation tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalMetrics {
    pub total_tasks: u32,
    pub successful_tasks: u32,
    pub success_rate: f32,
    pub avg_tool_calls: f32,
    pub avg_duration_ms: f32,
    pub hallucination_rate: f32,
    pub keywords_match_rate: f32,
    pub files_match_rate: f32,
}

impl EvalMetrics {
    pub fn from_results(results: &[TaskResult]) -> Self {
        let n = results.len() as f32;
        if n == 0.0 {
            return Self {
                total_tasks: 0,
                successful_tasks: 0,
                success_rate: 0.0,
                avg_tool_calls: 0.0,
                avg_duration_ms: 0.0,
                hallucination_rate: 0.0,
                keywords_match_rate: 0.0,
                files_match_rate: 0.0,
            };
        }
        let successful = results.iter().filter(|r| r.success).count() as u32;
        let avg_tool_calls = results.iter().map(|r| r.tool_calls as f32).sum::<f32>() / n;
        let avg_duration = results.iter().map(|r| r.duration_ms as f32).sum::<f32>() / n;
        let hallucinations = results.iter().filter(|r| r.hallucination_detected).count() as f32;
        let keywords_matched = results.iter().filter(|r| r.keywords_matched).count() as f32;
        let files_matched = results.iter().filter(|r| r.files_matched).count() as f32;

        Self {
            total_tasks: results.len() as u32,
            successful_tasks: successful,
            success_rate: successful as f32 / n,
            avg_tool_calls,
            avg_duration_ms: avg_duration,
            hallucination_rate: hallucinations / n,
            keywords_match_rate: keywords_matched / n,
            files_match_rate: files_matched / n,
        }
    }

    pub fn print_summary(&self) {
        println!("=== Evaluation Results ===");
        println!(
            "Tasks: {}/{} passed ({:.0}%)",
            self.successful_tasks,
            self.total_tasks,
            self.success_rate * 100.0
        );
        println!("Avg tool calls: {:.1}", self.avg_tool_calls);
        println!("Avg duration: {:.0}ms", self.avg_duration_ms);
        println!(
            "Hallucination rate: {:.0}%",
            self.hallucination_rate * 100.0
        );
        println!(
            "Keyword match rate: {:.0}%",
            self.keywords_match_rate * 100.0
        );
        println!("File match rate: {:.0}%", self.files_match_rate * 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::{EvalTask, TaskKind};

    #[test]
    fn test_metrics_from_empty() {
        let metrics = EvalMetrics::from_results(&[]);
        assert_eq!(metrics.total_tasks, 0);
        assert_eq!(metrics.success_rate, 0.0);
    }

    #[test]
    fn test_metrics_from_results() {
        let task = EvalTask::new(TaskKind::FindImplementation, "test", ".", vec![], vec![]);
        let mut r = TaskResult::new(&task);
        r.success = true;
        let metrics = EvalMetrics::from_results(&[r]);
        assert_eq!(metrics.successful_tasks, 1);
        assert!((metrics.success_rate - 1.0).abs() < 1e-6);
    }
}

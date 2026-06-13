use crate::{
    metrics::{EvalMetrics, TaskResult},
    tasks::EvalTask,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for running evaluations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    pub workspace_root: PathBuf,
    pub output_dir: PathBuf,
    pub timeout_secs: u64,
}

impl Default for EvalConfig {
    fn default() -> Self {
        Self {
            workspace_root: PathBuf::from("."),
            output_dir: PathBuf::from("./eval-results"),
            timeout_secs: 120,
        }
    }
}

/// Runs a set of evaluation tasks and collects results.
pub struct EvalRunner {
    config: EvalConfig,
}

impl EvalRunner {
    pub fn new(config: EvalConfig) -> Self {
        Self { config }
    }

    pub fn with_defaults() -> Self {
        Self {
            config: EvalConfig::default(),
        }
    }

    /// Run all tasks and return aggregated metrics.
    ///
    /// In the current implementation this is a dry-run framework that returns
    /// placeholder results.  Future iterations will integrate with the MCP
    /// server to drive real model interactions.
    pub async fn run(&self, tasks: Vec<EvalTask>) -> EvalMetrics {
        let mut results = Vec::new();

        for task in &tasks {
            tracing_or_print(format!("Running task: {}", task.description));
            let start = std::time::Instant::now();

            // Placeholder: in a real eval the runner would call the MCP server
            // and collect the response, then score it.
            let mut result = TaskResult::new(task);
            result.duration_ms = start.elapsed().as_millis() as u64;
            results.push(result);
        }

        let metrics = EvalMetrics::from_results(&results);

        // Write results to the output directory.
        if let Err(e) = self.write_results(&results, &metrics) {
            tracing_or_print(format!("Failed to write eval results: {e}"));
        }

        metrics
    }

    fn write_results(&self, results: &[TaskResult], metrics: &EvalMetrics) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config.output_dir)?;
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        let results_path = self.config.output_dir.join(format!("results_{ts}.json"));
        let results_json =
            serde_json::to_string_pretty(results).unwrap_or_else(|_| "[]".to_string());
        std::fs::write(&results_path, results_json)?;

        let metrics_path = self.config.output_dir.join(format!("metrics_{ts}.json"));
        let metrics_json =
            serde_json::to_string_pretty(metrics).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(&metrics_path, metrics_json)?;

        Ok(())
    }
}

fn tracing_or_print(msg: String) {
    // Use eprintln as a fallback when tracing is not initialized.
    eprintln!("[eval] {msg}");
}

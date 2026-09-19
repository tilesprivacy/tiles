//! The log tail an error report carries
//!
//! Since the logs stopped carrying conversation text, their tail is safe to
//! hand to the UI for an error report. The report still travels only in the
//! URL fragment the user chooses to share; this endpoint just fills it in.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use axum::{Json, Router, routing::get};
use serde_json::json;

use crate::{
    daemon::{ApiResponse, AppError, AppState},
    utils::config::{ConfigProvider, DefaultProvider},
};

/// Enough context to diagnose, small enough to survive compression into a
/// scannable QR code.
const TAIL_LINES: usize = 40;
const MAX_LINE_CHARS: usize = 200;

pub fn diagnostics_router() -> Router<Arc<AppState>> {
    Router::new().route("/v1/tilekit/diagnostics/logs", get(log_tails))
}

/// The last lines of each log, labeled by file. Missing files are skipped
/// rather than failed: a fresh install has no llama log yet, and a report
/// with partial context beats no report.
async fn log_tails() -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // Two log homes, two accessors. boot.log lives under the user data dir
    // (`<root>/data/logs`, where the app-boot spawn writes); the daemon,
    // python server and llama-server log under the root's own `logs/`
    // (`get_data_dir`, which despite the name is the tiles root).
    let boot_logs = DefaultProvider
        .get_user_data_dir()
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .join("logs");
    let root_logs = DefaultProvider
        .get_data_dir()
        .map_err(|e| AppError::InternalServerError(e.to_string()))?
        .join("logs");

    let mut lines: Vec<String> = Vec::new();

    for (dir, name) in [
        (&boot_logs, "boot.log"),
        (&root_logs, "daemon.err.log"),
        (&root_logs, "server.err.log"),
        (&root_logs, "llama-server.err.log"),
    ] {
        let tail = tail_of(&dir.join(name));
        if tail.is_empty() {
            continue;
        }
        lines.push(format!("== {name} =="));
        lines.extend(tail);
    }

    Ok(ApiResponse::success(json!({ "lines": lines })))
}

fn tail_of(path: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(path) else {
        return vec![];
    };

    let all: Vec<&str> = content.lines().collect();
    all.iter()
        .skip(all.len().saturating_sub(TAIL_LINES))
        .map(|line| {
            if line.chars().count() > MAX_LINE_CHARS {
                let truncated: String = line.chars().take(MAX_LINE_CHARS).collect();
                format!("{truncated}…")
            } else {
                (*line).to_owned()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_is_bounded_in_lines_and_line_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.log");
        let long_line = "x".repeat(500);
        let content: Vec<String> = (0..100).map(|i| format!("line {i} {long_line}")).collect();
        fs::write(&path, content.join("\n")).unwrap();

        let tail = tail_of(&path);

        assert_eq!(tail.len(), TAIL_LINES);
        assert!(tail[0].starts_with("line 60"));
        assert!(tail.iter().all(|l| l.chars().count() <= MAX_LINE_CHARS + 1));
    }

    #[test]
    fn a_missing_log_is_no_tail_rather_than_an_error() {
        assert!(tail_of(Path::new("/nowhere/at/all.log")).is_empty());
    }
}

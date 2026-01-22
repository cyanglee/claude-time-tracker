use anyhow::Result;

use crate::models::MonthlyReport;

/// Generate JSON report
pub fn generate(report: &MonthlyReport) -> Result<String> {
    let json = serde_json::to_string_pretty(report)?;
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitSummary, ProjectReport, WorkItemReport};

    #[test]
    fn test_generate_json() {
        let report = MonthlyReport {
            period: "2025-01".to_string(),
            total_seconds: 7200,
            projects: vec![ProjectReport {
                name: "Test Project".to_string(),
                path: "/test/path".to_string(),
                total_seconds: 7200,
                work_items: vec![WorkItemReport {
                    id: "ABC-123".to_string(),
                    branch: Some("feature/ABC-123-test".to_string()),
                    total_seconds: 7200,
                    completed_date: Some("2025-01-15".to_string()),
                    commits: vec![CommitSummary {
                        hash: "abc123".to_string(),
                        message: "Test commit".to_string(),
                    }],
                }],
            }],
        };

        let json = generate(&report).unwrap();
        assert!(json.contains("\"period\": \"2025-01\""));
        assert!(json.contains("\"total_seconds\": 7200"));
        assert!(json.contains("\"name\": \"Test Project\""));
    }
}

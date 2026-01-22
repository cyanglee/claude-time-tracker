use anyhow::Result;
use std::io::Write;

use crate::models::MonthlyReport;

/// Generate CSV report
pub fn generate<W: Write>(report: &MonthlyReport, writer: W, include_commits: bool) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(writer);

    // Write header
    if include_commits {
        wtr.write_record(["project", "work_item", "hours", "minutes", "total_seconds", "commits"])?;
    } else {
        wtr.write_record(["project", "work_item", "hours", "minutes", "total_seconds"])?;
    }

    // Write data rows
    for project in &report.projects {
        for item in &project.work_items {
            let hours = item.total_seconds / 3600;
            let minutes = (item.total_seconds % 3600) / 60;

            if include_commits {
                let commits_str = item
                    .commits
                    .iter()
                    .map(|c| c.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ");

                wtr.write_record([
                    &project.name,
                    &item.id,
                    &hours.to_string(),
                    &minutes.to_string(),
                    &item.total_seconds.to_string(),
                    &commits_str,
                ])?;
            } else {
                wtr.write_record([
                    &project.name,
                    &item.id,
                    &hours.to_string(),
                    &minutes.to_string(),
                    &item.total_seconds.to_string(),
                ])?;
            }
        }
    }

    wtr.flush()?;
    Ok(())
}

/// Generate CSV report as string
pub fn generate_string(report: &MonthlyReport, include_commits: bool) -> Result<String> {
    let mut buffer = Vec::new();
    generate(report, &mut buffer, include_commits)?;
    Ok(String::from_utf8(buffer)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitSummary, ProjectReport, WorkItemReport};

    #[test]
    fn test_generate_csv() {
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
                    commits: vec![CommitSummary {
                        hash: "abc123".to_string(),
                        message: "Test commit".to_string(),
                    }],
                }],
            }],
        };

        let csv = generate_string(&report, true).unwrap();
        assert!(csv.contains("project,work_item,hours,minutes,total_seconds,commits"));
        assert!(csv.contains("Test Project"));
        assert!(csv.contains("ABC-123"));
        assert!(csv.contains("2,0,7200")); // 2 hours, 0 minutes, 7200 seconds
    }
}

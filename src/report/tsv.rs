use anyhow::Result;

use crate::models::MonthlyReport;

/// Generate TSV report (Tab-Separated Values for easy paste into Google Sheets)
pub fn generate_string(report: &MonthlyReport, include_commits: bool) -> Result<String> {
    let mut output = String::new();

    // Write header
    if include_commits {
        output.push_str("project\twork_item\tcompleted_date\thours\tminutes\ttotal_seconds\tcommits\n");
    } else {
        output.push_str("project\twork_item\tcompleted_date\thours\tminutes\ttotal_seconds\n");
    }

    // Write data rows
    for project in &report.projects {
        for item in &project.work_items {
            let hours = item.total_seconds / 3600;
            let minutes = (item.total_seconds % 3600) / 60;
            let date_str = item.completed_date.as_deref().unwrap_or("");

            // Escape tabs and newlines in text fields
            let project_name = escape_tsv(&project.name);
            let work_item = escape_tsv(&item.id);

            if include_commits {
                let commits_str = item
                    .commits
                    .iter()
                    .map(|c| c.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ");
                let commits_escaped = escape_tsv(&commits_str);

                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    project_name,
                    work_item,
                    date_str,
                    hours,
                    minutes,
                    item.total_seconds,
                    commits_escaped
                ));
            } else {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\n",
                    project_name,
                    work_item,
                    date_str,
                    hours,
                    minutes,
                    item.total_seconds
                ));
            }
        }
    }

    Ok(output)
}

/// Escape special characters for TSV format
fn escape_tsv(s: &str) -> String {
    s.replace('\t', " ").replace('\n', " ").replace('\r', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitSummary, ProjectReport, WorkItemReport};

    #[test]
    fn test_generate_tsv() {
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

        let tsv = generate_string(&report, true).unwrap();
        assert!(tsv.contains("project\twork_item\tcompleted_date"));
        assert!(tsv.contains("Test Project"));
        assert!(tsv.contains("ABC-123"));
        assert!(tsv.contains("2025-01-15"));
        assert!(tsv.contains("\t2\t0\t7200\t")); // hours, minutes, seconds
    }
}

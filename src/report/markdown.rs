use crate::models::MonthlyReport;
use crate::tracker::format_duration;

/// Generate markdown report
pub fn generate(report: &MonthlyReport, include_commits: bool) -> String {
    let mut output = String::new();

    // Header
    output.push_str("# Claude Code 工作時間報告\n\n");

    // Parse period for display
    let period_display = format_period(&report.period);
    output.push_str(&format!("**期間：** {}\n", period_display));
    output.push_str(&format!(
        "**總時數：** {}\n\n",
        format_duration(report.total_seconds)
    ));

    output.push_str("---\n\n");

    // Projects
    for project in &report.projects {
        output.push_str(&format!("## {}\n\n", project.name));
        output.push_str(&format!(
            "**小計：** {}\n\n",
            format_duration(project.total_seconds)
        ));

        // Work items table
        if include_commits {
            output.push_str("| 工作項 | 時間 | Commits |\n");
            output.push_str("|--------|------|----------|\n");
        } else {
            output.push_str("| 工作項 | 時間 |\n");
            output.push_str("|--------|------|\n");
        }

        for item in &project.work_items {
            let time_str = format_duration(item.total_seconds);

            if include_commits {
                let commits_str = if item.commits.is_empty() {
                    "-".to_string()
                } else {
                    item.commits
                        .iter()
                        .map(|c| c.message.clone())
                        .collect::<Vec<_>>()
                        .join("、")
                };

                output.push_str(&format!("| {} | {} | {} |\n", item.id, time_str, commits_str));
            } else {
                output.push_str(&format!("| {} | {} |\n", item.id, time_str));
            }
        }

        output.push_str("\n---\n\n");
    }

    output
}

fn format_period(period: &str) -> String {
    // Parse "2025-01" into "2025 年 1 月"
    let parts: Vec<&str> = period.split('-').collect();
    if parts.len() == 2 {
        if let (Ok(year), Ok(month)) = (parts[0].parse::<i32>(), parts[1].parse::<u32>()) {
            return format!("{} 年 {} 月", year, month);
        }
    }
    period.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CommitSummary, ProjectReport, WorkItemReport};

    #[test]
    fn test_generate_markdown() {
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

        let md = generate(&report, true);
        assert!(md.contains("Claude Code 工作時間報告"));
        assert!(md.contains("2025 年 1 月"));
        assert!(md.contains("Test Project"));
        assert!(md.contains("ABC-123"));
    }
}

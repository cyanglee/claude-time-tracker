pub mod csv;
pub mod json;
pub mod markdown;

use anyhow::{Context, Result};
use chrono::{Datelike, NaiveDate, TimeZone, Utc};
use std::collections::HashMap;

use crate::db::Database;
use crate::models::{CommitSummary, MonthlyReport, ProjectReport, WorkItemReport};

/// Generate report data for a given month
pub fn generate_report(
    db: &Database,
    year: i32,
    month: u32,
    project_filter: Option<&str>,
    max_commits_per_item: usize,
) -> Result<MonthlyReport> {
    // Calculate date range for the month
    let start = Utc
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .context("Invalid start date")?;

    let end = if month == 12 {
        Utc.with_ymd_and_hms(year + 1, 1, 1, 0, 0, 0)
    } else {
        Utc.with_ymd_and_hms(year, month + 1, 1, 0, 0, 0)
    }
    .single()
    .context("Invalid end date")?;

    // Get all projects
    let projects = db.list_projects()?;

    let mut project_reports = Vec::new();
    let mut total_seconds: i64 = 0;

    for project in projects {
        // Apply project filter if specified
        if let Some(filter) = project_filter {
            let name = project.display_name.as_deref().unwrap_or(&project.path);
            if !name.to_lowercase().contains(&filter.to_lowercase())
                && !project.path.to_lowercase().contains(&filter.to_lowercase())
            {
                continue;
            }
        }

        let sessions = db.get_sessions_in_range(start, end, Some(project.id))?;

        if sessions.is_empty() {
            continue;
        }

        // Group sessions by work item
        let mut work_items: HashMap<String, (i64, Vec<CommitSummary>, Option<String>)> = HashMap::new();

        for session in &sessions {
            let work_item_id = session
                .work_item
                .clone()
                .unwrap_or_else(|| session.branch.clone());

            let entry = work_items
                .entry(work_item_id)
                .or_insert_with(|| (0, Vec::new(), Some(session.branch.clone())));

            entry.0 += session.active_seconds.unwrap_or(0);

            // Get commits for this session
            if let Ok(commits) = db.get_commits(session.id) {
                for commit in commits {
                    if entry.1.len() < max_commits_per_item {
                        entry.1.push(CommitSummary {
                            hash: commit.hash[..8.min(commit.hash.len())].to_string(),
                            message: commit.message.unwrap_or_default(),
                        });
                    }
                }
            }
        }

        let project_total: i64 = work_items.values().map(|(s, _, _)| s).sum();

        if project_total == 0 {
            continue;
        }

        total_seconds += project_total;

        let mut work_item_reports: Vec<WorkItemReport> = work_items
            .into_iter()
            .map(|(id, (seconds, commits, branch))| WorkItemReport {
                id,
                branch,
                total_seconds: seconds,
                commits,
            })
            .collect();

        // Sort by time descending
        work_item_reports.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));

        project_reports.push(ProjectReport {
            name: project
                .display_name
                .unwrap_or_else(|| project.path.clone()),
            path: project.path,
            total_seconds: project_total,
            work_items: work_item_reports,
        });
    }

    // Sort projects by total time descending
    project_reports.sort_by(|a, b| b.total_seconds.cmp(&a.total_seconds));

    let period = format!("{}-{:02}", year, month);

    Ok(MonthlyReport {
        period,
        total_seconds,
        projects: project_reports,
    })
}

/// Parse month string (YYYY-MM) into year and month
pub fn parse_month(month_str: &str) -> Result<(i32, u32)> {
    let date = NaiveDate::parse_from_str(&format!("{}-01", month_str), "%Y-%m-%d")
        .with_context(|| format!("Invalid month format: {}. Expected YYYY-MM", month_str))?;

    Ok((date.year(), date.month()))
}

/// Get current year and month
pub fn current_month() -> (i32, u32) {
    let now = Utc::now();
    (now.year(), now.month())
}

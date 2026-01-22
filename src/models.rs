use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Project information stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub path: String,
    pub git_remote: Option<String>,
    pub display_name: Option<String>,
    pub work_item_pattern: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// A tracking session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: i64,
    pub project_id: i64,
    pub branch: String,
    pub work_item: Option<String>,
    pub start_commit: Option<String>,
    pub end_commit: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub active_seconds: Option<i64>,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    Active,
    Completed,
    Abandoned,
}

impl SessionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionStatus::Active => "active",
            SessionStatus::Completed => "completed",
            SessionStatus::Abandoned => "abandoned",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "active" => Some(SessionStatus::Active),
            "completed" => Some(SessionStatus::Completed),
            "abandoned" => Some(SessionStatus::Abandoned),
            _ => None,
        }
    }
}

/// A heartbeat timestamp within a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub id: i64,
    pub session_id: i64,
    pub timestamp: DateTime<Utc>,
}

/// A commit associated with a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub id: i64,
    pub session_id: i64,
    pub hash: String,
    pub message: Option<String>,
    pub committed_at: Option<DateTime<Utc>>,
}

/// Report data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectReport {
    pub name: String,
    pub path: String,
    pub total_seconds: i64,
    pub work_items: Vec<WorkItemReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemReport {
    pub id: String,
    pub branch: Option<String>,
    pub total_seconds: i64,
    pub completed_date: Option<String>,
    pub commits: Vec<CommitSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitSummary {
    pub hash: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonthlyReport {
    pub period: String,
    pub total_seconds: i64,
    pub projects: Vec<ProjectReport>,
}

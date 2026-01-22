use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

use crate::models::{Commit, Heartbeat, Project, Session, SessionStatus};

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path.display()))?;

        let db = Self { conn };
        db.initialize()?;
        Ok(db)
    }

    /// Initialize database schema
    fn initialize(&self) -> Result<()> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                git_remote TEXT,
                display_name TEXT,
                work_item_pattern TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY,
                project_id INTEGER NOT NULL REFERENCES projects(id),
                branch TEXT NOT NULL,
                work_item TEXT,
                start_commit TEXT,
                end_commit TEXT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                active_seconds INTEGER,
                status TEXT NOT NULL DEFAULT 'active'
            );

            CREATE TABLE IF NOT EXISTS heartbeats (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL REFERENCES sessions(id),
                timestamp TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS commits (
                id INTEGER PRIMARY KEY,
                session_id INTEGER NOT NULL REFERENCES sessions(id),
                hash TEXT NOT NULL,
                message TEXT,
                committed_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON sessions(project_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
            CREATE INDEX IF NOT EXISTS idx_heartbeats_session_id ON heartbeats(session_id);
            CREATE INDEX IF NOT EXISTS idx_commits_session_id ON commits(session_id);
            "#,
        )
        .context("Failed to initialize database schema")?;

        Ok(())
    }

    // ==================== Projects ====================

    /// Get or create a project by path
    pub fn get_or_create_project(
        &self,
        path: &str,
        git_remote: Option<&str>,
        display_name: Option<&str>,
        work_item_pattern: Option<&str>,
    ) -> Result<Project> {
        // Try to find existing project
        if let Some(project) = self.get_project_by_path(path)? {
            // Update if new info provided
            if git_remote.is_some() || display_name.is_some() || work_item_pattern.is_some() {
                self.conn.execute(
                    "UPDATE projects SET
                        git_remote = COALESCE(?, git_remote),
                        display_name = COALESCE(?, display_name),
                        work_item_pattern = COALESCE(?, work_item_pattern)
                    WHERE id = ?",
                    params![git_remote, display_name, work_item_pattern, project.id],
                )?;
                return self.get_project_by_id(project.id);
            }
            return Ok(project);
        }

        // Create new project
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO projects (path, git_remote, display_name, work_item_pattern, created_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                path,
                git_remote,
                display_name,
                work_item_pattern,
                now.to_rfc3339()
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        self.get_project_by_id(id)
    }

    /// Get project by ID
    pub fn get_project_by_id(&self, id: i64) -> Result<Project> {
        self.conn
            .query_row(
                "SELECT id, path, git_remote, display_name, work_item_pattern, created_at
                 FROM projects WHERE id = ?",
                params![id],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        git_remote: row.get(2)?,
                        display_name: row.get(3)?,
                        work_item_pattern: row.get(4)?,
                        created_at: parse_datetime(row.get::<_, String>(5)?),
                    })
                },
            )
            .context("Project not found")
    }

    /// Get project by path
    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>> {
        self.conn
            .query_row(
                "SELECT id, path, git_remote, display_name, work_item_pattern, created_at
                 FROM projects WHERE path = ?",
                params![path],
                |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        git_remote: row.get(2)?,
                        display_name: row.get(3)?,
                        work_item_pattern: row.get(4)?,
                        created_at: parse_datetime(row.get::<_, String>(5)?),
                    })
                },
            )
            .optional()
            .context("Failed to query project")
    }

    /// List all projects
    pub fn list_projects(&self) -> Result<Vec<Project>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, path, git_remote, display_name, work_item_pattern, created_at
             FROM projects ORDER BY path",
        )?;

        let projects = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    git_remote: row.get(2)?,
                    display_name: row.get(3)?,
                    work_item_pattern: row.get(4)?,
                    created_at: parse_datetime(row.get::<_, String>(5)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }

    // ==================== Sessions ====================

    /// Create a new session
    pub fn create_session(
        &self,
        project_id: i64,
        branch: &str,
        work_item: Option<&str>,
        start_commit: Option<&str>,
    ) -> Result<Session> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO sessions (project_id, branch, work_item, start_commit, started_at, status)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                project_id,
                branch,
                work_item,
                start_commit,
                now.to_rfc3339(),
                SessionStatus::Active.as_str()
            ],
        )?;

        let id = self.conn.last_insert_rowid();
        self.get_session_by_id(id)
    }

    /// Get session by ID
    pub fn get_session_by_id(&self, id: i64) -> Result<Session> {
        self.conn
            .query_row(
                "SELECT id, project_id, branch, work_item, start_commit, end_commit,
                        started_at, ended_at, active_seconds, status
                 FROM sessions WHERE id = ?",
                params![id],
                row_to_session,
            )
            .context("Session not found")
    }

    /// Get active session for a project
    pub fn get_active_session(&self, project_id: i64) -> Result<Option<Session>> {
        self.conn
            .query_row(
                "SELECT id, project_id, branch, work_item, start_commit, end_commit,
                        started_at, ended_at, active_seconds, status
                 FROM sessions WHERE project_id = ? AND status = 'active'
                 ORDER BY started_at DESC LIMIT 1",
                params![project_id],
                row_to_session,
            )
            .optional()
            .context("Failed to query active session")
    }

    /// Get all active sessions (for cleanup)
    pub fn get_all_active_sessions(&self) -> Result<Vec<Session>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, project_id, branch, work_item, start_commit, end_commit,
                    started_at, ended_at, active_seconds, status
             FROM sessions WHERE status = 'active'",
        )?;

        let sessions = stmt
            .query_map([], row_to_session)?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(sessions)
    }

    /// Update session end state
    pub fn complete_session(
        &self,
        session_id: i64,
        end_commit: Option<&str>,
        active_seconds: i64,
        status: SessionStatus,
    ) -> Result<()> {
        let now = Utc::now();
        self.conn.execute(
            "UPDATE sessions SET ended_at = ?, end_commit = ?, active_seconds = ?, status = ?
             WHERE id = ?",
            params![
                now.to_rfc3339(),
                end_commit,
                active_seconds,
                status.as_str(),
                session_id
            ],
        )?;
        Ok(())
    }

    /// Get sessions within a time range
    pub fn get_sessions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        project_id: Option<i64>,
    ) -> Result<Vec<Session>> {
        let query = if project_id.is_some() {
            "SELECT id, project_id, branch, work_item, start_commit, end_commit,
                    started_at, ended_at, active_seconds, status
             FROM sessions
             WHERE started_at >= ? AND started_at < ? AND project_id = ? AND status != 'active'
             ORDER BY started_at"
        } else {
            "SELECT id, project_id, branch, work_item, start_commit, end_commit,
                    started_at, ended_at, active_seconds, status
             FROM sessions
             WHERE started_at >= ? AND started_at < ? AND status != 'active'
             ORDER BY started_at"
        };

        let mut stmt = self.conn.prepare(query)?;

        let sessions = if let Some(pid) = project_id {
            stmt.query_map(
                params![start.to_rfc3339(), end.to_rfc3339(), pid],
                row_to_session,
            )?
        } else {
            stmt.query_map(params![start.to_rfc3339(), end.to_rfc3339()], row_to_session)?
        };

        sessions.collect::<Result<Vec<_>, _>>().context("Failed to query sessions")
    }

    // ==================== Heartbeats ====================

    /// Record a heartbeat
    pub fn record_heartbeat(&self, session_id: i64) -> Result<Heartbeat> {
        let now = Utc::now();
        self.conn.execute(
            "INSERT INTO heartbeats (session_id, timestamp) VALUES (?, ?)",
            params![session_id, now.to_rfc3339()],
        )?;

        Ok(Heartbeat {
            id: self.conn.last_insert_rowid(),
            session_id,
            timestamp: now,
        })
    }

    /// Get heartbeats for a session
    pub fn get_heartbeats(&self, session_id: i64) -> Result<Vec<Heartbeat>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, timestamp FROM heartbeats
             WHERE session_id = ? ORDER BY timestamp",
        )?;

        let heartbeats = stmt
            .query_map(params![session_id], |row| {
                Ok(Heartbeat {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    timestamp: parse_datetime(row.get::<_, String>(2)?),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(heartbeats)
    }

    /// Get last heartbeat for a session
    pub fn get_last_heartbeat(&self, session_id: i64) -> Result<Option<Heartbeat>> {
        self.conn
            .query_row(
                "SELECT id, session_id, timestamp FROM heartbeats
                 WHERE session_id = ? ORDER BY timestamp DESC LIMIT 1",
                params![session_id],
                |row| {
                    Ok(Heartbeat {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        timestamp: parse_datetime(row.get::<_, String>(2)?),
                    })
                },
            )
            .optional()
            .context("Failed to query last heartbeat")
    }

    // ==================== Commits ====================

    /// Record commits for a session
    pub fn record_commits(&self, session_id: i64, commits: &[(String, String, Option<DateTime<Utc>>)]) -> Result<()> {
        for (hash, message, committed_at) in commits {
            self.conn.execute(
                "INSERT INTO commits (session_id, hash, message, committed_at) VALUES (?, ?, ?, ?)",
                params![
                    session_id,
                    hash,
                    message,
                    committed_at.map(|dt| dt.to_rfc3339())
                ],
            )?;
        }
        Ok(())
    }

    /// Get commits for a session
    pub fn get_commits(&self, session_id: i64) -> Result<Vec<Commit>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, session_id, hash, message, committed_at FROM commits
             WHERE session_id = ? ORDER BY committed_at",
        )?;

        let commits = stmt
            .query_map(params![session_id], |row| {
                Ok(Commit {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    hash: row.get(2)?,
                    message: row.get(3)?,
                    committed_at: row
                        .get::<_, Option<String>>(4)?
                        .map(parse_datetime),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(commits)
    }
}

fn row_to_session(row: &rusqlite::Row) -> rusqlite::Result<Session> {
    Ok(Session {
        id: row.get(0)?,
        project_id: row.get(1)?,
        branch: row.get(2)?,
        work_item: row.get(3)?,
        start_commit: row.get(4)?,
        end_commit: row.get(5)?,
        started_at: parse_datetime(row.get::<_, String>(6)?),
        ended_at: row.get::<_, Option<String>>(7)?.map(parse_datetime),
        active_seconds: row.get(8)?,
        status: SessionStatus::from_str(&row.get::<_, String>(9)?).unwrap_or(SessionStatus::Active),
    })
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_database_creation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        // Should be able to create a project
        let project = db.get_or_create_project("/test/path", None, None, None).unwrap();
        assert_eq!(project.path, "/test/path");
    }

    #[test]
    fn test_session_lifecycle() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();

        let project = db.get_or_create_project("/test/path", None, None, None).unwrap();

        // Create session
        let session = db.create_session(project.id, "main", None, None).unwrap();
        assert_eq!(session.status, SessionStatus::Active);

        // Record heartbeats
        db.record_heartbeat(session.id).unwrap();
        db.record_heartbeat(session.id).unwrap();

        let heartbeats = db.get_heartbeats(session.id).unwrap();
        assert_eq!(heartbeats.len(), 2);

        // Complete session
        db.complete_session(session.id, None, 3600, SessionStatus::Completed).unwrap();

        let completed = db.get_session_by_id(session.id).unwrap();
        assert_eq!(completed.status, SessionStatus::Completed);
        assert_eq!(completed.active_seconds, Some(3600));
    }
}

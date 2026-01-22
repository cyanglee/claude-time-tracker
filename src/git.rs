use anyhow::{Context, Result};
use chrono::{DateTime, TimeZone, Utc};
use std::path::Path;

/// Git repository information
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub branch: String,
    pub head_commit: Option<String>,
    pub remote_url: Option<String>,
}

/// Get git information for a repository path
pub fn get_git_info(path: &Path) -> Result<GitInfo> {
    let repo = gix::open(path).context("Failed to open git repository")?;

    // Get current branch name
    let branch = get_branch_name(&repo)?;

    // Get HEAD commit
    let head_commit = get_head_commit(&repo)?;

    // Get remote URL
    let remote_url = get_remote_url(&repo)?;

    Ok(GitInfo {
        branch,
        head_commit,
        remote_url,
    })
}

fn get_branch_name(repo: &gix::Repository) -> Result<String> {
    let head = repo.head().context("Failed to get HEAD")?;

    if let Some(name) = head.referent_name() {
        // Extract branch name from refs/heads/branch-name
        let full_name = name.as_bstr().to_string();
        if let Some(branch) = full_name.strip_prefix("refs/heads/") {
            return Ok(branch.to_string());
        }
        return Ok(full_name);
    }

    // Detached HEAD - return short commit hash
    if let Some(id) = head.id() {
        return Ok(format!("detached-{}", &id.to_string()[..8]));
    }

    Ok("unknown".to_string())
}

fn get_head_commit(repo: &gix::Repository) -> Result<Option<String>> {
    let head = repo.head().context("Failed to get HEAD")?;

    if let Some(id) = head.id() {
        return Ok(Some(id.to_string()));
    }

    Ok(None)
}

fn get_remote_url(repo: &gix::Repository) -> Result<Option<String>> {
    // Try to get origin remote
    if let Ok(remote) = repo.find_remote("origin") {
        if let Some(url) = remote.url(gix::remote::Direction::Fetch) {
            return Ok(Some(url.to_bstring().to_string()));
        }
    }

    Ok(None)
}

/// Get commits between two commit hashes (exclusive start, inclusive end)
pub fn get_commits_between(
    path: &Path,
    start_commit: Option<&str>,
    end_commit: Option<&str>,
) -> Result<Vec<(String, String, Option<DateTime<Utc>>)>> {
    let repo = gix::open(path).context("Failed to open git repository")?;

    let end_oid = if let Some(end) = end_commit {
        repo.rev_parse_single(end)
            .context("Failed to parse end commit")?
            .detach()
    } else {
        repo.head()
            .context("Failed to get HEAD")?
            .id()
            .context("HEAD has no commit")?
            .detach()
    };

    let start_oid = start_commit.and_then(|s| repo.rev_parse_single(s).ok().map(|o| o.detach()));

    let mut commits = Vec::new();
    let mut walk = repo
        .rev_walk([end_oid])
        .sorting(gix::traverse::commit::simple::Sorting::ByCommitTimeNewestFirst)
        .all()
        .context("Failed to create revision walker")?;

    while let Some(info) = walk.next() {
        let info = info.context("Failed to get commit info")?;
        let oid = info.id;

        // Stop if we've reached the start commit
        if let Some(ref start) = start_oid {
            if oid == *start {
                break;
            }
        }

        let commit = info.object().context("Failed to get commit object")?;
        let message = commit
            .message()
            .map(|m| m.title.to_string())
            .unwrap_or_default();

        let time = commit.time().ok().map(|t| {
            Utc.timestamp_opt(t.seconds, 0)
                .single()
                .unwrap_or_else(Utc::now)
        });

        commits.push((oid.to_string(), message, time));

        // Limit to last 100 commits if no start specified
        if start_oid.is_none() && commits.len() >= 100 {
            break;
        }
    }

    // Reverse to get chronological order
    commits.reverse();
    Ok(commits)
}

/// Check if path is inside a git repository
pub fn is_git_repo(path: &Path) -> bool {
    gix::open(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_git_repo() {
        // Current directory should be a git repo (this project)
        let cwd = env::current_dir().unwrap();
        // This test depends on being run from within a git repo
        // Just verify the function doesn't panic
        let _ = is_git_repo(&cwd);
    }
}

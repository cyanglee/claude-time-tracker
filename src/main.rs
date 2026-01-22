mod cli;
mod config;
mod db;
mod git;
mod models;
mod report;
mod tracker;

use anyhow::{Context, Result};
use chrono::Utc;
use clap::Parser;
use std::fs;
use std::path::PathBuf;

use cli::{Cli, Commands, ConfigAction, ProjectsAction};
use config::EffectiveConfig;
use db::Database;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { path } => cmd_start(&path),
        Commands::Heartbeat { path } => cmd_heartbeat(&path),
        Commands::Stop { path } => cmd_stop(&path),
        Commands::Report {
            month,
            project,
            format,
            output,
            all_formats,
        } => cmd_report(month, project, format, output, all_formats),
        Commands::Status => cmd_status(),
        Commands::Config { action } => match action {
            ConfigAction::Init => cmd_config_init(),
            ConfigAction::Edit => cmd_config_edit(),
            ConfigAction::Show => cmd_config_show(),
        },
        Commands::Projects { action } => match action {
            ProjectsAction::List => cmd_projects_list(),
            ProjectsAction::SetName { path, name } => cmd_projects_set_name(&path, &name),
        },
    }
}

fn get_db() -> Result<Database> {
    let config = EffectiveConfig::load(None)?;
    Database::open(&config.database_path)
}

fn cmd_start(path: &str) -> Result<()> {
    let project_path = PathBuf::from(path).canonicalize()
        .with_context(|| format!("Invalid path: {}", path))?;

    let config = EffectiveConfig::load(Some(&project_path))?;
    let db = Database::open(&config.database_path)?;

    tracker::start_session(&db, &project_path, &config)
}

fn cmd_heartbeat(path: &str) -> Result<()> {
    let project_path = PathBuf::from(path).canonicalize()
        .with_context(|| format!("Invalid path: {}", path))?;

    let config = EffectiveConfig::load(Some(&project_path))?;
    let db = Database::open(&config.database_path)?;

    tracker::record_heartbeat(&db, &project_path)
}

fn cmd_stop(path: &str) -> Result<()> {
    let project_path = PathBuf::from(path).canonicalize()
        .with_context(|| format!("Invalid path: {}", path))?;

    let config = EffectiveConfig::load(Some(&project_path))?;
    let db = Database::open(&config.database_path)?;

    tracker::stop_session(&db, &project_path, &config)
}

fn cmd_report(
    month: Option<String>,
    project_filter: Option<String>,
    format: String,
    output: Option<String>,
    all_formats: bool,
) -> Result<()> {
    let config = EffectiveConfig::load(None)?;
    let db = Database::open(&config.database_path)?;

    // Parse month
    let (year, month_num) = if let Some(ref m) = month {
        report::parse_month(m)?
    } else {
        report::current_month()
    };

    // Generate report data
    let report_data = report::generate_report(
        &db,
        year,
        month_num,
        project_filter.as_deref(),
        config.max_commits_per_item,
    )?;

    // Determine formats to output
    let formats: Vec<&str> = if all_formats {
        vec!["md", "csv", "json"]
    } else {
        format.split(',').map(|s| s.trim()).collect()
    };

    let multiple_formats = formats.len() > 1;

    // Generate and output reports
    for fmt in formats {
        let content = match fmt {
            "md" | "markdown" => report::markdown::generate(&report_data, config.include_commits),
            "csv" => report::csv::generate_string(&report_data, config.include_commits)?,
            "json" => report::json::generate(&report_data)?,
            _ => {
                eprintln!("Unknown format: {}", fmt);
                continue;
            }
        };

        if let Some(ref base_path) = output {
            let ext = match fmt {
                "md" | "markdown" => "md",
                "csv" => "csv",
                "json" => "json",
                _ => fmt,
            };
            let file_path = if multiple_formats {
                format!("{}.{}", base_path, ext)
            } else if base_path.ends_with(&format!(".{}", ext)) {
                base_path.clone()
            } else {
                format!("{}.{}", base_path, ext)
            };

            fs::write(&file_path, &content)
                .with_context(|| format!("Failed to write report to {}", file_path))?;
            eprintln!("Report written to: {}", file_path);
        } else {
            println!("{}", content);
        }
    }

    Ok(())
}

fn cmd_status() -> Result<()> {
    let config = EffectiveConfig::load(None)?;
    let db = Database::open(&config.database_path)?;

    let active_sessions = db.get_all_active_sessions()?;

    if active_sessions.is_empty() {
        println!("No active tracking sessions.");
        return Ok(());
    }

    println!("Active tracking sessions:\n");

    for session in active_sessions {
        let project = db.get_project_by_id(session.project_id)?;
        let heartbeats = db.get_heartbeats(session.id)?;

        let elapsed = calculate_active_time_with_current(&heartbeats, config.idle_timeout_minutes);

        println!(
            "  Project: {}",
            project.display_name.as_deref().unwrap_or(&project.path)
        );
        println!("  Branch:  {}", session.branch);
        println!("  Started: {}", session.started_at);
        println!("  Active:  {}", tracker::format_duration(elapsed));
        println!();
    }

    Ok(())
}

/// Calculate active time including time since last heartbeat (for status display)
fn calculate_active_time_with_current(heartbeats: &[models::Heartbeat], idle_timeout_minutes: u32) -> i64 {
    if heartbeats.is_empty() {
        return 0;
    }

    let timeout_seconds = (idle_timeout_minutes as i64) * 60;
    let mut total_seconds: i64 = 0;

    for window in heartbeats.windows(2) {
        let interval = (window[1].timestamp - window[0].timestamp).num_seconds();

        if interval <= timeout_seconds {
            total_seconds += interval;
        }
    }

    // Add time from last heartbeat to now (if within timeout)
    if let Some(last) = heartbeats.last() {
        let since_last = (Utc::now() - last.timestamp).num_seconds();
        if since_last <= timeout_seconds {
            total_seconds += since_last;
        }
    }

    total_seconds
}

fn cmd_config_init() -> Result<()> {
    let path = config::init_global_config()?;
    println!("Configuration initialized at: {}", path.display());
    Ok(())
}

fn cmd_config_edit() -> Result<()> {
    let path = config::global_config_path()?;

    if !path.exists() {
        config::init_global_config()?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    Ok(())
}

fn cmd_config_show() -> Result<()> {
    let config = config::load_global_config()?;
    let toml = toml::to_string_pretty(&config)?;
    println!("{}", toml);
    Ok(())
}

fn cmd_projects_list() -> Result<()> {
    let db = get_db()?;
    let projects = db.list_projects()?;

    if projects.is_empty() {
        println!("No tracked projects yet.");
        return Ok(());
    }

    println!("Tracked projects:\n");

    for project in projects {
        let name = project.display_name.as_deref().unwrap_or("-");
        println!("  Path: {}", project.path);
        println!("  Name: {}", name);
        if let Some(ref remote) = project.git_remote {
            println!("  Remote: {}", remote);
        }
        println!();
    }

    Ok(())
}

fn cmd_projects_set_name(path: &str, name: &str) -> Result<()> {
    let db = get_db()?;

    let project_path = PathBuf::from(path).canonicalize()
        .with_context(|| format!("Invalid path: {}", path))?;

    let path_str = project_path.to_str().context("Invalid path")?;

    db.get_or_create_project(path_str, None, Some(name), None)?;

    println!("Set display name for {} to: {}", path_str, name);
    Ok(())
}

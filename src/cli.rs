use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claude-time-tracker")]
#[command(about = "Track Claude Code usage time per project", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start tracking a new session (called by SessionStart hook)
    Start {
        /// Project path
        #[arg(short, long)]
        path: String,
    },

    /// Record activity heartbeat (called by UserPromptSubmit hook)
    Heartbeat {
        /// Project path
        #[arg(short, long)]
        path: String,
    },

    /// Stop tracking current session (called by Stop hook)
    Stop {
        /// Project path
        #[arg(short, long)]
        path: String,
    },

    /// Generate time tracking report
    Report {
        /// Month to report (YYYY-MM format), defaults to current month
        #[arg(short, long)]
        month: Option<String>,

        /// Filter by project name or path
        #[arg(short = 'P', long)]
        project: Option<String>,

        /// Output format: md, csv, json (can specify multiple, comma-separated)
        #[arg(short, long, default_value = "md")]
        format: String,

        /// Output file path (without extension if multiple formats)
        #[arg(short, long)]
        output: Option<String>,

        /// Output all formats (md, csv, json)
        #[arg(long)]
        all_formats: bool,
    },

    /// Show current tracking status
    Status,

    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Manage projects
    Projects {
        #[command(subcommand)]
        action: ProjectsAction,
    },
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Initialize default configuration
    Init,
    /// Open configuration file in editor
    Edit,
    /// Show current configuration
    Show,
}

#[derive(Subcommand)]
pub enum ProjectsAction {
    /// List all tracked projects
    List,
    /// Set display name for a project
    SetName {
        /// Project path
        path: String,
        /// Display name
        name: String,
    },
}

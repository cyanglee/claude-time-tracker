use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Global configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default)]
    pub settings: Settings,
    #[serde(default)]
    pub report: ReportSettings,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            report: ReportSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_minutes: u32,
    #[serde(default = "default_database_path")]
    pub database_path: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            idle_timeout_minutes: default_idle_timeout(),
            database_path: default_database_path(),
        }
    }
}

fn default_idle_timeout() -> u32 {
    10
}

fn default_database_path() -> String {
    "~/.local/share/claude-time-tracker/data.db".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSettings {
    #[serde(default = "default_format")]
    pub default_format: String,
    #[serde(default = "default_include_commits")]
    pub include_commits: bool,
    #[serde(default = "default_max_commits")]
    pub max_commits_per_item: usize,
}

impl Default for ReportSettings {
    fn default() -> Self {
        Self {
            default_format: default_format(),
            include_commits: default_include_commits(),
            max_commits_per_item: default_max_commits(),
        }
    }
}

fn default_format() -> String {
    "markdown".to_string()
}

fn default_include_commits() -> bool {
    true
}

fn default_max_commits() -> usize {
    10
}

/// Project-specific configuration (found in project directory)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub work_item_pattern: Option<String>,
    #[serde(default)]
    pub report: ProjectReportSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectReportSettings {
    pub include_commits: Option<bool>,
    pub max_commits_per_item: Option<usize>,
}

/// Merged configuration for a specific project
#[derive(Debug, Clone)]
pub struct EffectiveConfig {
    pub idle_timeout_minutes: u32,
    pub database_path: PathBuf,
    pub project_name: Option<String>,
    pub work_item_pattern: Option<String>,
    pub include_commits: bool,
    pub max_commits_per_item: usize,
}

impl EffectiveConfig {
    /// Load configuration with project-specific overrides
    pub fn load(project_path: Option<&Path>) -> Result<Self> {
        let global = load_global_config()?;
        let project = project_path.and_then(|p| load_project_config(p).ok());

        let database_path = expand_path(&global.settings.database_path)?;

        Ok(Self {
            idle_timeout_minutes: global.settings.idle_timeout_minutes,
            database_path,
            project_name: project.as_ref().and_then(|p| p.name.clone()),
            work_item_pattern: project.as_ref().and_then(|p| p.work_item_pattern.clone()),
            include_commits: project
                .as_ref()
                .and_then(|p| p.report.include_commits)
                .unwrap_or(global.report.include_commits),
            max_commits_per_item: project
                .as_ref()
                .and_then(|p| p.report.max_commits_per_item)
                .unwrap_or(global.report.max_commits_per_item),
        })
    }
}

/// Get the global config directory path
pub fn global_config_dir() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("claude-time-tracker");
    Ok(config_dir)
}

/// Get the global config file path
pub fn global_config_path() -> Result<PathBuf> {
    Ok(global_config_dir()?.join("config.toml"))
}

/// Load global configuration from ~/.config/claude-time-tracker/config.toml
pub fn load_global_config() -> Result<GlobalConfig> {
    let config_path = global_config_path()?;

    if !config_path.exists() {
        return Ok(GlobalConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;

    let config: GlobalConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;

    Ok(config)
}

/// Load project-specific configuration from <project>/.claude-time-tracker.toml
pub fn load_project_config(project_path: &Path) -> Result<ProjectConfig> {
    let config_path = project_path.join(".claude-time-tracker.toml");

    if !config_path.exists() {
        return Ok(ProjectConfig::default());
    }

    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read project config: {}", config_path.display()))?;

    let config: ProjectConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse project config: {}", config_path.display()))?;

    Ok(config)
}

/// Initialize global config directory and create default config if not exists
pub fn init_global_config() -> Result<PathBuf> {
    let config_dir = global_config_dir()?;
    std::fs::create_dir_all(&config_dir)
        .with_context(|| format!("Failed to create config directory: {}", config_dir.display()))?;

    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        let default_config = GlobalConfig::default();
        let content = toml::to_string_pretty(&default_config)
            .context("Failed to serialize default config")?;
        std::fs::write(&config_path, content)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
    }

    Ok(config_path)
}

/// Expand ~ and environment variables in path
pub fn expand_path(path: &str) -> Result<PathBuf> {
    let expanded = shellexpand::full(path)
        .with_context(|| format!("Failed to expand path: {}", path))?;
    Ok(PathBuf::from(expanded.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GlobalConfig::default();
        assert_eq!(config.settings.idle_timeout_minutes, 10);
        assert_eq!(config.report.default_format, "markdown");
    }

    #[test]
    fn test_expand_path() {
        let expanded = expand_path("~/.config/test").unwrap();
        assert!(expanded.to_string_lossy().contains("/.config/test"));
        assert!(!expanded.to_string_lossy().starts_with("~"));
    }
}

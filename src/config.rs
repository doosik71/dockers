#![allow(dead_code)]

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub docker: DockerConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            docker: DockerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub command: String,
    pub default_args: Vec<String>,
    pub output_format: DockerOutputFormat,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            command: "docker".to_string(),
            default_args: Vec::new(),
            output_format: DockerOutputFormat::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum DockerOutputFormat {
    #[default]
    Json,
}

#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub write_to_file: bool,
    pub log_dir: PathBuf,
    pub include_target: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            write_to_file: false,
            log_dir: default_log_dir(),
            include_target: false,
        }
    }
}

fn default_log_dir() -> PathBuf {
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dockers")
            .join("logs")
    } else {
        std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dockers")
            .join("logs")
    }
}

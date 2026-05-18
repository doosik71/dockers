use serde::Deserialize;

use crate::config::{DockerConfig, DockerOutputFormat};
use crate::process::{CommandOutput, DockerCommandRunner};

#[derive(Debug, Clone)]
pub struct DockerEnvironmentStatus {
    pub installation: StatusLine,
    pub daemon: StatusLine,
    pub version: StatusLine,
    pub output_strategy: StatusLine,
}

#[derive(Debug, Clone)]
pub struct StatusLine {
    pub label: &'static str,
    pub level: StatusLevel,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Ok,
    Warning,
    Error,
    Info,
}

pub struct DockerService {
    runner: DockerCommandRunner,
    output_format: DockerOutputFormat,
}

impl DockerService {
    pub fn new(config: &DockerConfig) -> Self {
        Self {
            runner: DockerCommandRunner::new(config.command.clone(), config.default_args.clone()),
            output_format: config.output_format,
        }
    }

    pub fn inspect_environment(&self) -> DockerEnvironmentStatus {
        let installation_output = self.runner.run_captured(["--version"]);

        let installation = match installation_output {
            Ok(output) if output.status_code == Some(0) => StatusLine::ok(
                "Docker installed",
                non_empty_or_fallback(output.stdout, "docker command is available"),
            ),
            Ok(output) => StatusLine::error(
                "Docker installed",
                format_command_failure(&output, "docker command returned an error"),
            ),
            Err(error) => StatusLine::error("Docker installed", error.to_string()),
        };

        if installation.level == StatusLevel::Error {
            return DockerEnvironmentStatus {
                installation,
                daemon: StatusLine::warning(
                    "Docker daemon",
                    "skipped because docker command is not available",
                ),
                version: StatusLine::warning(
                    "Docker version",
                    "skipped because docker command is not available",
                ),
                output_strategy: self.output_strategy_status(),
            };
        }

        let daemon = self.inspect_daemon();
        let version = self.inspect_version();

        DockerEnvironmentStatus {
            installation,
            daemon,
            version,
            output_strategy: self.output_strategy_status(),
        }
    }

    fn inspect_daemon(&self) -> StatusLine {
        match self.runner.run_captured(["info", "--format", "{{json .ServerVersion}}"]) {
            Ok(output) if output.status_code == Some(0) => {
                let server_version = self.parse_json_string(&output).unwrap_or_else(|_| "available".to_string());
                StatusLine::ok("Docker daemon", format!("reachable (server {server_version})"))
            }
            Ok(output) => StatusLine::warning(
                "Docker daemon",
                format_command_failure(&output, "docker daemon is not reachable"),
            ),
            Err(error) => StatusLine::error("Docker daemon", error.to_string()),
        }
    }

    fn inspect_version(&self) -> StatusLine {
        match self.runner.run_captured(["version", "--format", "{{json .}}"]) {
            Ok(output) if output.status_code == Some(0) => match serde_json::from_str::<DockerVersionInfo>(&output.stdout) {
                Ok(version) => {
                    let client = version.client.version.unwrap_or_else(|| "unknown".to_string());
                    let server = version
                        .server
                        .and_then(|server| server.version)
                        .unwrap_or_else(|| "unavailable".to_string());
                    StatusLine::ok("Docker version", format!("client {client}, server {server}"))
                }
                Err(error) => StatusLine::warning(
                    "Docker version",
                    format!("version command succeeded but JSON parsing failed: {error}"),
                ),
            },
            Ok(output) => StatusLine::warning(
                "Docker version",
                format_command_failure(&output, "failed to query docker version"),
            ),
            Err(error) => StatusLine::error("Docker version", error.to_string()),
        }
    }

    fn output_strategy_status(&self) -> StatusLine {
        let detail = match self.output_format {
            DockerOutputFormat::Json => {
                "JSON-first strategy: use `--format {{json .}}` for objects and `{{json .Field}}` for scalar values"
            }
        };

        StatusLine::info("Output strategy", detail)
    }

    fn parse_json_string(&self, output: &CommandOutput) -> Result<String, serde_json::Error> {
        serde_json::from_str::<String>(&output.stdout)
    }
}

impl StatusLine {
    pub fn ok(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            level: StatusLevel::Ok,
            detail: detail.into(),
        }
    }

    pub fn warning(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            level: StatusLevel::Warning,
            detail: detail.into(),
        }
    }

    pub fn error(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            level: StatusLevel::Error,
            detail: detail.into(),
        }
    }

    pub fn info(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            level: StatusLevel::Info,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct DockerVersionInfo {
    #[serde(rename = "Client")]
    client: DockerVersionComponent,
    #[serde(rename = "Server")]
    server: Option<DockerVersionComponent>,
}

#[derive(Debug, Deserialize)]
struct DockerVersionComponent {
    #[serde(rename = "Version")]
    version: Option<String>,
}

fn format_command_failure(output: &CommandOutput, fallback: &str) -> String {
    let stderr = if output.stderr.is_empty() {
        fallback.to_string()
    } else {
        output.stderr.clone()
    };

    format!(
        "exit code {:?}: {}",
        output.status_code,
        stderr.replace('\n', " ").trim()
    )
}

fn non_empty_or_fallback(value: String, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value
    }
}

use serde::Deserialize;

use crate::config::{DockerConfig, DockerOutputFormat};
use crate::process::{CommandOutput, DockerCommandRunner};

#[derive(Debug, Clone)]
pub struct DockerOverview {
    pub environment: DockerEnvironmentStatus,
    pub resources: DockerResourceOverview,
}

#[derive(Debug, Clone)]
pub struct DockerEnvironmentStatus {
    pub installation: StatusLine,
    pub daemon: StatusLine,
    pub version: StatusLine,
    pub output_strategy: StatusLine,
}

#[derive(Debug, Clone)]
pub struct DockerResourceOverview {
    pub containers: ResourceQuery<ContainerSummary>,
    pub images: ResourceQuery<ImageSummary>,
    pub volumes: ResourceQuery<VolumeSummary>,
    pub networks: ResourceQuery<NetworkSummary>,
}

#[derive(Debug, Clone)]
pub struct ContainerCreateRequest {
    pub image: String,
    pub name: Option<String>,
    pub ports: Vec<String>,
    pub volumes: Vec<String>,
    pub env: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResourceQuery<T> {
    pub label: &'static str,
    pub items: Vec<T>,
    pub status: StatusLine,
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

#[derive(Debug, Clone, Deserialize)]
pub struct ContainerSummary {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Names")]
    pub names: String,
    #[serde(rename = "Image")]
    pub image: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "Status")]
    pub status: String,
    #[serde(rename = "Ports")]
    pub ports: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageSummary {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Repository")]
    pub repository: String,
    #[serde(rename = "Tag")]
    pub tag: String,
    #[serde(rename = "CreatedSince")]
    pub created_since: String,
    #[serde(rename = "Size")]
    pub size: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageSearchResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "StarCount")]
    pub stars: String,
    #[serde(rename = "IsOfficial")]
    pub official: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VolumeSummary {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Driver")]
    pub driver: String,
    #[serde(rename = "Mountpoint")]
    pub mountpoint: String,
    #[serde(rename = "Scope")]
    pub scope: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkSummary {
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Driver")]
    pub driver: String,
    #[serde(rename = "Scope")]
    pub scope: String,
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

    pub fn inspect(&self) -> DockerOverview {
        let environment = self.inspect_environment();
        let resources = self.inspect_resources(&environment);

        DockerOverview {
            environment,
            resources,
        }
    }

    pub fn start_container(&self, container_id: &str) -> Result<String, String> {
        self.run_container_action(["start", container_id], "container started")
    }

    pub fn stop_container(&self, container_id: &str) -> Result<String, String> {
        self.run_container_action(["stop", container_id], "container stopped")
    }

    pub fn restart_container(&self, container_id: &str) -> Result<String, String> {
        self.run_container_action(["restart", container_id], "container restarted")
    }

    pub fn remove_container(&self, container_id: &str) -> Result<String, String> {
        self.run_container_action(["rm", "-f", container_id], "container removed")
    }

    pub fn inspect_container(&self, container_id: &str) -> Result<String, String> {
        self.runner
            .run(["inspect", container_id])
            .map(|output| output.stdout)
            .map_err(|error| error.to_string())
    }

    pub fn container_logs(&self, container_id: &str) -> Result<String, String> {
        self.runner
            .run(["logs", "--tail", "200", container_id])
            .map(|output| {
                if output.stdout.is_empty() {
                    "No container logs available.".to_string()
                } else {
                    output.stdout
                }
            })
            .map_err(|error| error.to_string())
    }

    pub fn open_container_shell(&self, container_id: &str) -> Result<(), String> {
        match self.runner.run_interactive(["exec", "-it", container_id, "/bin/sh"]) {
            Ok(()) => Ok(()),
            Err(sh_error) => self
                .runner
                .run_interactive(["exec", "-it", container_id, "/bin/bash"])
                .map_err(|bash_error| {
                    format!(
                        "failed to open shell with /bin/sh ({sh_error}); fallback /bin/bash also failed ({bash_error})"
                    )
                }),
        }
    }

    pub fn remove_image(&self, image_id: &str) -> Result<String, String> {
        self.run_simple_action(["image", "rm", image_id], "image removed")
    }

    pub fn inspect_image(&self, image_id: &str) -> Result<String, String> {
        self.runner
            .run(["image", "inspect", image_id])
            .map(|output| output.stdout)
            .map_err(|error| error.to_string())
    }

    pub fn search_images(&self, query: &str) -> Result<Vec<ImageSearchResult>, String> {
        match self
            .runner
            .run_captured(["search", "--format", "{{json .}}", query])
        {
            Ok(output) if output.status_code == Some(0) => {
                parse_json_lines::<ImageSearchResult>(&output.stdout)
                    .map_err(|error| format!("failed to parse search results: {error}"))
            }
            Ok(output) => Err(format_command_failure(&output, "failed to search images")),
            Err(error) => Err(error.to_string()),
        }
    }

    pub fn run_interactive<I, S>(&self, args: I) -> Result<(), String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.runner
            .run_interactive(args)
            .map_err(|error| error.to_string())
    }

    pub fn remove_volume(&self, volume_name: &str) -> Result<String, String> {
        self.run_simple_action(["volume", "rm", volume_name], "volume removed")
    }

    pub fn inspect_volume(&self, volume_name: &str) -> Result<String, String> {
        self.runner
            .run(["volume", "inspect", volume_name])
            .map(|output| output.stdout)
            .map_err(|error| error.to_string())
    }

    pub fn remove_network(&self, network_id: &str) -> Result<String, String> {
        self.run_simple_action(["network", "rm", network_id], "network removed")
    }

    pub fn inspect_network(&self, network_id: &str) -> Result<String, String> {
        self.runner
            .run(["network", "inspect", network_id])
            .map(|output| output.stdout)
            .map_err(|error| error.to_string())
    }

    pub fn create_container(&self, request: &ContainerCreateRequest) -> Result<String, String> {
        let mut args = vec!["run".to_string(), "-d".to_string()];

        if let Some(name) = &request.name {
            args.push("--name".to_string());
            args.push(name.clone());
        }

        for port in &request.ports {
            args.push("-p".to_string());
            args.push(port.clone());
        }

        for volume in &request.volumes {
            args.push("-v".to_string());
            args.push(volume.clone());
        }

        for env in &request.env {
            args.push("-e".to_string());
            args.push(env.clone());
        }

        args.push(request.image.clone());

        self.run_simple_action(args, "container created")
    }

    fn inspect_environment(&self) -> DockerEnvironmentStatus {
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

    fn inspect_resources(&self, environment: &DockerEnvironmentStatus) -> DockerResourceOverview {
        if environment.installation.level == StatusLevel::Error {
            return DockerResourceOverview {
                containers: ResourceQuery::skipped(
                    "Containers",
                    "skipped because docker command is not available",
                ),
                images: ResourceQuery::skipped(
                    "Images",
                    "skipped because docker command is not available",
                ),
                volumes: ResourceQuery::skipped(
                    "Volumes",
                    "skipped because docker command is not available",
                ),
                networks: ResourceQuery::skipped(
                    "Networks",
                    "skipped because docker command is not available",
                ),
            };
        }

        if environment.daemon.level == StatusLevel::Error
            || environment.daemon.level == StatusLevel::Warning
        {
            return DockerResourceOverview {
                containers: ResourceQuery::skipped(
                    "Containers",
                    "skipped because docker daemon is not reachable",
                ),
                images: ResourceQuery::skipped(
                    "Images",
                    "skipped because docker daemon is not reachable",
                ),
                volumes: ResourceQuery::skipped(
                    "Volumes",
                    "skipped because docker daemon is not reachable",
                ),
                networks: ResourceQuery::skipped(
                    "Networks",
                    "skipped because docker daemon is not reachable",
                ),
            };
        }

        DockerResourceOverview {
            containers: self.inspect_query(
                "Containers",
                ["ps", "-a", "--format", "{{json .}}"],
                format!("loaded with {}", self.output_format_description()),
            ),
            images: self.inspect_query(
                "Images",
                ["images", "--format", "{{json .}}"],
                format!("loaded with {}", self.output_format_description()),
            ),
            volumes: self.inspect_query(
                "Volumes",
                ["volume", "ls", "--format", "{{json .}}"],
                format!("loaded with {}", self.output_format_description()),
            ),
            networks: self.inspect_query(
                "Networks",
                ["network", "ls", "--format", "{{json .}}"],
                format!("loaded with {}", self.output_format_description()),
            ),
        }
    }

    fn inspect_daemon(&self) -> StatusLine {
        match self
            .runner
            .run_captured(["info", "--format", "{{json .ServerVersion}}"])
        {
            Ok(output) if output.status_code == Some(0) => {
                let server_version = self
                    .parse_json_string(&output)
                    .unwrap_or_else(|_| "available".to_string());
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
            Ok(output) if output.status_code == Some(0) => {
                match serde_json::from_str::<DockerVersionInfo>(&output.stdout) {
                    Ok(version) => {
                        let client = version
                            .client
                            .version
                            .unwrap_or_else(|| "unknown".to_string());
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
                }
            }
            Ok(output) => StatusLine::warning(
                "Docker version",
                format_command_failure(&output, "failed to query docker version"),
            ),
            Err(error) => StatusLine::error("Docker version", error.to_string()),
        }
    }

    fn inspect_query<T, I, S>(
        &self,
        label: &'static str,
        args: I,
        success_detail: String,
    ) -> ResourceQuery<T>
    where
        T: for<'de> Deserialize<'de>,
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        match self.runner.run_captured(args) {
            Ok(output) if output.status_code == Some(0) => match parse_json_lines::<T>(&output.stdout) {
                Ok(items) => {
                    let detail = if items.is_empty() {
                        format!("0 items; {success_detail}")
                    } else {
                        format!("{} items; {success_detail}", items.len())
                    };

                    ResourceQuery {
                        label,
                        items,
                        status: StatusLine::ok(label, detail),
                    }
                }
                Err(error) => ResourceQuery {
                    label,
                    items: Vec::new(),
                    status: StatusLine::warning(
                        label,
                        format!("query succeeded but JSON parsing failed: {error}"),
                    ),
                },
            },
            Ok(output) => ResourceQuery {
                label,
                items: Vec::new(),
                status: StatusLine::warning(
                    label,
                    format_command_failure(&output, &format!("failed to load {label}")),
                ),
            },
            Err(error) => ResourceQuery {
                label,
                items: Vec::new(),
                status: StatusLine::error(label, format!("failed to load {label}: {error}")),
            },
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

    fn output_format_description(&self) -> &'static str {
        match self.output_format {
            DockerOutputFormat::Json => "JSON lines via `--format {{json .}}`",
        }
    }

    fn parse_json_string(&self, output: &CommandOutput) -> Result<String, serde_json::Error> {
        serde_json::from_str::<String>(&output.stdout)
    }

    fn run_container_action<I, S>(&self, args: I, success_fallback: &str) -> Result<String, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.run_simple_action(args, success_fallback)
    }

    fn run_simple_action<I, S>(&self, args: I, success_fallback: &str) -> Result<String, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.runner
            .run(args)
            .map(|output| non_empty_or_fallback(output.stdout, success_fallback))
            .map_err(|error| error.to_string())
    }
}

impl<T> ResourceQuery<T> {
    pub fn skipped(label: &'static str, detail: impl Into<String>) -> Self {
        Self {
            label,
            items: Vec::new(),
            status: StatusLine::warning(label, detail),
        }
    }
}

impl ResourceQuery<ContainerSummary> {
    pub fn preview(&self) -> String {
        self.items
            .first()
            .map(ContainerSummary::preview)
            .unwrap_or_else(|| "no container rows loaded".to_string())
    }
}

impl ResourceQuery<ImageSummary> {
    pub fn preview(&self) -> String {
        self.items
            .first()
            .map(ImageSummary::preview)
            .unwrap_or_else(|| "no image rows loaded".to_string())
    }
}

impl ResourceQuery<VolumeSummary> {
    pub fn preview(&self) -> String {
        self.items
            .first()
            .map(VolumeSummary::preview)
            .unwrap_or_else(|| "no volume rows loaded".to_string())
    }
}

impl ResourceQuery<NetworkSummary> {
    pub fn preview(&self) -> String {
        self.items
            .first()
            .map(NetworkSummary::preview)
            .unwrap_or_else(|| "no network rows loaded".to_string())
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

impl ContainerSummary {
    pub fn preview(&self) -> String {
        let ports = if self.ports.is_empty() {
            "no published ports"
        } else {
            &self.ports
        };

        format!(
            "{} ({}) image={} state={} status={} ports={}",
            self.names, self.id, self.image, self.state, self.status, ports
        )
    }
}

impl ImageSummary {
    pub fn preview(&self) -> String {
        format!(
            "{}:{} ({}) size={} created={}",
            self.repository, self.tag, self.id, self.size, self.created_since
        )
    }
}

impl ImageSearchResult {
    pub fn preview(&self) -> String {
        let official = if self.official == "true" {
            " [Official]"
        } else {
            ""
        };
        format!(
            "{} stars={}{}\n\n{}",
            self.name, self.stars, official, self.description
        )
    }
}

impl VolumeSummary {
    pub fn preview(&self) -> String {
        format!(
            "{} driver={} scope={} mountpoint={}",
            self.name, self.driver, self.scope, self.mountpoint
        )
    }
}

impl NetworkSummary {
    pub fn preview(&self) -> String {
        format!(
            "{} ({}) driver={} scope={}",
            self.name, self.id, self.driver, self.scope
        )
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

fn parse_json_lines<T>(stdout: &str) -> Result<Vec<T>, serde_json::Error>
where
    T: for<'de> Deserialize<'de>,
{
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str::<T>)
        .collect()
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

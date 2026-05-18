use crossterm::event::KeyCode;

use crate::config::AppConfig;
use crate::docker::{ContainerSummary, DockerOverview, DockerService, ResourceQuery, StatusLevel};

#[derive(Debug)]
pub struct App {
    pub config: AppConfig,
    pub docker: DockerOverview,
    pub title: &'static str,
    pub should_quit: bool,
    pub screen: Screen,
    pub menu_index: usize,
    pub list_index: usize,
    pub confirm: Option<ConfirmState>,
    pub status_message: String,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub tick_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    ResourceList(ResourceKind),
    TextView(TextViewState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    Containers,
    Images,
    Volumes,
    Networks,
}

#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: &'static str,
    pub detail: &'static str,
}

#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub title: String,
    pub message: String,
    pub confirm_label: String,
    pub cancel_label: String,
    pub selected_confirm: bool,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmAction {
    Quit,
    StopContainer,
    RestartContainer,
    RemoveContainer,
}

#[derive(Debug, Clone)]
pub struct ResourceListState {
    pub title: String,
    pub detail: String,
    pub preview: String,
    pub status: StatusLevel,
    pub rows: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextViewState {
    pub source: ResourceKind,
    pub kind: TextViewKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextViewKind {
    Inspect,
    Logs,
    ShellHelp,
}

#[derive(Debug, Clone)]
pub struct TextViewContent {
    pub title: String,
    pub subtitle: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub enum AppCommand {
    OpenContainerShell {
        container_id: String,
        container_name: String,
    },
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let docker_service = DockerService::new(&config.docker);
        let docker = docker_service.inspect();

        Self {
            config,
            docker,
            title: "dockers",
            should_quit: false,
            screen: Screen::MainMenu,
            menu_index: 0,
            list_index: 0,
            confirm: None,
            status_message:
                "Ready. Use arrow keys to navigate, Enter to open, r to refresh.".to_string(),
            error_message: None,
            is_loading: false,
            tick_count: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<AppCommand> {
        if self.confirm.is_some() {
            return self.handle_confirm_key(key);
        }

        match key {
            KeyCode::Char('q') => {
                self.open_quit_confirm();
                None
            }
            KeyCode::Esc => {
                self.handle_escape();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_up();
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_down();
                None
            }
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => {
                self.activate_selection();
                None
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.go_back();
                None
            }
            KeyCode::Char('r') => {
                self.refresh();
                None
            }
            KeyCode::Char('?') => {
                self.status_message = "Keys: arrows/hjkl move, Enter open, r refresh, Esc back, q quit. Container actions: s start, t stop, R restart, d delete, g logs, i inspect, e shell.".to_string();
                None
            }
            _ => self.handle_context_key(key),
        }
    }

    pub fn handle_shell_result(&mut self, result: Result<(), String>, container_name: &str) {
        match result {
            Ok(()) => {
                self.status_message =
                    format!("Returned from interactive shell for container `{container_name}`.");
                self.error_message = None;
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message =
                    format!("Failed to open interactive shell for `{container_name}`.");
                self.screen = Screen::TextView(TextViewState {
                    source: ResourceKind::Containers,
                    kind: TextViewKind::ShellHelp,
                });
            }
        }
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Containers",
                detail: "Browse all containers and run container-specific actions",
            },
            MenuItem {
                label: "Images",
                detail: "Browse images available on the current Docker host",
            },
            MenuItem {
                label: "Volumes",
                detail: "Browse persistent volumes and mount metadata",
            },
            MenuItem {
                label: "Networks",
                detail: "Browse Docker networks and driver information",
            },
        ]
    }

    pub fn selected_menu_item(&self) -> ResourceKind {
        match self.menu_index {
            0 => ResourceKind::Containers,
            1 => ResourceKind::Images,
            2 => ResourceKind::Volumes,
            _ => ResourceKind::Networks,
        }
    }

    pub fn resource_list_state(&self, kind: ResourceKind) -> ResourceListState {
        match kind {
            ResourceKind::Containers => {
                let query = &self.docker.resources.containers;
                ResourceListState {
                    title: "Containers".to_string(),
                    detail: query.status.detail.clone(),
                    preview: query.preview(),
                    status: query.status.level,
                    rows: query
                        .items
                        .iter()
                        .map(|item| format!("{} | {} | {}", item.names, item.state, item.image))
                        .collect(),
                }
            }
            ResourceKind::Images => {
                let query = &self.docker.resources.images;
                ResourceListState {
                    title: "Images".to_string(),
                    detail: query.status.detail.clone(),
                    preview: query.preview(),
                    status: query.status.level,
                    rows: query
                        .items
                        .iter()
                        .map(|item| {
                            format!(
                                "{}:{} | {} | {}",
                                item.repository, item.tag, item.id, item.size
                            )
                        })
                        .collect(),
                }
            }
            ResourceKind::Volumes => {
                let query = &self.docker.resources.volumes;
                ResourceListState {
                    title: "Volumes".to_string(),
                    detail: query.status.detail.clone(),
                    preview: query.preview(),
                    status: query.status.level,
                    rows: query
                        .items
                        .iter()
                        .map(|item| format!("{} | {} | {}", item.name, item.driver, item.scope))
                        .collect(),
                }
            }
            ResourceKind::Networks => {
                let query = &self.docker.resources.networks;
                ResourceListState {
                    title: "Networks".to_string(),
                    detail: query.status.detail.clone(),
                    preview: query.preview(),
                    status: query.status.level,
                    rows: query
                        .items
                        .iter()
                        .map(|item| format!("{} | {} | {}", item.name, item.driver, item.scope))
                        .collect(),
                }
            }
        }
    }

    pub fn text_view_content(&self, state: TextViewState) -> TextViewContent {
        match state.kind {
            TextViewKind::Inspect => {
                if let Some(container) = self.selected_container() {
                    let service = DockerService::new(&self.config.docker);
                    match service.inspect_container(&container.id) {
                        Ok(output) => TextViewContent {
                            title: format!("Inspect: {}", container.names),
                            subtitle: "Raw `docker inspect` output".to_string(),
                            body: output,
                        },
                        Err(error) => TextViewContent {
                            title: format!("Inspect: {}", container.names),
                            subtitle: "Failed to load details".to_string(),
                            body: error,
                        },
                    }
                } else {
                    TextViewContent {
                        title: "Inspect".to_string(),
                        subtitle: "No container selected".to_string(),
                        body: "Select a container first.".to_string(),
                    }
                }
            }
            TextViewKind::Logs => {
                if let Some(container) = self.selected_container() {
                    let service = DockerService::new(&self.config.docker);
                    match service.container_logs(&container.id) {
                        Ok(output) => TextViewContent {
                            title: format!("Logs: {}", container.names),
                            subtitle: "Latest 200 log lines".to_string(),
                            body: output,
                        },
                        Err(error) => TextViewContent {
                            title: format!("Logs: {}", container.names),
                            subtitle: "Failed to load logs".to_string(),
                            body: error,
                        },
                    }
                } else {
                    TextViewContent {
                        title: "Logs".to_string(),
                        subtitle: "No container selected".to_string(),
                        body: "Select a container first.".to_string(),
                    }
                }
            }
            TextViewKind::ShellHelp => {
                let container_name = self
                    .selected_container()
                    .map(|container| container.names.clone())
                    .unwrap_or_else(|| "container".to_string());

                TextViewContent {
                    title: format!("Shell: {container_name}"),
                    subtitle: "Interactive shell guidance".to_string(),
                    body: "The shell command uses `docker exec -it <container> /bin/sh` and falls back to `/bin/bash`.\n\nIf it fails, check whether the container is running and whether the image includes a shell binary.\n\nPress `e` again from the container list to retry.".to_string(),
                }
            }
        }
    }

    pub fn selected_row_preview(&self) -> String {
        match self.screen {
            Screen::MainMenu => self.menu_items()[self.menu_index].detail.to_string(),
            Screen::ResourceList(kind) => {
                let rows = self.resource_list_state(kind).rows;
                rows.get(self.list_index)
                    .cloned()
                    .unwrap_or_else(|| "No rows available".to_string())
            }
            Screen::TextView(state) => self.text_view_content(state).subtitle,
        }
    }

    pub fn status_lines(&self) -> Vec<(String, StatusLevel)> {
        vec![
            (
                format!(
                    "{}: {}",
                    self.docker.environment.installation.label,
                    self.docker.environment.installation.detail
                ),
                self.docker.environment.installation.level,
            ),
            (
                format!(
                    "{}: {}",
                    self.docker.environment.daemon.label, self.docker.environment.daemon.detail
                ),
                self.docker.environment.daemon.level,
            ),
            (
                format!(
                    "{}: {}",
                    self.docker.environment.version.label, self.docker.environment.version.detail
                ),
                self.docker.environment.version.level,
            ),
            (
                format!(
                    "{}: {}",
                    self.docker.environment.output_strategy.label,
                    self.docker.environment.output_strategy.detail
                ),
                self.docker.environment.output_strategy.level,
            ),
        ]
    }

    pub fn help_text(&self) -> &'static str {
        match self.screen {
            Screen::MainMenu => "Up/Down move  Enter open  r refresh  q quit  ? help",
            Screen::ResourceList(ResourceKind::Containers) => {
                "Up/Down move  Enter inspect  s start  t stop  R restart  d delete  g logs  i inspect  e shell  Esc back"
            }
            Screen::ResourceList(_) => {
                "Up/Down move  Enter inspect row  Esc back  r refresh  q quit"
            }
            Screen::TextView(_) => "Esc back  q quit  r refresh",
        }
    }

    pub fn progress_text(&self) -> String {
        let spinner = ["|", "/", "-", "\\"];
        let glyph = spinner[self.tick_count % spinner.len()];

        if self.is_loading {
            format!("{glyph} loading docker data...")
        } else {
            format!("{glyph} idle")
        }
    }

    fn handle_context_key(&mut self, key: KeyCode) -> Option<AppCommand> {
        match self.screen {
            Screen::ResourceList(ResourceKind::Containers) => match key {
                KeyCode::Char('s') => {
                    self.start_selected_container();
                    None
                }
                KeyCode::Char('t') => {
                    self.open_container_confirm(ConfirmAction::StopContainer);
                    None
                }
                KeyCode::Char('R') => {
                    self.open_container_confirm(ConfirmAction::RestartContainer);
                    None
                }
                KeyCode::Char('d') => {
                    self.open_container_confirm(ConfirmAction::RemoveContainer);
                    None
                }
                KeyCode::Char('g') => {
                    self.open_text_view(TextViewKind::Logs);
                    None
                }
                KeyCode::Char('i') | KeyCode::Enter => {
                    self.open_text_view(TextViewKind::Inspect);
                    None
                }
                KeyCode::Char('e') => self.open_shell(),
                _ => None,
            },
            Screen::TextView(_) => None,
            _ => None,
        }
    }

    fn handle_confirm_key(&mut self, key: KeyCode) -> Option<AppCommand> {
        let Some(confirm) = self.confirm.as_mut() else {
            return None;
        };

        match key {
            KeyCode::Left | KeyCode::Char('h') => confirm.selected_confirm = false,
            KeyCode::Right | KeyCode::Char('l') => confirm.selected_confirm = true,
            KeyCode::Tab => confirm.selected_confirm = !confirm.selected_confirm,
            KeyCode::Esc => {
                self.confirm = None;
                self.status_message = "Cancelled.".to_string();
            }
            KeyCode::Enter => {
                let accepted = confirm.selected_confirm;
                let action = confirm.action;
                self.confirm = None;

                if accepted {
                    return self.execute_confirm_action(action);
                }

                self.status_message = "Cancelled.".to_string();
            }
            _ => {}
        }

        None
    }

    fn handle_escape(&mut self) {
        match self.screen {
            Screen::MainMenu => self.open_quit_confirm(),
            Screen::ResourceList(_) | Screen::TextView(_) => self.go_back(),
        }
    }

    fn move_up(&mut self) {
        match self.screen {
            Screen::MainMenu => {
                if self.menu_index > 0 {
                    self.menu_index -= 1;
                }
            }
            Screen::ResourceList(_) => {
                if self.list_index > 0 {
                    self.list_index -= 1;
                }
            }
            Screen::TextView(_) => {}
        }
    }

    fn move_down(&mut self) {
        match self.screen {
            Screen::MainMenu => {
                let max = self.menu_items().len().saturating_sub(1);
                if self.menu_index < max {
                    self.menu_index += 1;
                }
            }
            Screen::ResourceList(kind) => {
                let max = self.resource_list_state(kind).rows.len().saturating_sub(1);
                if self.list_index < max {
                    self.list_index += 1;
                }
            }
            Screen::TextView(_) => {}
        }
    }

    fn activate_selection(&mut self) {
        match self.screen {
            Screen::MainMenu => {
                self.screen = Screen::ResourceList(self.selected_menu_item());
                self.list_index = 0;
                self.error_message = None;
                self.status_message =
                    "Opened resource list. Use Up/Down to move and Esc to return.".to_string();
            }
            Screen::ResourceList(ResourceKind::Containers) => {
                self.open_text_view(TextViewKind::Inspect);
            }
            Screen::ResourceList(_) => {
                self.status_message =
                    "Row selection is active. Resource-specific detail screens come next."
                        .to_string();
            }
            Screen::TextView(_) => {}
        }
    }

    fn go_back(&mut self) {
        match self.screen {
            Screen::ResourceList(_) => {
                self.screen = Screen::MainMenu;
                self.list_index = 0;
                self.status_message = "Returned to main menu.".to_string();
            }
            Screen::TextView(state) => {
                self.screen = Screen::ResourceList(state.source);
                self.status_message = "Returned to container list.".to_string();
            }
            Screen::MainMenu => {}
        }
    }

    fn refresh(&mut self) {
        self.is_loading = true;
        self.status_message = "Refreshing docker data...".to_string();
        self.error_message = None;

        let docker_service = DockerService::new(&self.config.docker);
        self.docker = docker_service.inspect();
        self.is_loading = false;

        if self.has_errors() {
            self.error_message = Some(
                "Some Docker checks or resource queries failed. Review the status panel."
                    .to_string(),
            );
            self.status_message = "Refresh completed with warnings.".to_string();
        } else {
            self.status_message = "Refresh completed successfully.".to_string();
        }

        self.clamp_selection();
    }

    fn has_errors(&self) -> bool {
        self.status_lines()
            .iter()
            .any(|(_, level)| matches!(level, StatusLevel::Error | StatusLevel::Warning))
            || self.any_resource_status(|query| {
                matches!(query.status, StatusLevel::Error | StatusLevel::Warning)
            })
    }

    fn any_resource_status<F>(&self, predicate: F) -> bool
    where
        F: Fn(&GenericResourceStatus) -> bool,
    {
        let statuses = [
            GenericResourceStatus::from(&self.docker.resources.containers),
            GenericResourceStatus::from(&self.docker.resources.images),
            GenericResourceStatus::from(&self.docker.resources.volumes),
            GenericResourceStatus::from(&self.docker.resources.networks),
        ];

        statuses.iter().any(predicate)
    }

    fn clamp_selection(&mut self) {
        if let Screen::ResourceList(kind) = self.screen {
            let len = self.resource_list_state(kind).rows.len();
            if len == 0 {
                self.list_index = 0;
            } else if self.list_index >= len {
                self.list_index = len - 1;
            }
        }
    }

    fn open_quit_confirm(&mut self) {
        self.confirm = Some(ConfirmState {
            title: "Quit dockers?".to_string(),
            message: "Close the application and leave the current terminal view?".to_string(),
            confirm_label: "Quit".to_string(),
            cancel_label: "Stay".to_string(),
            selected_confirm: false,
            action: ConfirmAction::Quit,
        });
    }

    fn open_container_confirm(&mut self, action: ConfirmAction) {
        let Some(container) = self.selected_container() else {
            self.error_message = Some("No container selected.".to_string());
            return;
        };

        let (title, message, confirm_label) = match action {
            ConfirmAction::StopContainer => (
                "Stop container?",
                format!(
                    "Stop container `{}` ({})? Running processes inside the container will be interrupted.",
                    container.names, container.id
                ),
                "Stop".to_string(),
            ),
            ConfirmAction::RestartContainer => (
                "Restart container?",
                format!(
                    "Restart container `{}` ({})? Connected sessions may be interrupted.",
                    container.names, container.id
                ),
                "Restart".to_string(),
            ),
            ConfirmAction::RemoveContainer => (
                "Delete container?",
                format!(
                    "Remove container `{}` ({}) with `docker rm -f`? This is destructive.",
                    container.names, container.id
                ),
                "Delete".to_string(),
            ),
            ConfirmAction::Quit => ("Quit dockers?", "Quit?".to_string(), "Quit".to_string()),
        };

        self.confirm = Some(ConfirmState {
            title: title.to_string(),
            message,
            confirm_label,
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
            action,
        });
    }

    fn open_text_view(&mut self, kind: TextViewKind) {
        self.screen = Screen::TextView(TextViewState {
            source: ResourceKind::Containers,
            kind,
        });
        self.status_message = match kind {
            TextViewKind::Inspect => "Opened container details.".to_string(),
            TextViewKind::Logs => "Opened container logs.".to_string(),
            TextViewKind::ShellHelp => "Opened shell guidance.".to_string(),
        };
        self.error_message = None;
    }

    fn open_shell(&mut self) -> Option<AppCommand> {
        let Some((container_id, container_name)) = self
            .selected_container()
            .map(|container| (container.id.clone(), container.names.clone()))
        else {
            self.error_message = Some("No container selected.".to_string());
            return None;
        };

        self.status_message = format!(
            "Opening interactive shell for `{}`. Exit the shell to return to dockers.",
            container_name
        );
        self.error_message = None;

        Some(AppCommand::OpenContainerShell {
            container_id,
            container_name,
        })
    }

    fn execute_confirm_action(&mut self, action: ConfirmAction) -> Option<AppCommand> {
        match action {
            ConfirmAction::Quit => {
                self.quit();
                None
            }
            ConfirmAction::StopContainer => {
                self.run_selected_container_action(|service, id| service.stop_container(id), "Container stopped.")
            }
            ConfirmAction::RestartContainer => self.run_selected_container_action(
                |service, id| service.restart_container(id),
                "Container restarted.",
            ),
            ConfirmAction::RemoveContainer => self.run_selected_container_action(
                |service, id| service.remove_container(id),
                "Container removed.",
            ),
        }
    }

    fn start_selected_container(&mut self) {
        let _ = self.run_selected_container_action(
            |service, id| service.start_container(id),
            "Container started.",
        );
    }

    fn run_selected_container_action<F>(
        &mut self,
        action: F,
        success_prefix: &str,
    ) -> Option<AppCommand>
    where
        F: Fn(&DockerService, &str) -> Result<String, String>,
    {
        let Some((container_id, container_name)) = self
            .selected_container()
            .map(|container| (container.id.clone(), container.names.clone()))
        else {
            self.error_message = Some("No container selected.".to_string());
            return None;
        };

        let service = DockerService::new(&self.config.docker);
        match action(&service, &container_id) {
            Ok(output) => {
                self.status_message = format!("{success_prefix} {}", output.trim());
                self.error_message = None;
                self.screen = Screen::ResourceList(ResourceKind::Containers);
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message = format!("Action failed for container `{}`.", container_name);
            }
        }

        None
    }

    fn selected_container(&self) -> Option<&ContainerSummary> {
        self.docker.resources.containers.items.get(self.list_index)
    }
}

struct GenericResourceStatus {
    status: StatusLevel,
}

impl<T> From<&ResourceQuery<T>> for GenericResourceStatus {
    fn from(value: &ResourceQuery<T>) -> Self {
        Self {
            status: value.status.level,
        }
    }
}

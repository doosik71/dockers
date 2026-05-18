use crossterm::event::KeyCode;

use crate::config::AppConfig;
use crate::docker::{DockerOverview, DockerService, ResourceQuery, StatusLevel};

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
}

#[derive(Debug, Clone)]
pub struct ResourceListState {
    pub title: String,
    pub detail: String,
    pub preview: String,
    pub status: StatusLevel,
    pub rows: Vec<String>,
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
            status_message: "Ready. Use arrow keys to navigate, Enter to open, r to refresh."
                .to_string(),
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

    pub fn handle_key(&mut self, key: KeyCode) {
        if self.confirm.is_some() {
            self.handle_confirm_key(key);
            return;
        }

        match key {
            KeyCode::Char('q') => self.open_quit_confirm(),
            KeyCode::Esc => self.handle_escape(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Enter | KeyCode::Right | KeyCode::Char('l') => self.activate_selection(),
            KeyCode::Left | KeyCode::Char('h') => self.go_back(),
            KeyCode::Char('r') => self.refresh(),
            KeyCode::Char('?') => {
                self.status_message =
                    "Keys: arrows/hjkl move, Enter open, r refresh, Esc back, q quit".to_string();
            }
            KeyCode::Char('d') => self.open_placeholder_confirm(),
            _ => {}
        }
    }

    pub fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem {
                label: "Containers",
                detail: "Browse all containers and inspect their current status",
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
                        .map(|item| format!("{}:{} | {} | {}", item.repository, item.tag, item.id, item.size))
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

    pub fn selected_row_preview(&self) -> String {
        match self.screen {
            Screen::MainMenu => self.menu_items()[self.menu_index].detail.to_string(),
            Screen::ResourceList(kind) => {
                let rows = self.resource_list_state(kind).rows;
                rows.get(self.list_index)
                    .cloned()
                    .unwrap_or_else(|| "No rows available".to_string())
            }
        }
    }

    pub fn status_lines(&self) -> Vec<(String, StatusLevel)> {
        vec![
            (
                format!(
                    "{}: {}",
                    self.docker.environment.installation.label, self.docker.environment.installation.detail
                ),
                self.docker.environment.installation.level,
            ),
            (
                format!("{}: {}", self.docker.environment.daemon.label, self.docker.environment.daemon.detail),
                self.docker.environment.daemon.level,
            ),
            (
                format!("{}: {}", self.docker.environment.version.label, self.docker.environment.version.detail),
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
            Screen::ResourceList(_) => "Up/Down move  Enter inspect row  d confirm demo  Esc back  r refresh  q quit",
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

    fn handle_confirm_key(&mut self, key: KeyCode) {
        let Some(confirm) = self.confirm.as_mut() else {
            return;
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
                let title = confirm.title.clone();
                self.confirm = None;

                if accepted && title == "Quit dockers?" {
                    self.quit();
                } else if accepted {
                    self.status_message =
                        "Confirmed. Action flow is wired, but the command is not implemented yet."
                            .to_string();
                } else {
                    self.status_message = "Cancelled.".to_string();
                }
            }
            _ => {}
        }
    }

    fn handle_escape(&mut self) {
        match self.screen {
            Screen::MainMenu => self.open_quit_confirm(),
            Screen::ResourceList(_) => self.go_back(),
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
                let max = self
                    .resource_list_state(kind)
                    .rows
                    .len()
                    .saturating_sub(1);
                if self.list_index < max {
                    self.list_index += 1;
                }
            }
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
            Screen::ResourceList(_) => {
                self.status_message =
                    "Row selection is active. Detail inspection is the next step.".to_string();
            }
        }
    }

    fn go_back(&mut self) {
        if let Screen::ResourceList(_) = self.screen {
            self.screen = Screen::MainMenu;
            self.list_index = 0;
            self.status_message = "Returned to main menu.".to_string();
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
            self.error_message =
                Some("Some Docker checks or resource queries failed. Review the status panel.".to_string());
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
            || self.any_resource_status(|query| matches!(query.status, StatusLevel::Error | StatusLevel::Warning))
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
        });
    }

    fn open_placeholder_confirm(&mut self) {
        self.confirm = Some(ConfirmState {
            title: "Confirm action".to_string(),
            message:
                "This is the shared confirmation dialog that later destructive actions will reuse."
                    .to_string(),
            confirm_label: "Confirm".to_string(),
            cancel_label: "Cancel".to_string(),
            selected_confirm: false,
        });
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

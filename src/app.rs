use std::collections::BTreeSet;

use crossterm::event::KeyCode;

use crate::config::AppConfig;
use crate::docker::{
    ContainerCreateRequest, ContainerSummary, DockerOverview, DockerService, ImageSummary,
    NetworkSummary, ResourceQuery, StatusLevel, VolumeSummary,
};

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
    pub search_mode: bool,
    pub search_query: String,
    pub filter: ResourceFilter,
    pub sort: SortMode,
    pub selected_keys: BTreeSet<String>,
    pub recent_actions: Vec<String>,
    pub wizard: CreateWizardState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    MainMenu,
    ResourceList(ResourceKind),
    TextView(TextViewState),
    CreateWizard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    RemoveImage,
    RemoveVolume,
    RemoveNetwork,
}

#[derive(Debug, Clone)]
pub struct ResourceListState {
    pub title: String,
    pub detail: String,
    pub preview: String,
    pub status: StatusLevel,
    pub rows: Vec<ResourceRow>,
    pub total_count: usize,
    pub visible_count: usize,
    pub selected_count: usize,
    pub search_query: String,
    pub filter: ResourceFilter,
    pub sort: SortMode,
    pub action_hint: &'static str,
}

#[derive(Debug, Clone)]
pub struct ResourceRow {
    pub key: String,
    pub line: String,
    pub preview: String,
    pub searchable: String,
    pub status_bucket: ItemStatus,
    pub sort_name: String,
    pub sort_rank: usize,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Active,
    Inactive,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceFilter {
    All,
    Active,
    Inactive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    NameAsc,
    NameDesc,
    Status,
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

#[derive(Debug, Clone)]
pub struct CreateWizardState {
    pub step: CreateWizardStep,
    pub image_mode: ImageInputMode,
    pub image_index: usize,
    pub image_value: String,
    pub name_value: String,
    pub ports_value: String,
    pub volumes_value: String,
    pub env_value: String,
    pub input_buffer: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateWizardStep {
    Image,
    Name,
    Ports,
    Volumes,
    Env,
    Summary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageInputMode {
    Select,
    Manual,
}

#[derive(Debug, Clone)]
pub struct CreateWizardView {
    pub step: CreateWizardStep,
    pub title: String,
    pub prompt: String,
    pub guidance: String,
    pub input_value: String,
    pub image_mode: ImageInputMode,
    pub image_rows: Vec<String>,
    pub image_index: usize,
    pub summary_lines: Vec<String>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let docker_service = DockerService::new(&config.docker);
        let docker = docker_service.inspect();
        let wizard = CreateWizardState::new(&docker);

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
                "Ready. Use arrow keys to navigate, / search, f filter, o sort, Space select."
                    .to_string(),
            error_message: None,
            is_loading: false,
            tick_count: 0,
            search_mode: false,
            search_query: String::new(),
            filter: ResourceFilter::All,
            sort: SortMode::NameAsc,
            selected_keys: BTreeSet::new(),
            recent_actions: vec!["Opened dockers.".to_string()],
            wizard,
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

        if self.search_mode {
            self.handle_search_key(key);
            return None;
        }

        if self.screen == Screen::CreateWizard {
            self.handle_wizard_key(key);
            return None;
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
            KeyCode::Char('/') => {
                self.search_mode = true;
                self.status_message =
                    "Search mode: type to filter rows, Enter to apply, Esc to cancel."
                        .to_string();
                None
            }
            KeyCode::Char('f') => {
                self.cycle_filter();
                None
            }
            KeyCode::Char('o') => {
                self.cycle_sort();
                None
            }
            KeyCode::Char(' ') => {
                self.toggle_selected_row();
                None
            }
            KeyCode::Char('a') => {
                self.toggle_select_all_visible();
                None
            }
            KeyCode::Char('n') => {
                self.open_create_wizard();
                None
            }
            KeyCode::Char('?') => {
                self.status_message = "Keys: / search, f filter, o sort, Space select, a select all, n new container, r refresh. Containers add s/t/R/d/g/i/e actions.".to_string();
                None
            }
            _ => self.handle_context_key(key),
        }
    }

    pub fn handle_shell_result(&mut self, result: Result<(), String>, container_name: &str) {
        match result {
            Ok(()) => {
                self.push_recent_action(format!(
                    "Returned from shell for container `{container_name}`."
                ));
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

    pub fn wizard_view(&self) -> CreateWizardView {
        self.wizard.view(&self.docker)
    }

    pub fn resource_list_state(&self, kind: ResourceKind) -> ResourceListState {
        let (title, detail, preview, status, base_rows, action_hint) = match kind {
            ResourceKind::Containers => {
                let query = &self.docker.resources.containers;
                (
                    "Containers".to_string(),
                    query.status.detail.clone(),
                    query.preview(),
                    query.status.level,
                    self.build_container_rows(query),
                    "n wizard, s start, t stop, R restart, d delete, g logs, i inspect, e shell",
                )
            }
            ResourceKind::Images => {
                let query = &self.docker.resources.images;
                (
                    "Images".to_string(),
                    query.status.detail.clone(),
                    query.preview(),
                    query.status.level,
                    self.build_image_rows(query),
                    "i inspect, d delete",
                )
            }
            ResourceKind::Volumes => {
                let query = &self.docker.resources.volumes;
                (
                    "Volumes".to_string(),
                    query.status.detail.clone(),
                    query.preview(),
                    query.status.level,
                    self.build_volume_rows(query),
                    "i inspect, d delete",
                )
            }
            ResourceKind::Networks => {
                let query = &self.docker.resources.networks;
                (
                    "Networks".to_string(),
                    query.status.detail.clone(),
                    query.preview(),
                    query.status.level,
                    self.build_network_rows(query),
                    "i inspect, d delete",
                )
            }
        };

        let total_count = base_rows.len();
        let mut rows = self.apply_filter_and_sort(base_rows);
        for row in &mut rows {
            row.selected = self.selected_keys.contains(&row.key);
        }
        let visible_count = rows.len();
        let selected_count = rows.iter().filter(|row| row.selected).count();

        ResourceListState {
            title,
            detail,
            preview,
            status,
            rows,
            total_count,
            visible_count,
            selected_count,
            search_query: self.search_query.clone(),
            filter: self.filter,
            sort: self.sort,
            action_hint,
        }
    }

    pub fn text_view_content(&self, state: TextViewState) -> TextViewContent {
        match (state.source, state.kind) {
            (ResourceKind::Containers, TextViewKind::Inspect) => {
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
            (ResourceKind::Images, TextViewKind::Inspect) => {
                if let Some(image) = self.selected_image() {
                    let service = DockerService::new(&self.config.docker);
                    match service.inspect_image(&image.id) {
                        Ok(output) => TextViewContent {
                            title: format!("Inspect: {}:{}", image.repository, image.tag),
                            subtitle: "Raw `docker image inspect` output".to_string(),
                            body: output,
                        },
                        Err(error) => TextViewContent {
                            title: format!("Inspect: {}:{}", image.repository, image.tag),
                            subtitle: "Failed to load details".to_string(),
                            body: error,
                        },
                    }
                } else {
                    TextViewContent {
                        title: "Inspect".to_string(),
                        subtitle: "No image selected".to_string(),
                        body: "Select an image first.".to_string(),
                    }
                }
            }
            (ResourceKind::Volumes, TextViewKind::Inspect) => {
                if let Some(volume) = self.selected_volume() {
                    let service = DockerService::new(&self.config.docker);
                    match service.inspect_volume(&volume.name) {
                        Ok(output) => TextViewContent {
                            title: format!("Inspect: {}", volume.name),
                            subtitle: "Raw `docker volume inspect` output".to_string(),
                            body: output,
                        },
                        Err(error) => TextViewContent {
                            title: format!("Inspect: {}", volume.name),
                            subtitle: "Failed to load details".to_string(),
                            body: error,
                        },
                    }
                } else {
                    TextViewContent {
                        title: "Inspect".to_string(),
                        subtitle: "No volume selected".to_string(),
                        body: "Select a volume first.".to_string(),
                    }
                }
            }
            (ResourceKind::Networks, TextViewKind::Inspect) => {
                if let Some(network) = self.selected_network() {
                    let service = DockerService::new(&self.config.docker);
                    match service.inspect_network(&network.id) {
                        Ok(output) => TextViewContent {
                            title: format!("Inspect: {}", network.name),
                            subtitle: "Raw `docker network inspect` output".to_string(),
                            body: output,
                        },
                        Err(error) => TextViewContent {
                            title: format!("Inspect: {}", network.name),
                            subtitle: "Failed to load details".to_string(),
                            body: error,
                        },
                    }
                } else {
                    TextViewContent {
                        title: "Inspect".to_string(),
                        subtitle: "No network selected".to_string(),
                        body: "Select a network first.".to_string(),
                    }
                }
            }
            (ResourceKind::Containers, TextViewKind::Logs) => {
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
            (_, TextViewKind::Logs) => TextViewContent {
                title: "Logs".to_string(),
                subtitle: "Not supported for this resource".to_string(),
                body: "Logs are currently available only for containers.".to_string(),
            },
            (_, TextViewKind::ShellHelp) => {
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
                let state = self.resource_list_state(kind);
                state
                    .rows
                    .get(self.list_index)
                    .map(|row| row.preview.clone())
                    .unwrap_or_else(|| "No rows available".to_string())
            }
            Screen::TextView(state) => self.text_view_content(state).subtitle,
            Screen::CreateWizard => self.wizard_view().guidance,
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
            Screen::MainMenu => "Up/Down move  Enter open  n new container  q quit  ? help",
            Screen::ResourceList(ResourceKind::Containers) => {
                "/ search  f filter  o sort  Space select  a select all  n wizard  s/t/R/d/g/i/e actions"
            }
            Screen::ResourceList(_) => {
                "/ search  f filter  o sort  Space select  a select all  i inspect  d delete"
            }
            Screen::TextView(_) => "Esc back  q quit  r refresh",
            Screen::CreateWizard => {
                "Wizard: type input  Enter next  Tab image mode  Up/Down image select  Backspace edit  Esc cancel"
            }
        }
    }

    pub fn progress_text(&self) -> String {
        let spinner = ["|", "/", "-", "\\"];
        let glyph = spinner[self.tick_count % spinner.len()];

        if self.is_loading {
            format!("{glyph} loading docker data...")
        } else if self.search_mode {
            format!("{glyph} search: {}", self.search_query)
        } else if self.screen == Screen::CreateWizard {
            format!("{glyph} wizard step: {}", self.wizard.step.label())
        } else {
            format!("{glyph} idle")
        }
    }

    pub fn recent_actions_summary(&self) -> String {
        if self.recent_actions.is_empty() {
            "No recent actions yet.".to_string()
        } else {
            self.recent_actions
                .iter()
                .rev()
                .take(4)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
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
                KeyCode::Char('n') => {
                    self.open_create_wizard();
                    None
                }
                _ => None,
            },
            Screen::ResourceList(ResourceKind::Images) => match key {
                KeyCode::Char('d') => {
                    self.open_resource_confirm(ConfirmAction::RemoveImage);
                    None
                }
                KeyCode::Char('i') | KeyCode::Enter => {
                    self.open_text_view_for(ResourceKind::Images, TextViewKind::Inspect);
                    None
                }
                _ => None,
            },
            Screen::ResourceList(ResourceKind::Volumes) => match key {
                KeyCode::Char('d') => {
                    self.open_resource_confirm(ConfirmAction::RemoveVolume);
                    None
                }
                KeyCode::Char('i') | KeyCode::Enter => {
                    self.open_text_view_for(ResourceKind::Volumes, TextViewKind::Inspect);
                    None
                }
                _ => None,
            },
            Screen::ResourceList(ResourceKind::Networks) => match key {
                KeyCode::Char('d') => {
                    self.open_resource_confirm(ConfirmAction::RemoveNetwork);
                    None
                }
                KeyCode::Char('i') | KeyCode::Enter => {
                    self.open_text_view_for(ResourceKind::Networks, TextViewKind::Inspect);
                    None
                }
                _ => None,
            },
            Screen::TextView(_) | Screen::CreateWizard => None,
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

    fn handle_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.search_mode = false;
                self.status_message = "Search mode cancelled.".to_string();
            }
            KeyCode::Enter => {
                self.search_mode = false;
                self.list_index = 0;
                self.status_message = if self.search_query.is_empty() {
                    "Search cleared.".to_string()
                } else {
                    format!("Search applied: `{}`.", self.search_query)
                };
            }
            KeyCode::Backspace => {
                self.search_query.pop();
                self.list_index = 0;
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
                self.list_index = 0;
            }
            _ => {}
        }
    }

    fn handle_wizard_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.screen = Screen::ResourceList(ResourceKind::Containers);
                self.status_message = "Container creation wizard cancelled.".to_string();
            }
            KeyCode::Tab if self.wizard.step == CreateWizardStep::Image => {
                self.wizard.toggle_image_mode();
                self.status_message = format!(
                    "Image mode set to {}.",
                    self.wizard.image_mode.label()
                );
            }
            KeyCode::Up if self.wizard.step == CreateWizardStep::Image => {
                self.wizard.move_image_selection(-1, &self.docker);
            }
            KeyCode::Down if self.wizard.step == CreateWizardStep::Image => {
                self.wizard.move_image_selection(1, &self.docker);
            }
            KeyCode::Backspace => {
                self.wizard.input_buffer.pop();
            }
            KeyCode::Enter => {
                self.advance_wizard();
            }
            KeyCode::Char(c) => {
                self.wizard.input_buffer.push(c);
            }
            _ => {}
        }
    }

    fn handle_escape(&mut self) {
        match self.screen {
            Screen::MainMenu => self.open_quit_confirm(),
            Screen::ResourceList(_) | Screen::TextView(_) => self.go_back(),
            Screen::CreateWizard => {
                self.screen = Screen::ResourceList(ResourceKind::Containers);
                self.status_message = "Container creation wizard cancelled.".to_string();
            }
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
            Screen::TextView(_) | Screen::CreateWizard => {}
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
            Screen::TextView(_) | Screen::CreateWizard => {}
        }
    }

    fn activate_selection(&mut self) {
        match self.screen {
            Screen::MainMenu => {
                self.screen = Screen::ResourceList(self.selected_menu_item());
                self.list_index = 0;
                self.error_message = None;
                self.status_message = "Opened resource list. Use / search, f filter, o sort, Space select.".to_string();
            }
            Screen::ResourceList(ResourceKind::Containers) => {
                self.open_text_view(TextViewKind::Inspect);
            }
            Screen::ResourceList(kind) => {
                self.open_text_view_for(kind, TextViewKind::Inspect);
            }
            Screen::TextView(_) | Screen::CreateWizard => {}
        }
    }

    fn go_back(&mut self) {
        match self.screen {
            Screen::ResourceList(_) => {
                self.screen = Screen::MainMenu;
                self.list_index = 0;
                self.search_mode = false;
                self.status_message = "Returned to main menu.".to_string();
            }
            Screen::TextView(state) => {
                self.screen = Screen::ResourceList(state.source);
                self.status_message = format!("Returned to {} list.", state.source.label());
            }
            Screen::CreateWizard => {
                self.screen = Screen::ResourceList(ResourceKind::Containers);
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
        self.wizard.sync_after_refresh(&self.docker);
        self.is_loading = false;

        if self.has_errors() {
            self.error_message = Some(
                "Some Docker checks or resource queries failed. Review the status panel."
                    .to_string(),
            );
            self.status_message = "Refresh completed with warnings.".to_string();
            self.push_recent_action("Refresh completed with warnings.".to_string());
        } else {
            self.status_message = "Refresh completed successfully.".to_string();
            self.push_recent_action("Refresh completed successfully.".to_string());
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
            _ => ("Quit dockers?", "Quit?".to_string(), "Quit".to_string()),
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
        self.open_text_view_for(ResourceKind::Containers, kind);
    }

    fn open_text_view_for(&mut self, source: ResourceKind, kind: TextViewKind) {
        self.screen = Screen::TextView(TextViewState { source, kind });
        self.status_message = match kind {
            TextViewKind::Inspect => format!("Opened {} details.", source.label()),
            TextViewKind::Logs => "Opened container logs.".to_string(),
            TextViewKind::ShellHelp => "Opened shell guidance.".to_string(),
        };
        self.push_recent_action(self.status_message.clone());
        self.error_message = None;
    }

    fn open_resource_confirm(&mut self, action: ConfirmAction) {
        let (title, message, confirm_label) = match action {
            ConfirmAction::RemoveImage => {
                let Some(image) = self.selected_image() else {
                    self.error_message = Some("No image selected.".to_string());
                    return;
                };
                (
                    "Delete image?",
                    format!(
                        "Remove image `{}:{}` ({})? This may fail if containers still reference it.",
                        image.repository, image.tag, image.id
                    ),
                    "Delete".to_string(),
                )
            }
            ConfirmAction::RemoveVolume => {
                let Some(volume) = self.selected_volume() else {
                    self.error_message = Some("No volume selected.".to_string());
                    return;
                };
                (
                    "Delete volume?",
                    format!(
                        "Remove volume `{}`? This is destructive if data is still needed.",
                        volume.name
                    ),
                    "Delete".to_string(),
                )
            }
            ConfirmAction::RemoveNetwork => {
                let Some(network) = self.selected_network() else {
                    self.error_message = Some("No network selected.".to_string());
                    return;
                };
                (
                    "Delete network?",
                    format!(
                        "Remove network `{}` ({})? Connected containers may be affected.",
                        network.name, network.id
                    ),
                    "Delete".to_string(),
                )
            }
            _ => return,
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
        self.push_recent_action(self.status_message.clone());
        self.error_message = None;

        Some(AppCommand::OpenContainerShell {
            container_id,
            container_name,
        })
    }

    fn open_create_wizard(&mut self) {
        self.wizard.reset_for_open(&self.docker);
        self.screen = Screen::CreateWizard;
        self.error_message = None;
        self.status_message =
            "Opened container creation wizard. Follow the steps and press Enter to continue."
                .to_string();
        self.push_recent_action("Opened container creation wizard.".to_string());
    }

    fn advance_wizard(&mut self) {
        match self.wizard.step {
            CreateWizardStep::Image => {
                if self.wizard.commit_image_step(&self.docker).is_err() {
                    self.error_message = Some("Choose or enter an image before continuing.".to_string());
                    return;
                }
                self.wizard.step = CreateWizardStep::Name;
                self.status_message = "Container name step.".to_string();
            }
            CreateWizardStep::Name => {
                self.wizard.name_value = self.wizard.input_buffer.trim().to_string();
                self.wizard.input_buffer = self.wizard.ports_value.clone();
                self.wizard.step = CreateWizardStep::Ports;
                self.status_message = "Port mapping step.".to_string();
            }
            CreateWizardStep::Ports => {
                self.wizard.ports_value = self.wizard.input_buffer.trim().to_string();
                self.wizard.input_buffer = self.wizard.volumes_value.clone();
                self.wizard.step = CreateWizardStep::Volumes;
                self.status_message = "Volume mount step.".to_string();
            }
            CreateWizardStep::Volumes => {
                self.wizard.volumes_value = self.wizard.input_buffer.trim().to_string();
                self.wizard.input_buffer = self.wizard.env_value.clone();
                self.wizard.step = CreateWizardStep::Env;
                self.status_message = "Environment variable step.".to_string();
            }
            CreateWizardStep::Env => {
                self.wizard.env_value = self.wizard.input_buffer.trim().to_string();
                self.wizard.input_buffer.clear();
                self.wizard.step = CreateWizardStep::Summary;
                self.status_message = "Review the summary and press Enter to create the container."
                    .to_string();
            }
            CreateWizardStep::Summary => {
                self.execute_create_wizard();
            }
        }
    }

    fn execute_create_wizard(&mut self) {
        let image = if self.wizard.image_value.trim().is_empty() {
            self.error_message = Some("Image is required.".to_string());
            return;
        } else {
            self.wizard.image_value.trim().to_string()
        };

        let request = ContainerCreateRequest {
            image,
            name: blank_to_none(&self.wizard.name_value),
            ports: parse_csv_list(&self.wizard.ports_value),
            volumes: parse_csv_list(&self.wizard.volumes_value),
            env: parse_csv_list(&self.wizard.env_value),
        };

        self.is_loading = true;
        let service = DockerService::new(&self.config.docker);
        let result = service.create_container(&request);
        self.is_loading = false;

        match result {
            Ok(output) => {
                self.status_message = format!("Container created. {}", output.trim());
                self.push_recent_action(self.status_message.clone());
                self.error_message = None;
                self.screen = Screen::ResourceList(ResourceKind::Containers);
                self.wizard = CreateWizardState::new(&self.docker);
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message = "Container creation failed. Review the error panel."
                    .to_string();
            }
        }
    }

    fn execute_confirm_action(&mut self, action: ConfirmAction) -> Option<AppCommand> {
        match action {
            ConfirmAction::Quit => {
                self.quit();
                None
            }
            ConfirmAction::StopContainer => self.run_selected_container_action(
                |service, id| service.stop_container(id),
                "Container stopped.",
            ),
            ConfirmAction::RestartContainer => self.run_selected_container_action(
                |service, id| service.restart_container(id),
                "Container restarted.",
            ),
            ConfirmAction::RemoveContainer => self.run_selected_container_action(
                |service, id| service.remove_container(id),
                "Container removed.",
            ),
            ConfirmAction::RemoveImage => self.run_selected_image_action(
                |service, id| service.remove_image(id),
                "Image removed.",
            ),
            ConfirmAction::RemoveVolume => self.run_selected_volume_action(
                |service, name| service.remove_volume(name),
                "Volume removed.",
            ),
            ConfirmAction::RemoveNetwork => self.run_selected_network_action(
                |service, id| service.remove_network(id),
                "Network removed.",
            ),
        }
    }

    fn start_selected_container(&mut self) {
        let _ = self.run_selected_container_action(
            |service, id| service.start_container(id),
            "Container started.",
        );
    }

    fn selected_container(&self) -> Option<&ContainerSummary> {
        self.selected_container_key().and_then(|key| {
            self.docker
                .resources
                .containers
                .items
                .iter()
                .find(|item| item.id == key)
        })
    }

    fn selected_image(&self) -> Option<&ImageSummary> {
        self.selected_row_key(ResourceKind::Images).and_then(|key| {
            self.docker
                .resources
                .images
                .items
                .iter()
                .find(|item| item.id == key)
        })
    }

    fn selected_volume(&self) -> Option<&VolumeSummary> {
        self.selected_row_key(ResourceKind::Volumes).and_then(|key| {
            self.docker
                .resources
                .volumes
                .items
                .iter()
                .find(|item| item.name == key)
        })
    }

    fn selected_network(&self) -> Option<&NetworkSummary> {
        self.selected_row_key(ResourceKind::Networks).and_then(|key| {
            self.docker
                .resources
                .networks
                .items
                .iter()
                .find(|item| item.id == key)
        })
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
                self.push_recent_action(self.status_message.clone());
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

    fn run_selected_image_action<F>(
        &mut self,
        action: F,
        success_prefix: &str,
    ) -> Option<AppCommand>
    where
        F: Fn(&DockerService, &str) -> Result<String, String>,
    {
        let Some((image_id, image_name)) = self
            .selected_image()
            .map(|image| (image.id.clone(), format!("{}:{}", image.repository, image.tag)))
        else {
            self.error_message = Some("No image selected.".to_string());
            return None;
        };

        let service = DockerService::new(&self.config.docker);
        match action(&service, &image_id) {
            Ok(output) => {
                self.status_message = format!("{success_prefix} {}", output.trim());
                self.push_recent_action(self.status_message.clone());
                self.error_message = None;
                self.screen = Screen::ResourceList(ResourceKind::Images);
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message = format!("Action failed for image `{}`.", image_name);
            }
        }

        None
    }

    fn run_selected_volume_action<F>(
        &mut self,
        action: F,
        success_prefix: &str,
    ) -> Option<AppCommand>
    where
        F: Fn(&DockerService, &str) -> Result<String, String>,
    {
        let Some(volume_name) = self.selected_volume().map(|volume| volume.name.clone()) else {
            self.error_message = Some("No volume selected.".to_string());
            return None;
        };

        let service = DockerService::new(&self.config.docker);
        match action(&service, &volume_name) {
            Ok(output) => {
                self.status_message = format!("{success_prefix} {}", output.trim());
                self.push_recent_action(self.status_message.clone());
                self.error_message = None;
                self.screen = Screen::ResourceList(ResourceKind::Volumes);
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message = format!("Action failed for volume `{}`.", volume_name);
            }
        }

        None
    }

    fn run_selected_network_action<F>(
        &mut self,
        action: F,
        success_prefix: &str,
    ) -> Option<AppCommand>
    where
        F: Fn(&DockerService, &str) -> Result<String, String>,
    {
        let Some((network_id, network_name)) = self
            .selected_network()
            .map(|network| (network.id.clone(), network.name.clone()))
        else {
            self.error_message = Some("No network selected.".to_string());
            return None;
        };

        let service = DockerService::new(&self.config.docker);
        match action(&service, &network_id) {
            Ok(output) => {
                self.status_message = format!("{success_prefix} {}", output.trim());
                self.push_recent_action(self.status_message.clone());
                self.error_message = None;
                self.screen = Screen::ResourceList(ResourceKind::Networks);
                self.refresh();
            }
            Err(error) => {
                self.error_message = Some(error);
                self.status_message = format!("Action failed for network `{}`.", network_name);
            }
        }

        None
    }

    fn build_container_rows(&self, query: &ResourceQuery<ContainerSummary>) -> Vec<ResourceRow> {
        query.items
            .iter()
            .map(|item| ResourceRow {
                key: item.id.clone(),
                line: format!("{} | {} | {}", item.names, item.state, item.image),
                preview: item.preview(),
                searchable: format!(
                    "{} {} {} {} {} {}",
                    item.id, item.names, item.image, item.state, item.status, item.ports
                )
                .to_lowercase(),
                status_bucket: if item.state.eq_ignore_ascii_case("running") {
                    ItemStatus::Active
                } else {
                    ItemStatus::Inactive
                },
                sort_name: item.names.to_lowercase(),
                sort_rank: if item.state.eq_ignore_ascii_case("running") {
                    0
                } else {
                    1
                },
                selected: false,
            })
            .collect()
    }

    fn build_image_rows(&self, query: &ResourceQuery<ImageSummary>) -> Vec<ResourceRow> {
        query.items
            .iter()
            .map(|item| ResourceRow {
                key: item.id.clone(),
                line: format!(
                    "{}:{} | {} | {}",
                    item.repository, item.tag, item.id, item.size
                ),
                preview: item.preview(),
                searchable: format!(
                    "{} {} {} {} {}",
                    item.id, item.repository, item.tag, item.size, item.created_since
                )
                .to_lowercase(),
                status_bucket: ItemStatus::Other,
                sort_name: format!("{}:{}", item.repository, item.tag).to_lowercase(),
                sort_rank: 1,
                selected: false,
            })
            .collect()
    }

    fn build_volume_rows(&self, query: &ResourceQuery<VolumeSummary>) -> Vec<ResourceRow> {
        query.items
            .iter()
            .map(|item| ResourceRow {
                key: item.name.clone(),
                line: format!("{} | {} | {}", item.name, item.driver, item.scope),
                preview: item.preview(),
                searchable: format!(
                    "{} {} {} {}",
                    item.name, item.driver, item.scope, item.mountpoint
                )
                .to_lowercase(),
                status_bucket: ItemStatus::Other,
                sort_name: item.name.to_lowercase(),
                sort_rank: 1,
                selected: false,
            })
            .collect()
    }

    fn build_network_rows(&self, query: &ResourceQuery<NetworkSummary>) -> Vec<ResourceRow> {
        query.items
            .iter()
            .map(|item| ResourceRow {
                key: item.id.clone(),
                line: format!("{} | {} | {}", item.name, item.driver, item.scope),
                preview: item.preview(),
                searchable: format!("{} {} {} {}", item.id, item.name, item.driver, item.scope)
                    .to_lowercase(),
                status_bucket: ItemStatus::Other,
                sort_name: item.name.to_lowercase(),
                sort_rank: 1,
                selected: false,
            })
            .collect()
    }

    fn apply_filter_and_sort(&self, mut rows: Vec<ResourceRow>) -> Vec<ResourceRow> {
        if !self.search_query.trim().is_empty() {
            let needle = self.search_query.to_lowercase();
            rows.retain(|row| row.searchable.contains(&needle));
        }

        match self.filter {
            ResourceFilter::All => {}
            ResourceFilter::Active => rows.retain(|row| row.status_bucket == ItemStatus::Active),
            ResourceFilter::Inactive => {
                rows.retain(|row| row.status_bucket == ItemStatus::Inactive)
            }
        }

        match self.sort {
            SortMode::NameAsc => rows.sort_by(|a, b| a.sort_name.cmp(&b.sort_name)),
            SortMode::NameDesc => rows.sort_by(|a, b| b.sort_name.cmp(&a.sort_name)),
            SortMode::Status => rows.sort_by(|a, b| {
                a.sort_rank
                    .cmp(&b.sort_rank)
                    .then_with(|| a.sort_name.cmp(&b.sort_name))
            }),
        }

        rows
    }

    fn selected_row_key(&self, kind: ResourceKind) -> Option<String> {
        self.resource_list_state(kind)
            .rows
            .get(self.list_index)
            .map(|row| row.key.clone())
    }

    fn selected_container_key(&self) -> Option<String> {
        self.selected_row_key(ResourceKind::Containers)
    }

    fn cycle_filter(&mut self) {
        self.filter = match self.filter {
            ResourceFilter::All => ResourceFilter::Active,
            ResourceFilter::Active => ResourceFilter::Inactive,
            ResourceFilter::Inactive => ResourceFilter::All,
        };
        self.list_index = 0;
        self.status_message = format!("Filter set to {}.", self.filter.label());
    }

    fn cycle_sort(&mut self) {
        self.sort = match self.sort {
            SortMode::NameAsc => SortMode::NameDesc,
            SortMode::NameDesc => SortMode::Status,
            SortMode::Status => SortMode::NameAsc,
        };
        self.list_index = 0;
        self.status_message = format!("Sort set to {}.", self.sort.label());
    }

    fn toggle_selected_row(&mut self) {
        let Screen::ResourceList(kind) = self.screen else {
            return;
        };

        let Some(key) = self.selected_row_key(kind) else {
            self.status_message = "No visible row to select.".to_string();
            return;
        };

        if self.selected_keys.contains(&key) {
            self.selected_keys.remove(&key);
            self.status_message = "Row removed from selection.".to_string();
        } else {
            self.selected_keys.insert(key);
            self.status_message = "Row added to selection.".to_string();
        }
    }

    fn toggle_select_all_visible(&mut self) {
        let Screen::ResourceList(kind) = self.screen else {
            return;
        };

        let state = self.resource_list_state(kind);
        if state.rows.is_empty() {
            self.status_message = "No visible rows to select.".to_string();
            return;
        }

        let all_selected = state
            .rows
            .iter()
            .all(|row| self.selected_keys.contains(&row.key));

        if all_selected {
            for row in &state.rows {
                self.selected_keys.remove(&row.key);
            }
            self.status_message = format!("Cleared {} visible selections.", state.rows.len());
        } else {
            for row in &state.rows {
                self.selected_keys.insert(row.key.clone());
            }
            self.status_message = format!("Selected all {} visible rows.", state.rows.len());
        }
    }

    fn push_recent_action(&mut self, action: String) {
        if action.trim().is_empty() {
            return;
        }
        self.recent_actions.push(action);
        if self.recent_actions.len() > 12 {
            let drain = self.recent_actions.len() - 12;
            self.recent_actions.drain(0..drain);
        }
    }
}

impl ResourceKind {
    fn label(&self) -> &'static str {
        match self {
            ResourceKind::Containers => "container",
            ResourceKind::Images => "image",
            ResourceKind::Volumes => "volume",
            ResourceKind::Networks => "network",
        }
    }
}

impl ResourceFilter {
    pub fn label(&self) -> &'static str {
        match self {
            ResourceFilter::All => "all",
            ResourceFilter::Active => "active",
            ResourceFilter::Inactive => "inactive",
        }
    }
}

impl SortMode {
    pub fn label(&self) -> &'static str {
        match self {
            SortMode::NameAsc => "name asc",
            SortMode::NameDesc => "name desc",
            SortMode::Status => "status",
        }
    }
}

impl CreateWizardState {
    pub fn new(docker: &DockerOverview) -> Self {
        let mut wizard = Self {
            step: CreateWizardStep::Image,
            image_mode: ImageInputMode::Select,
            image_index: 0,
            image_value: String::new(),
            name_value: String::new(),
            ports_value: String::new(),
            volumes_value: String::new(),
            env_value: String::new(),
            input_buffer: String::new(),
        };
        wizard.sync_after_refresh(docker);
        wizard
    }

    pub fn reset_for_open(&mut self, docker: &DockerOverview) {
        *self = Self::new(docker);
    }

    pub fn sync_after_refresh(&mut self, docker: &DockerOverview) {
        let image_count = docker.resources.images.items.len();
        if image_count == 0 {
            self.image_index = 0;
            self.image_mode = ImageInputMode::Manual;
            if self.image_value.is_empty() {
                self.input_buffer = self.image_value.clone();
            }
        } else if self.image_index >= image_count {
            self.image_index = image_count - 1;
        }
    }

    pub fn toggle_image_mode(&mut self) {
        self.image_mode = match self.image_mode {
            ImageInputMode::Select => {
                self.input_buffer = self.image_value.clone();
                ImageInputMode::Manual
            }
            ImageInputMode::Manual => ImageInputMode::Select,
        };
    }

    pub fn move_image_selection(&mut self, delta: isize, docker: &DockerOverview) {
        let len = docker.resources.images.items.len();
        if len == 0 {
            return;
        }

        if delta < 0 {
            self.image_index = self.image_index.saturating_sub(delta.unsigned_abs());
        } else {
            self.image_index = (self.image_index + delta as usize).min(len.saturating_sub(1));
        }
    }

    pub fn commit_image_step(&mut self, docker: &DockerOverview) -> Result<(), ()> {
        match self.image_mode {
            ImageInputMode::Select => {
                let Some(image) = docker.resources.images.items.get(self.image_index) else {
                    return Err(());
                };
                self.image_value = format!("{}:{}", image.repository, image.tag);
                self.input_buffer = self.name_value.clone();
                Ok(())
            }
            ImageInputMode::Manual => {
                if self.input_buffer.trim().is_empty() {
                    return Err(());
                }
                self.image_value = self.input_buffer.trim().to_string();
                self.input_buffer = self.name_value.clone();
                Ok(())
            }
        }
    }

    pub fn view(&self, docker: &DockerOverview) -> CreateWizardView {
        let image_rows = docker
            .resources
            .images
            .items
            .iter()
            .map(|image| format!("{}:{}", image.repository, image.tag))
            .collect::<Vec<_>>();

        let input_value = match self.step {
            CreateWizardStep::Image => match self.image_mode {
                ImageInputMode::Select => self
                    .selected_image_label(docker)
                    .unwrap_or_else(|| "No images available".to_string()),
                ImageInputMode::Manual => self.input_buffer.clone(),
            },
            _ => self.input_buffer.clone(),
        };

        CreateWizardView {
            step: self.step,
            title: format!("Create Container / {}", self.step.label()),
            prompt: self.step.prompt(),
            guidance: self.step.guidance(),
            input_value,
            image_mode: self.image_mode,
            image_rows,
            image_index: self.image_index,
            summary_lines: self.summary_lines(),
        }
    }

    fn selected_image_label(&self, docker: &DockerOverview) -> Option<String> {
        docker
            .resources
            .images
            .items
            .get(self.image_index)
            .map(|image| format!("{}:{}", image.repository, image.tag))
    }

    fn summary_lines(&self) -> Vec<String> {
        vec![
            format!("Image: {}", blank_display(&self.image_value)),
            format!("Name: {}", blank_display(&self.name_value)),
            format!("Ports: {}", blank_display(&self.ports_value)),
            format!("Volumes: {}", blank_display(&self.volumes_value)),
            format!("Env: {}", blank_display(&self.env_value)),
        ]
    }
}

impl CreateWizardStep {
    pub fn label(&self) -> &'static str {
        match self {
            CreateWizardStep::Image => "image",
            CreateWizardStep::Name => "name",
            CreateWizardStep::Ports => "ports",
            CreateWizardStep::Volumes => "volumes",
            CreateWizardStep::Env => "environment",
            CreateWizardStep::Summary => "summary",
        }
    }

    pub fn prompt(&self) -> String {
        match self {
            CreateWizardStep::Image => "Choose an image from the list or switch to manual input."
                .to_string(),
            CreateWizardStep::Name => "Enter a container name. Leave empty to let Docker choose."
                .to_string(),
            CreateWizardStep::Ports => {
                "Enter port mappings separated by commas, e.g. `8080:80, 8443:443`.".to_string()
            }
            CreateWizardStep::Volumes => {
                "Enter volume mounts separated by commas, e.g. `/host:/app, data:/data`."
                    .to_string()
            }
            CreateWizardStep::Env => {
                "Enter environment variables separated by commas, e.g. `KEY=VALUE, MODE=dev`."
                    .to_string()
            }
            CreateWizardStep::Summary => {
                "Review the request below. Press Enter to run `docker run -d`.".to_string()
            }
        }
    }

    pub fn guidance(&self) -> String {
        match self {
            CreateWizardStep::Image => {
                "Tab toggles between image selection and manual image input.".to_string()
            }
            CreateWizardStep::Name => {
                "Names help later management, but this step is optional.".to_string()
            }
            CreateWizardStep::Ports => {
                "Each mapping should follow `host:container`.".to_string()
            }
            CreateWizardStep::Volumes => {
                "Each mount should follow `source:target[:ro]`.".to_string()
            }
            CreateWizardStep::Env => {
                "Each environment entry should follow `KEY=VALUE`.".to_string()
            }
            CreateWizardStep::Summary => {
                "Creation runs in detached mode and returns the created container id.".to_string()
            }
        }
    }
}

impl ImageInputMode {
    pub fn label(&self) -> &'static str {
        match self {
            ImageInputMode::Select => "select",
            ImageInputMode::Manual => "manual",
        }
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

fn parse_csv_list(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn blank_display(value: &str) -> &str {
    if value.trim().is_empty() {
        "(empty)"
    } else {
        value
    }
}

fn blank_to_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

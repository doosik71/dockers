use crate::config::AppConfig;
use crate::docker::{DockerEnvironmentStatus, DockerService};

#[derive(Debug)]
pub struct App {
    pub config: AppConfig,
    pub docker: DockerEnvironmentStatus,
    pub title: &'static str,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let docker_service = DockerService::new(&config.docker);
        let docker = docker_service.inspect_environment();

        Self {
            config,
            docker,
            title: "dockers",
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

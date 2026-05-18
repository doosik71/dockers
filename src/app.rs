use crate::config::AppConfig;

#[derive(Debug)]
pub struct App {
    pub config: AppConfig,
    pub title: &'static str,
    pub should_quit: bool,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config,
            title: "dockers",
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

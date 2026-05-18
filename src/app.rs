#[derive(Debug)]
pub struct App {
    pub title: &'static str,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        Self {
            title: "dockers",
            should_quit: false,
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

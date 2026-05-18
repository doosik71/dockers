mod home;

use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    home::render(frame, app);
}

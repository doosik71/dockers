use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

pub struct MainAreas {
    pub header: Rect,
    pub content: Rect,
    pub error: Rect,
    pub status: Rect,
    pub help: Rect,
}

pub fn main_areas(frame: &Frame) -> MainAreas {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(frame.area());

    MainAreas {
        header: areas[0],
        content: areas[1],
        error: areas[2],
        status: areas[3],
        help: areas[4],
    }
}

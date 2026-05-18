use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let header = Paragraph::new(app.title)
        .block(Block::default().borders(Borders::ALL).title("dockers"))
        .style(Style::default().add_modifier(Modifier::BOLD));

    let body = Paragraph::new(
        "Docker를 텍스트 콘솔에서 쉽게 다루기 위한 TUI 골격입니다.\n\n다음 단계:\n- Docker 연결 확인\n- 리소스 목록 화면 추가\n- 작업 메뉴 연결",
    )
    .block(Block::default().borders(Borders::ALL).title("Home"));

    let footer = Paragraph::new("Press q or Esc to quit")
        .block(Block::default().borders(Borders::ALL).title("Help"));

    frame.render_widget(header, areas[0]);
    frame.render_widget(body, areas[1]);
    frame.render_widget(footer, areas[2]);
}

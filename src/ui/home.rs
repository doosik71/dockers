use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::docker::StatusLevel;

pub fn render(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .split(frame.area());

    let body = Paragraph::new(format!(
        "dockers TUI scaffold\n\nConfig:\n- docker command: {}\n- log level: {}\n- file logging: {}\n\nEnvironment checks:\n- {}: {}\n- {}: {}\n- {}: {}\n- {}: {}\n\nNext:\n- add resource list screens\n- wire command actions\n- support refresh",
        app.config.docker.command,
        app.config.logging.level,
        if app.config.logging.write_to_file {
            "on"
        } else {
            "off"
        },
        app.docker.installation.label,
        app.docker.installation.detail,
        app.docker.daemon.label,
        app.docker.daemon.detail,
        app.docker.version.label,
        app.docker.version.detail,
        app.docker.output_strategy.label,
        app.docker.output_strategy.detail,
    ))
    .block(Block::default().borders(Borders::ALL).title("Home"));

    let footer = Paragraph::new("Press q or Esc to quit")
        .block(Block::default().borders(Borders::ALL).title("Help"));

    let header_style = match overall_status(app) {
        StatusLevel::Ok | StatusLevel::Info => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        StatusLevel::Warning => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        StatusLevel::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let header = Paragraph::new(app.title)
        .block(Block::default().borders(Borders::ALL).title("dockers"))
        .style(header_style);

    frame.render_widget(header, areas[0]);
    frame.render_widget(body, areas[1]);
    frame.render_widget(footer, areas[2]);
}

fn overall_status(app: &App) -> StatusLevel {
    let levels = [
        app.docker.installation.level,
        app.docker.daemon.level,
        app.docker.version.level,
    ];

    if levels.contains(&StatusLevel::Error) {
        StatusLevel::Error
    } else if levels.contains(&StatusLevel::Warning) {
        StatusLevel::Warning
    } else {
        StatusLevel::Ok
    }
}

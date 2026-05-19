use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

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
        app.docker.environment.installation.label,
        app.docker.environment.installation.detail,
        app.docker.environment.daemon.label,
        app.docker.environment.daemon.detail,
        app.docker.environment.version.label,
        app.docker.environment.version.detail,
        app.docker.environment.output_strategy.label,
        app.docker.environment.output_strategy.detail,
    ))
    .block(Block::default().borders(Borders::ALL).title("Home"));

    let resources = Paragraph::new(format!(
        "Resource queries:\n- {}: {}\n  preview: {}\n- {}: {}\n  preview: {}\n- {}: {}\n  preview: {}\n- {}: {}\n  preview: {}",
        app.docker.resources.containers.label,
        app.docker.resources.containers.status.detail,
        app.docker.resources.containers.preview(),
        app.docker.resources.images.label,
        app.docker.resources.images.status.detail,
        app.docker.resources.images.preview(),
        app.docker.resources.volumes.label,
        app.docker.resources.volumes.status.detail,
        app.docker.resources.volumes.preview(),
        app.docker.resources.networks.label,
        app.docker.resources.networks.status.detail,
        app.docker.resources.networks.preview(),
    ))
    .block(Block::default().borders(Borders::ALL).title("Resources"));

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

    let header = Paragraph::new("dockers")
        .block(Block::default().borders(Borders::ALL).title("dockers"))
        .style(header_style);

    let middle = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(areas[1]);

    frame.render_widget(header, areas[0]);
    frame.render_widget(body, middle[0]);
    frame.render_widget(resources, middle[1]);
    frame.render_widget(footer, areas[2]);
}

fn overall_status(app: &App) -> StatusLevel {
    let levels = [
        app.docker.environment.installation.level,
        app.docker.environment.daemon.level,
        app.docker.environment.version.level,
        app.docker.resources.containers.status.level,
        app.docker.resources.images.status.level,
        app.docker.resources.volumes.status.level,
        app.docker.resources.networks.status.level,
    ];

    if levels.contains(&StatusLevel::Error) {
        StatusLevel::Error
    } else if levels.contains(&StatusLevel::Warning) {
        StatusLevel::Warning
    } else {
        StatusLevel::Ok
    }
}

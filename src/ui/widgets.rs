use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::app::{App, ConfirmState, CreateWizardStep, CreateWizardView, ImageInputMode, ResourceListState, Screen, TextViewContent};
use crate::docker::StatusLevel;

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let style = match overall_status(app) {
        StatusLevel::Ok | StatusLevel::Info => Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD),
        StatusLevel::Warning => Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
        StatusLevel::Error => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
    };

    let subtitle = match app.screen {
        Screen::MainMenu => "Main Menu",
        Screen::ResourceList(_) => "Resource List",
        Screen::TextView(_) => "Text View",
        Screen::CreateWizard => "Create Wizard",
    };

    let header = Paragraph::new(Line::from(vec![
        Span::styled(app.title, Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(subtitle, Style::default().fg(Color::DarkGray)),
    ]))
    .block(Block::default().borders(Borders::ALL).title("dockers"))
    .style(style);

    frame.render_widget(header, area);
}

pub fn render_main_menu(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    let items = app
        .menu_items()
        .into_iter()
        .map(|item| ListItem::new(Line::from(vec![
            Span::styled(item.label, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(item.detail, Style::default().fg(Color::Gray)),
        ])))
        .collect::<Vec<_>>();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"))
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let mut list_state = ListState::default().with_selected(Some(app.menu_index));
    frame.render_stateful_widget(list, columns[0], &mut list_state);

    let overview = Paragraph::new(format!(
        "Choose a Docker resource category.\n\nSelection:\n{}\n\nSummary:\n{}\n\nEnvironment:\n- {}\n- {}\n- {}\n- {}",
        app.menu_items()[app.menu_index].label,
        app.selected_row_preview(),
        app.docker.environment.installation.detail,
        app.docker.environment.daemon.detail,
        app.docker.environment.version.detail,
        app.docker.environment.output_strategy.detail,
    ))
    .block(Block::default().borders(Borders::ALL).title("Overview"))
    .wrap(Wrap { trim: true });

    frame.render_widget(overview, columns[1]);
}

pub fn render_resource_list(frame: &mut Frame, area: Rect, app: &App, state: &ResourceListState) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let list_items = if state.rows.is_empty() {
        vec![ListItem::new("No rows available")]
    } else {
        state
            .rows
            .iter()
            .map(|row| {
                let marker = if row.selected { "[x]" } else { "[ ]" };
                ListItem::new(format!("{marker} {}", row.line))
            })
            .collect::<Vec<_>>()
    };

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} / {}", state.title, query_label(app))),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    let selected = if state.rows.is_empty() {
        Some(0)
    } else {
        Some(app.list_index.min(state.rows.len().saturating_sub(1)))
    };
    let mut list_state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(list, sections[0], &mut list_state);

    let detail_style = match state.status {
        StatusLevel::Ok | StatusLevel::Info => Style::default().fg(Color::Green),
        StatusLevel::Warning => Style::default().fg(Color::Yellow),
        StatusLevel::Error => Style::default().fg(Color::Red),
    };

    let detail = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Query status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(state.detail.clone(), detail_style),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Search: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(if state.search_query.is_empty() {
                "(empty)".to_string()
            } else {
                state.search_query.clone()
            }),
        ]),
        Line::from(vec![
            Span::styled("Filter: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(state.filter.label()),
            Span::raw("   "),
            Span::styled("Sort: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(state.sort.label()),
        ]),
        Line::from(vec![
            Span::styled("Counts: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                "{} visible / {} total / {} selected",
                state.visible_count, state.total_count, state.selected_count
            )),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Selected row: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(app.selected_row_preview()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Preview: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(state.preview.clone()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Actions: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(state.action_hint),
        ]),
    ])
    .block(Block::default().borders(Borders::ALL).title("Details"))
    .wrap(Wrap { trim: true });

    frame.render_widget(detail, sections[1]);
}

pub fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let panel = Paragraph::new(error)
        .block(Block::default().borders(Borders::ALL).title("Errors"))
        .style(Style::default().fg(Color::Red))
        .wrap(Wrap { trim: true });

    frame.render_widget(panel, area);
}

pub fn render_text_view(frame: &mut Frame, area: Rect, view: &TextViewContent) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    let meta = Paragraph::new(view.subtitle.clone())
        .block(Block::default().borders(Borders::ALL).title(view.title.clone()))
        .style(Style::default().fg(Color::Cyan));

    let body = Paragraph::new(view.body.clone())
        .block(Block::default().borders(Borders::ALL).title("Content"))
        .wrap(Wrap { trim: false });

    frame.render_widget(meta, sections[0]);
    frame.render_widget(body, sections[1]);
}

pub fn render_create_wizard(frame: &mut Frame, area: Rect, view: &CreateWizardView) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(sections[0]);

    let prompt = Paragraph::new(view.prompt.clone())
        .block(Block::default().borders(Borders::ALL).title(view.title.clone()))
        .wrap(Wrap { trim: true });

    frame.render_widget(prompt, left[0]);

    match view.step {
        CreateWizardStep::Image => {
            let items = if view.image_rows.is_empty() {
                vec![ListItem::new("No images available. Press Tab for manual image input.")]
            } else {
                view.image_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        let marker = if index == view.image_index { ">> " } else { "   " };
                        ListItem::new(format!("{marker}{row}"))
                    })
                    .collect::<Vec<_>>()
            };

            let title = match view.image_mode {
                ImageInputMode::Select => "Image Selection",
                ImageInputMode::Manual => "Manual Image Input",
            };

            let list = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
            frame.render_widget(list, left[1]);
        }
        _ => {
            let input = Paragraph::new(view.input_value.clone())
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .wrap(Wrap { trim: false });
            frame.render_widget(input, left[1]);
        }
    }

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(8)])
        .split(sections[1]);

    let guidance = Paragraph::new(format!(
        "Step: {}\nMode: {}\n{}",
        view.step.label(),
        match view.image_mode {
            ImageInputMode::Select => "select",
            ImageInputMode::Manual => "manual",
        },
        view.guidance
    ))
    .block(Block::default().borders(Borders::ALL).title("Guidance"))
    .wrap(Wrap { trim: true });

    let summary = Paragraph::new(view.summary_lines.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Request Summary"))
        .wrap(Wrap { trim: true });

    frame.render_widget(guidance, right[0]);
    frame.render_widget(summary, right[1]);
}

pub fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let left = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(20), Constraint::Percentage(35)])
        .split(area);

    let status = Paragraph::new(app.status_message.clone())
        .block(Block::default().borders(Borders::ALL).title("Status"))
        .wrap(Wrap { trim: true });
    let progress = Paragraph::new(app.progress_text())
        .block(Block::default().borders(Borders::ALL).title("Progress"));
    let summary = Paragraph::new(app.recent_actions_summary())
        .block(Block::default().borders(Borders::ALL).title("Recent Actions"))
        .wrap(Wrap { trim: true });

    frame.render_widget(status, left[0]);
    frame.render_widget(progress, left[1]);
    frame.render_widget(summary, left[2]);
}

pub fn render_help(frame: &mut Frame, area: Rect, app: &App) {
    let help = Paragraph::new(format!(
        "{}{}",
        app.help_text(),
        if app.search_mode {
            "  |  Search mode: type, Backspace edit, Enter apply, Esc cancel"
        } else {
            ""
        }
    ))
        .block(Block::default().borders(Borders::ALL).title("Help"))
        .style(Style::default().fg(Color::Gray));

    frame.render_widget(help, area);
}

pub fn render_confirm(frame: &mut Frame, confirm: &ConfirmState) {
    let popup = centered_rect(60, 30, frame.area());
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),
            Constraint::Length(3),
        ])
        .margin(1)
        .split(popup);

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(confirm.title.clone())
            .style(Style::default().bg(Color::Black)),
        popup,
    );

    let message = Paragraph::new(confirm.message.clone()).wrap(Wrap { trim: true });
    frame.render_widget(message, areas[0]);

    let confirm_style = if confirm.selected_confirm {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let cancel_style = if confirm.selected_confirm {
        Style::default()
    } else {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    let buttons = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", confirm.cancel_label),
            cancel_style.add_modifier(Modifier::BOLD),
        ),
        Span::raw("   "),
        Span::styled(
            format!(" {} ", confirm.confirm_label),
            confirm_style.add_modifier(Modifier::BOLD),
        ),
    ]));

    frame.render_widget(buttons, areas[1]);
}

fn overall_status(app: &App) -> StatusLevel {
    let mut levels = app
        .status_lines()
        .into_iter()
        .map(|(_, level)| level)
        .collect::<Vec<_>>();
    levels.push(app.docker.resources.containers.status.level);
    levels.push(app.docker.resources.images.status.level);
    levels.push(app.docker.resources.volumes.status.level);
    levels.push(app.docker.resources.networks.status.level);

    if levels.contains(&StatusLevel::Error) {
        StatusLevel::Error
    } else if levels.contains(&StatusLevel::Warning) {
        StatusLevel::Warning
    } else {
        StatusLevel::Ok
    }
}

fn query_label(app: &App) -> &'static str {
    match app.screen {
        Screen::MainMenu => "Menu",
        Screen::ResourceList(kind) => match kind {
            crate::app::ResourceKind::Containers => app.docker.resources.containers.label,
            crate::app::ResourceKind::Images => app.docker.resources.images.label,
            crate::app::ResourceKind::Volumes => app.docker.resources.volumes.label,
            crate::app::ResourceKind::Networks => app.docker.resources.networks.label,
        },
        Screen::TextView(_) => "Viewer",
        Screen::CreateWizard => "Wizard",
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::{
    App, ConfirmState, CreateWizardStep, CreateWizardView, HubFocus, ImageInputMode,
    ResourceActionButton, ResourceFocus, ResourceKind, ResourceListState, Screen, TextViewContent,
};
use crate::docker::StatusLevel;

const BG: Color = Color::Rgb(18, 22, 27);
const PANEL: Color = Color::Rgb(30, 36, 43);
const PANEL_ALT: Color = Color::Rgb(38, 45, 54);
const FG: Color = Color::Rgb(232, 238, 243);
const MUTED: Color = Color::Rgb(156, 168, 179);
const BORDER: Color = Color::Rgb(90, 103, 116);
const ACCENT: Color = Color::Rgb(73, 180, 190);
const ACCENT_DARK: Color = Color::Rgb(12, 64, 73);
const HOTKEY: Color = Color::Rgb(255, 214, 102);
const OK: Color = Color::Rgb(117, 216, 128);
const WARN: Color = Color::Rgb(255, 196, 87);
const ERROR: Color = Color::Rgb(255, 118, 118);
const SELECT_BG: Color = Color::Rgb(58, 104, 117);

pub fn render_background(frame: &mut Frame) {
    frame.render_widget(Block::default().style(base_style()), frame.area());
}

fn base_style() -> Style {
    Style::default().fg(FG).bg(BG)
}

fn panel_style() -> Style {
    Style::default().fg(FG).bg(PANEL)
}

fn muted_style() -> Style {
    Style::default().fg(MUTED).bg(PANEL)
}

fn label_style() -> Style {
    Style::default()
        .fg(FG)
        .bg(PANEL)
        .add_modifier(Modifier::BOLD)
}

fn block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title.into())
        .style(panel_style())
        .border_style(Style::default().fg(BORDER).bg(PANEL))
}

fn focused_block(title: impl Into<String>, focused: bool) -> Block<'static> {
    let border = if focused { HOTKEY } else { BORDER };
    block(title).border_style(Style::default().fg(border).bg(PANEL))
}

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let active = app.active_resource_kind();
    let mut spans = Vec::new();
    let is_menu_focused = app.resource_focus == ResourceFocus::Menu;

    for (index, kind) in [
        ResourceKind::Containers,
        ResourceKind::Images,
        ResourceKind::Volumes,
        ResourceKind::Networks,
        ResourceKind::Hub,
    ]
    .iter()
    .enumerate()
    {
        if index > 0 {
            spans.push(Span::styled("  ", panel_style()));
        }

        let is_focused = is_menu_focused && index == app.menu_index;
        let is_active = *kind == active;

        spans.extend(menu_button_spans(index + 1, *kind, is_active, is_focused));
    }

    let border_color = if is_menu_focused {
        HOTKEY
    } else {
        match overall_status(app) {
            StatusLevel::Ok | StatusLevel::Info => BORDER,
            StatusLevel::Warning => WARN,
            StatusLevel::Error => ERROR,
        }
    };

    let header = Paragraph::new(Line::from(spans))
        .block(block("Main Menu").border_style(Style::default().fg(border_color).bg(PANEL)))
        .style(panel_style());

    frame.render_widget(header, area);
}

fn menu_button_spans(
    function_number: usize,
    kind: ResourceKind,
    active: bool,
    focused: bool,
) -> Vec<Span<'static>> {
    let bg = if focused {
        ACCENT
    } else if active {
        SELECT_BG
    } else {
        PANEL_ALT
    };

    let label_fg = if focused { BG } else { FG };
    let hotkey_fg = if focused { BG } else { HOTKEY };

    let label_style = Style::default()
        .fg(label_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let hotkey_style = Style::default()
        .fg(hotkey_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    vec![
        Span::styled("[", label_style),
        Span::styled(format!("F{function_number}"), hotkey_style),
        Span::styled(format!(" {}]", kind.title()), label_style),
    ]
}

pub fn render_resource_list(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    kind: ResourceKind,
    state: &ResourceListState,
) {
    let sections = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(area);

    let list_items = if state.rows.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No rows available",
            muted_style(),
        )))]
    } else {
        state
            .rows
            .iter()
            .map(|row| {
                let marker = if row.selected { "[x]" } else { "[ ]" };
                let marker_style = if row.selected {
                    Style::default()
                        .fg(HOTKEY)
                        .bg(PANEL)
                        .add_modifier(Modifier::BOLD)
                } else {
                    muted_style()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(marker, marker_style),
                    Span::styled(" ", panel_style()),
                    Span::styled(row.line.clone(), panel_style()),
                ]))
            })
            .collect::<Vec<_>>()
    };

    let list_focused = app.resource_focus == ResourceFocus::List;
    let highlight_bg = if list_focused { SELECT_BG } else { PANEL_ALT };
    let list = List::new(list_items)
        .block(focused_block(
            format!("{} / {}", state.title, query_label(app)),
            list_focused,
        ))
        .style(panel_style())
        .highlight_style(
            Style::default()
                .fg(FG)
                .bg(highlight_bg)
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

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Length(3),
            Constraint::Min(8),
        ])
        .split(sections[1]);

    render_action_buttons(frame, right[0], app, kind);
    render_search_box(frame, right[1], app, state);

    let details_focused = app.resource_focus == ResourceFocus::Details;
    let detail_style = status_style(state.status);
    let detail = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Query status: ", label_style()),
            Span::styled(state.detail.clone(), detail_style),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Filter: ", label_style()),
            Span::styled(state.filter.label(), panel_style()),
            Span::styled("   ", panel_style()),
            Span::styled("Sort: ", label_style()),
            Span::styled(state.sort.label(), panel_style()),
        ]),
        Line::from(vec![
            Span::styled("Counts: ", label_style()),
            Span::styled(
                format!(
                    "{} visible / {} total / {} selected",
                    state.visible_count, state.total_count, state.selected_count
                ),
                panel_style(),
            ),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Selected row: ", label_style()),
            Span::styled(app.selected_row_preview(), panel_style()),
        ]),
        Line::raw(""),
        Line::from(vec![
            Span::styled("Preview: ", label_style()),
            Span::styled(state.preview.clone(), panel_style()),
        ]),
    ])
    .block(focused_block("Details", details_focused))
    .style(panel_style())
    .scroll((app.detail_scroll, 0))
    .wrap(Wrap { trim: true });

    frame.render_widget(detail, right[2]);
}

fn render_action_buttons(frame: &mut Frame, area: Rect, app: &App, kind: ResourceKind) {
    let actions = app.resource_actions(kind);
    let focused = app.resource_focus == ResourceFocus::Actions
        || (app.screen == crate::app::Screen::ImageSearch
            && app.image_search.focus == HubFocus::Actions);
    let mut spans = Vec::new();

    for (index, button) in actions.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" ", panel_style()));
        }
        spans.extend(action_button_spans(
            *button,
            focused && index == app.action_index,
        ));
    }

    if spans.is_empty() {
        spans.push(Span::styled("No actions available", muted_style()));
    }

    let actions = Paragraph::new(Line::from(spans))
        .block(focused_block("Actions", focused))
        .style(panel_style())
        .wrap(Wrap { trim: false });

    frame.render_widget(actions, area);
}

fn action_button_spans(button: ResourceActionButton, focused: bool) -> Vec<Span<'static>> {
    let bg = if focused { ACCENT_DARK } else { PANEL_ALT };
    let fg = if focused { FG } else { FG };
    let style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
    let hotkey_style = Style::default()
        .fg(HOTKEY)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    vec![
        Span::styled("[", style),
        Span::styled(button.hotkey.to_string(), hotkey_style),
        Span::styled(format!(" {}]", button.label), style),
    ]
}

fn render_search_box(frame: &mut Frame, area: Rect, app: &App, state: &ResourceListState) {
    let focused = app.resource_focus == ResourceFocus::Search;
    let bg = if focused { ACCENT_DARK } else { PANEL };
    let border = if focused { HOTKEY } else { BORDER };
    let text = if focused {
        format!("{}|", state.search_query)
    } else if state.search_query.is_empty() {
        "(empty)".to_string()
    } else {
        state.search_query.clone()
    };

    let search_style = Style::default().fg(FG).bg(bg);
    let search = Paragraph::new(Line::from(vec![
        Span::styled("Search: ", search_style.add_modifier(Modifier::BOLD)),
        Span::styled(text, search_style),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search")
            .style(search_style)
            .border_style(Style::default().fg(border).bg(bg)),
    )
    .style(search_style);

    frame.render_widget(search, area);
}

pub fn render_image_search(frame: &mut Frame, area: Rect, app: &App) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(10),
        ])
        .split(area);

    let query_focused = app.image_search.focus == HubFocus::Query;
    let results_focused = app.image_search.focus == HubFocus::Results;

    // Render Query Box
    let search_style = if query_focused {
        Style::default().fg(HOTKEY).bg(ACCENT_DARK)
    } else {
        panel_style()
    };

    let query_box = Paragraph::new(Line::from(vec![
        Span::styled("Hub Search: ", search_style.add_modifier(Modifier::BOLD)),
        Span::styled(
            format!(
                "{}{}",
                app.image_search.query,
                if query_focused { "|" } else { "" }
            ),
            search_style,
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Search Docker Hub")
            .style(search_style)
            .border_style(
                Style::default()
                    .fg(if query_focused { HOTKEY } else { BORDER })
                    .bg(PANEL),
            ),
    )
    .style(search_style);

    frame.render_widget(query_box, sections[0]);

    // Render Actions Box
    render_action_buttons(frame, sections[1], app, ResourceKind::Hub);

    // Render Results and Details
    let results_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(sections[2]);

    let items = if app.image_search.results.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "Enter query and press Enter to search.",
            muted_style(),
        )))]
    } else {
        app.image_search
            .results
            .iter()
            .enumerate()
            .map(|(index, res)| {
                let selected = index == app.list_index && results_focused;
                let style = if selected {
                    Style::default()
                        .fg(FG)
                        .bg(SELECT_BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    panel_style()
                };
                let official = if res.official == "true" { " [O]" } else { "" };
                ListItem::new(Line::from(vec![
                    Span::styled(if selected { ">> " } else { "   " }, style),
                    Span::styled(res.name.clone(), style),
                    Span::styled(official, Style::default().fg(OK).bg(style.bg.unwrap_or(PANEL))),
                    Span::styled("  ★ ", Style::default().fg(HOTKEY).bg(style.bg.unwrap_or(PANEL))),
                    Span::styled(format!("{}", res.stars), muted_style().bg(style.bg.unwrap_or(PANEL))),
                ]))
            })
            .collect::<Vec<_>>()
    };

    let list_block = block("Search Results").border_style(
        Style::default()
            .fg(if results_focused { HOTKEY } else { BORDER })
            .bg(PANEL),
    );

    let list = List::new(items).block(list_block).style(panel_style());
    frame.render_widget(list, results_area[0]);

    let preview = Paragraph::new(
        app.image_search
            .results
            .get(app.list_index)
            .map(|res| res.preview())
            .unwrap_or_else(|| "No selection".to_string()),
    )
    .block(block("Result Details"))
    .style(panel_style())
    .wrap(Wrap { trim: true });

    frame.render_widget(preview, results_area[1]);
}

pub fn render_error(frame: &mut Frame, area: Rect, error: &str) {
    let panel = Paragraph::new(error)
        .block(block("Errors"))
        .style(Style::default().fg(ERROR).bg(PANEL))
        .wrap(Wrap { trim: true });

    frame.render_widget(panel, area);
}

pub fn render_text_view(frame: &mut Frame, area: Rect, view: &TextViewContent, scroll: u16) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(area);

    let meta = Paragraph::new(view.subtitle.clone())
        .block(block(view.title.clone()))
        .style(Style::default().fg(ACCENT).bg(PANEL));

    let body = Paragraph::new(view.body.clone())
        .block(
            block("Content")
                .border_style(Style::default().fg(HOTKEY).bg(PANEL))
                .style(Style::default().fg(FG).bg(PANEL)),
        )
        .style(panel_style())
        .scroll((scroll, 0))
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
        .block(block(view.title.clone()))
        .style(panel_style())
        .wrap(Wrap { trim: true });

    frame.render_widget(prompt, left[0]);

    match view.step {
        CreateWizardStep::Image => {
            let items = if view.image_rows.is_empty() {
                vec![ListItem::new(Line::from(Span::styled(
                    "No images available. Press Tab for manual image input.",
                    muted_style(),
                )))]
            } else {
                view.image_rows
                    .iter()
                    .enumerate()
                    .map(|(index, row)| {
                        let selected = index == view.image_index;
                        let style = if selected {
                            Style::default()
                                .fg(FG)
                                .bg(SELECT_BG)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            panel_style()
                        };
                        let marker = if selected { ">> " } else { "   " };
                        ListItem::new(Line::from(vec![
                            Span::styled(marker, style),
                            Span::styled(row.clone(), style),
                        ]))
                    })
                    .collect::<Vec<_>>()
            };

            let title = match view.image_mode {
                ImageInputMode::Select => "Image Selection",
                ImageInputMode::Manual => "Manual Image Input",
            };

            let list = List::new(items).block(block(title)).style(panel_style());
            frame.render_widget(list, left[1]);
        }
        _ => {
            let input = Paragraph::new(view.input_value.clone())
                .block(block("Input"))
                .style(panel_style())
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
    .block(block("Guidance"))
    .style(panel_style())
    .wrap(Wrap { trim: true });

    let summary = Paragraph::new(view.summary_lines.join("\n"))
        .block(block("Request Summary"))
        .style(panel_style())
        .wrap(Wrap { trim: true });

    frame.render_widget(guidance, right[0]);
    frame.render_widget(summary, right[1]);
}

pub fn render_status(frame: &mut Frame, area: Rect, app: &App) {
    let left = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
        ])
        .split(area);

    let status = Paragraph::new(app.status_message.clone())
        .block(block("Status"))
        .style(panel_style())
        .wrap(Wrap { trim: true });
    let progress = Paragraph::new(app.progress_text())
        .block(block("Progress"))
        .style(panel_style());
    let summary = Paragraph::new(app.recent_actions_summary())
        .block(block("Recent Actions"))
        .style(panel_style())
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
    .block(block("Help"))
    .style(Style::default().fg(MUTED).bg(PANEL));

    frame.render_widget(help, area);
}

pub fn render_confirm(frame: &mut Frame, confirm: &ConfirmState) {
    let popup = centered_rect(60, 25, frame.area());
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .margin(1)
        .split(popup);

    frame.render_widget(Clear, popup);
    frame.render_widget(
        Block::default()
            .borders(Borders::ALL)
            .title(confirm.title.clone())
            .style(panel_style())
            .border_style(Style::default().fg(HOTKEY).bg(PANEL)),
        popup,
    );

    let message = Paragraph::new(confirm.message.clone())
        .style(panel_style())
        .wrap(Wrap { trim: true });
    frame.render_widget(message, areas[0]);

    let confirm_style = Style::default()
        .fg(FG)
        .bg(ACCENT_DARK)
        .add_modifier(Modifier::BOLD);

    let buttons = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", confirm.confirm_label),
        confirm_style,
    )]))
    .style(panel_style())
    .alignment(ratatui::layout::Alignment::Center);

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

fn status_style(status: StatusLevel) -> Style {
    let fg = match status {
        StatusLevel::Ok | StatusLevel::Info => OK,
        StatusLevel::Warning => WARN,
        StatusLevel::Error => ERROR,
    };

    Style::default()
        .fg(fg)
        .bg(PANEL)
        .add_modifier(Modifier::BOLD)
}

fn query_label(app: &App) -> &'static str {
    match app.screen {
        Screen::ResourceList(kind) => match kind {
            crate::app::ResourceKind::Containers => app.docker.resources.containers.label,
            crate::app::ResourceKind::Images => app.docker.resources.images.label,
            crate::app::ResourceKind::Volumes => app.docker.resources.volumes.label,
            crate::app::ResourceKind::Networks => app.docker.resources.networks.label,
            crate::app::ResourceKind::Hub => "Remote Hub",
        },
        Screen::TextView(_) => "Viewer",
        Screen::CreateWizard => "Wizard",
        Screen::ImageSearch => "Search",
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

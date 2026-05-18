mod layout;
mod widgets;

use ratatui::Frame;

use crate::app::{App, ConfirmState, ResourceKind, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    let areas = layout::main_areas(frame);

    widgets::render_header(frame, areas.header, app);
    widgets::render_status(frame, areas.status, app);
    widgets::render_help(frame, areas.help, app);

    match app.screen {
        Screen::MainMenu => widgets::render_main_menu(frame, areas.content, app),
        Screen::ResourceList(kind) => render_resource_list(frame, areas.content, app, kind),
    }

    if let Some(error) = &app.error_message {
        widgets::render_error(frame, areas.error, error);
    }

    if let Some(confirm) = &app.confirm {
        render_confirm(frame, confirm);
    }
}

fn render_resource_list(frame: &mut Frame, area: ratatui::layout::Rect, app: &App, kind: ResourceKind) {
    let state = app.resource_list_state(kind);
    widgets::render_resource_list(frame, area, app, &state);
}

fn render_confirm(frame: &mut Frame, confirm: &ConfirmState) {
    widgets::render_confirm(frame, confirm);
}

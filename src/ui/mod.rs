mod layout;
mod widgets;

use ratatui::Frame;

use crate::app::{App, ConfirmState, ResourceKind, Screen};

pub fn render(frame: &mut Frame, app: &App) {
    widgets::render_background(frame);

    let areas = layout::main_areas(frame);

    widgets::render_header(frame, areas.header, app);
    widgets::render_status(frame, areas.status, app);
    widgets::render_help(frame, areas.help, app);

    match app.screen {
        Screen::ResourceList(kind) => render_resource_list(frame, areas.content, app, kind),
        Screen::TextView(state) => widgets::render_text_view(
            frame,
            areas.content,
            &app.text_view_content(state),
            state.scroll,
        ),
        Screen::CreateWizard => {
            widgets::render_create_wizard(frame, areas.content, &app.wizard_view())
        }
        Screen::ImageSearch => widgets::render_image_search(frame, areas.content, app),
    }

    if let Some(error) = &app.error_message {
        widgets::render_error(frame, areas.error, error);
    }

    if let Some(confirm) = &app.confirm {
        render_confirm(frame, confirm);
    }
}

fn render_resource_list(
    frame: &mut Frame,
    area: ratatui::layout::Rect,
    app: &App,
    kind: ResourceKind,
) {
    let state = app.resource_list_state(kind);
    widgets::render_resource_list(frame, area, app, kind, &state);
}

fn render_confirm(frame: &mut Frame, confirm: &ConfirmState) {
    widgets::render_confirm(frame, confirm);
}

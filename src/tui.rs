use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use crate::app::{App, AppCommand};
use crate::docker::DockerService;
use crate::error::Result;
use crate::ui;

pub fn run(app: &mut App) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_app(&mut terminal, app);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;

    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|frame| ui::render(frame, app))?;
        app.tick();

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if let Some(command) = app.handle_key(key.code) {
            execute_app_command(terminal, app, command)?;
        }
    }

    Ok(())
}

fn execute_app_command(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    command: AppCommand,
) -> Result<()> {
    match command {
        AppCommand::OpenContainerShell {
            container_id,
            container_name,
        } => {
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let service = DockerService::new(&app.config.docker);
            let result = service.open_container_shell(&container_id);

            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.hide_cursor()?;

            app.handle_shell_result(result, &container_name);
        }
    }

    Ok(())
}

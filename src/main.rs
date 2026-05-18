mod app;
mod config;
mod docker;
mod error;
mod logging;
mod process;
mod tui;
mod ui;

use crate::app::App;
use crate::config::AppConfig;
use crate::error::Result;

fn main() -> Result<()> {
    color_eyre::install().map_err(|error| crate::error::AppError::startup(error.to_string()))?;

    let config = AppConfig::default();
    logging::init(&config.logging)?;

    let mut app = App::new(config);
    tui::run(&mut app)
}

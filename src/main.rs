mod app;
mod error;
mod tui;
mod ui;

use crate::app::App;
use crate::error::Result;

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut app = App::new();
    tui::run(&mut app)
}

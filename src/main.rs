mod app;
mod storage;
mod typing;
mod ui;
mod words;

use std::io;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = app::App::new().and_then(|mut app| app.run(&mut terminal));
    ratatui::restore();
    result
}

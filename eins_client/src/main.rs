mod api;
mod app;
mod ui;

use std::io::stdout;
use std::time::Duration;

use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton, MouseEventKind,
};
use ratatui::DefaultTerminal;

use crate::api::Api;
use crate::app::App;

/// Usage: `eins_client [server URL]`, by default `http://127.0.0.1:8080`.
fn main() -> std::io::Result<()> {
    let server = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    // Card colors are game information, so they stay on even when NO_COLOR is set.
    crossterm::style::force_color_output(true);
    let mut terminal = ratatui::init();
    crossterm::execute!(stdout(), EnableMouseCapture)?;
    let result = run(&mut terminal, App::new(Api::new(server)));
    crossterm::execute!(stdout(), DisableMouseCapture)?;
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, mut app: App) -> std::io::Result<()> {
    while !app.quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;
        if event::poll(Duration::from_millis(200))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key),
                Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                    app.on_click(mouse.column, mouse.row)
                }
                _ => {}
            }
        }
        app.tick();
    }
    Ok(())
}

use app::{App, Message};
use color_eyre::eyre::Result;
use ratatui::{
    DefaultTerminal,
    crossterm::event::{self, Event},
};
use ui::ui;

mod app;
mod ui;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = run(terminal);
    ratatui::restore();
    result
}

fn run(mut terminal: DefaultTerminal) -> Result<()> {
    let mut app = App::new();
    loop {
        terminal.draw(|f| ui(f, &app))?;
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Release {
                continue;
            }
            let Some(mut message) = app.handle_key(key) else {
                continue;
            };
            while let Some(new_message) = app.update(message) {
                if matches!(new_message, Message::Quit) {
                    return Ok(());
                }

                terminal.draw(|f| ui(f, &app))?;
                message = new_message;
            }
        }
    }
}

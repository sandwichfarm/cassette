pub mod components;
pub mod animations;
pub mod colors;
pub mod terminal;
pub mod record;
pub mod play;
pub mod dub;
pub mod cast;

use std::io::{self};
use crossterm::{
    cursor,
    execute,
    terminal::ClearType,
};

pub struct InteractiveUI {
    pub enabled: bool,
}

impl InteractiveUI {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    pub fn init(&self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        execute!(
            io::stdout(),
            crossterm::terminal::Clear(ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0)
        )?;

        Ok(())
    }

    pub fn cleanup(&self) -> io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        execute!(
            io::stdout(),
            cursor::Show,
            cursor::MoveTo(0, 0)
        )?;

        Ok(())
    }
}
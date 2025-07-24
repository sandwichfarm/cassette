use crossterm::{
    cursor,
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, stdout};

pub fn clear_screen() -> io::Result<()> {
    execute!(stdout(), terminal::Clear(ClearType::All))
}

pub fn move_cursor(x: u16, y: u16) -> io::Result<()> {
    execute!(stdout(), cursor::MoveTo(x, y))
}

pub fn hide_cursor() -> io::Result<()> {
    execute!(stdout(), cursor::Hide)
}

pub fn show_cursor() -> io::Result<()> {
    execute!(stdout(), cursor::Show)
}

pub fn clear_line() -> io::Result<()> {
    execute!(stdout(), terminal::Clear(ClearType::CurrentLine))
}

pub fn get_terminal_size() -> io::Result<(u16, u16)> {
    terminal::size()
}
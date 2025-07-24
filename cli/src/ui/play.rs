use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use super::{colors, components::Counter, animations::TapeReel};

pub struct PlayUI {
    tape_reel: TapeReel,
    event_counter: Counter,
    total_events: u64,
    current_event: u64,
}

impl PlayUI {
    pub fn new() -> Self {
        Self {
            tape_reel: TapeReel::new(8),
            event_counter: Counter::new(0),
            total_events: 0,
            current_event: 0,
        }
    }

    pub fn init(&self) -> io::Result<()> {
        execute!(
            io::stdout(),
            terminal::EnterAlternateScreen,
            cursor::Hide,
            Clear(ClearType::All)
        )?;
        Ok(())
    }

    pub fn cleanup(&self) -> io::Result<()> {
        execute!(
            io::stdout(),
            cursor::Show,
            terminal::LeaveAlternateScreen
        )?;
        Ok(())
    }

    pub fn show_loading(&self, cassette_path: &str) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE PLAYER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(&title_full),
            ResetColor
        )?;

        // Status - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 3),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("🎵 LOADING CASSETTE"),
            ResetColor
        )?;
        
        // File info - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 5),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("File: {}", cassette_path)),
            ResetColor
        )?;

        // Animated loading indicator
        for i in 0..3 {
            execute!(stdout, cursor::MoveTo(left_margin as u16, 7))?;
            let dots = ".".repeat((i % 4) + 1);
            execute!(stdout, SetForegroundColor(colors::ACCENT_GREEN))?;
            print!("Loading{:4}", dots);
            execute!(stdout, ResetColor)?;
            io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        Ok(())
    }

    pub fn update_playback(&mut self, event_count: u64, current: u64, event: Option<&serde_json::Value>) -> io::Result<()> {
        self.total_events = event_count;
        self.current_event = current;
        self.event_counter.set(current);

        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE PLAYER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(&title_full),
            ResetColor
        )?;

        // Tape reels - centered like record UI
        let reel_frame = self.tape_reel.next_frame();
        let tape_length = 15;
        let reel_unit_width = 7 + tape_length + 7;
        let reel_x = (term_width.saturating_sub(reel_unit_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(reel_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("  ╱│╲  "),
            cursor::MoveTo((reel_x + 7 + tape_length) as u16, 3),
            Print("  ╱│╲  "),
            ResetColor
        )?;
        
        execute!(
            stdout,
            cursor::MoveTo(reel_x as u16, 4),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(" ╱ │ ╲ "),
            cursor::MoveTo((reel_x + 7 + tape_length) as u16, 4),
            Print(" ╱ │ ╲ "),
            ResetColor
        )?;
        
        execute!(
            stdout,
            cursor::MoveTo(reel_x as u16, 5),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(format!("│  {}  │", reel_frame)),
            cursor::MoveTo((reel_x + 7) as u16, 5),
            SetForegroundColor(Color::Rgb { r: 100, g: 80, b: 60 }),
            Print("═".repeat(tape_length)),
            cursor::MoveTo((reel_x + 7 + tape_length) as u16, 5),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(format!("│  {}  │", reel_frame)),
            ResetColor
        )?;

        // Counter - centered
        let counter_text = format!("▶ EVENTS: {}", self.event_counter.render(colors::ACCENT_GREEN));
        let counter_x = (term_width.saturating_sub(18)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 9),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("▶ EVENTS: "),
            Print(&self.event_counter.render(colors::ACCENT_GREEN)),
            ResetColor
        )?;

        // Progress bar - left aligned
        let progress = if event_count > 0 {
            current as f32 / event_count as f32
        } else {
            0.0
        };
        
        let bar_width = 30;
        let filled = (progress * bar_width as f32) as usize;
        let empty = bar_width - filled;

        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 11),
            Print("["),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("█".repeat(filled)),
            SetForegroundColor(colors::DARK_GRAY),
            Print("─".repeat(empty)),
            ResetColor,
            Print("] "),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("{}/{} events ({:.1}%)", current, event_count, progress * 100.0)),
            ResetColor
        )?;

        // Current event info - left aligned
        if let Some(event) = event {
            execute!(
                stdout,
                cursor::MoveTo(left_margin as u16, 13),
                SetForegroundColor(colors::OP1_ORANGE),
                Print("📝 CURRENT EVENT"),
                ResetColor
            )?;

            let mut y = 14;
            // Event details
            if let Some(kind) = event.get("kind").and_then(|k| k.as_u64()) {
                let kind_color = match kind {
                    0 => colors::OP1_BLUE,
                    1 => colors::ACCENT_GREEN,
                    7 => colors::OP1_RED,
                    30023 => colors::OP1_ORANGE,
                    _ => colors::MEDIUM_GRAY,
                };
                
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    SetForegroundColor(kind_color),
                    Print(format!("Kind: {}", kind)),
                    ResetColor
                )?;
                y += 1;
            }

            if let Some(id) = event.get("id").and_then(|i| i.as_str()) {
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    SetForegroundColor(colors::MEDIUM_GRAY),
                    Print(format!("ID: {}...", &id[..8])),
                    ResetColor
                )?;
                y += 1;
            }

            if let Some(content) = event.get("content").and_then(|c| c.as_str()) {
                let preview = if content.len() > 50 {
                    format!("{}...", &content[..47])
                } else {
                    content.to_string()
                };
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    SetForegroundColor(colors::MEDIUM_GRAY),
                    Print(format!("Content: {}", preview)),
                    ResetColor
                )?;
            }
        }

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 20),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("● PLAYING"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }

    pub fn show_completion(&self, total_events: u64) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE PLAYER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(&title_full),
            ResetColor
        )?;

        // Static tape reels - centered like record UI
        let tape_length = 15;
        let reel_unit_width = 7 + tape_length + 7;
        let reel_x = (term_width.saturating_sub(reel_unit_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(reel_x as u16, 3),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print("  ╱│╲  "),
            cursor::MoveTo((reel_x + 7 + tape_length) as u16, 3),
            Print("  ╱│╲  "),
            ResetColor
        )?;
        
        execute!(
            stdout,
            cursor::MoveTo(reel_x as u16, 5),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print("│  ●  │"),
            cursor::MoveTo((reel_x + 7) as u16, 5),
            SetForegroundColor(Color::Rgb { r: 100, g: 80, b: 60 }),
            Print("═".repeat(tape_length)),
            cursor::MoveTo((reel_x + 7 + tape_length) as u16, 5),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print("│  ●  │"),
            ResetColor
        )?;

        // Counter with checkmark - centered
        let counter_x = (term_width.saturating_sub(20)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 9),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("✓ PLAYBACK COMPLETE"),
            ResetColor
        )?;

        // Final stats - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 11),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Total events played: {}", total_events)),
            ResetColor
        )?;

        // Continue prompt - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 13),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("◉ Press any key to continue..."),
            ResetColor
        )?;

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 15),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("● COMPLETE"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }
}
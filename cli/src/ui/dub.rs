use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use super::{colors, components::Counter, animations::TapeReel};

pub struct DubUI {
    tape_reels: Vec<TapeReel>,
    event_counter: Counter,
    total_events: u64,
    current_cassette: usize,
    cassette_names: Vec<String>,
}

impl DubUI {
    pub fn new(cassette_count: usize, cassette_names: Vec<String>) -> Self {
        Self {
            tape_reels: (0..cassette_count).map(|_| TapeReel::new(6)).collect(),
            event_counter: Counter::new(6),
            total_events: 0,
            current_cassette: 0,
            cassette_names,
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

    pub fn show_loading(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE DUBBER";
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
            Print("🎚️ PREPARING TO DUB"),
            ResetColor
        )?;
        
        // Cassette list - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 5),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Loading {} cassettes...", self.cassette_names.len())),
            ResetColor
        )?;
        
        for (i, name) in self.cassette_names.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, (7 + i) as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(format!("[{}/{}] {}", i + 1, self.cassette_names.len(), name)),
                ResetColor
            )?;
        }

        io::stdout().flush()?;
        Ok(())
    }

    pub fn update_processing(&mut self, cassette_index: usize, events_processed: u64, total_events: u64) -> io::Result<()> {
        self.current_cassette = cassette_index;
        self.total_events = total_events;
        self.event_counter.set(events_processed);

        // Animate current cassette's reel
        if cassette_index < self.tape_reels.len() {
            self.tape_reels[cassette_index].next_frame();
        }

        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE DUBBER";
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

        // Mixing visualization - centered like tape reels
        let mixer_width = 21; // ┌─────┐ ─► ┌─────┐
        let mixer_x = (term_width.saturating_sub(mixer_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(mixer_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("┌─────┐ ─► ┌─────┐"),
            cursor::MoveTo(mixer_x as u16, 4),
            Print("│ SRC │    │ OUT │"),
            cursor::MoveTo(mixer_x as u16, 5),
            Print("└─────┘ ◄─ └─────┘"),
            ResetColor
        )?;

        // Counter - centered
        let counter_x = (term_width.saturating_sub(20)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 7),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("🎚️ EVENTS: "),
            Print(&self.event_counter.render(colors::OP1_ORANGE)),
            ResetColor
        )?;

        // Current cassette info - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 9),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("📊 DUBBING PROGRESS"),
            ResetColor
        )?;

        execute!(
            stdout,
            cursor::MoveTo((left_margin + 2) as u16, 10),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Current: [{}/{}] {}", 
                cassette_index + 1, 
                self.cassette_names.len(),
                self.cassette_names.get(cassette_index).unwrap_or(&"Unknown".to_string())
            )),
            ResetColor
        )?;

        execute!(
            stdout,
            cursor::MoveTo((left_margin + 2) as u16, 11),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Events: {} processed", events_processed)),
            ResetColor
        )?;

        // Source cassettes - left aligned, more compact
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 13),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print("SOURCE CASSETTES:"),
            ResetColor
        )?;

        // Show first few cassettes in compact format
        let max_display = 5; // Limit for landscape view
        let display_count = self.cassette_names.len().min(max_display);
        
        for (i, name) in self.cassette_names.iter().take(display_count).enumerate() {
            let reel_symbol = if i < self.tape_reels.len() {
                self.tape_reels[i].get_frame_symbol()
            } else {
                "◯".to_string()
            };

            if i == cassette_index {
                // Currently processing cassette
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (14 + i) as u16),
                    SetForegroundColor(colors::ACCENT_GREEN),
                    Print(format!("▶ {} {} {}", reel_symbol, reel_symbol, 
                        if name.len() > 25 { format!("{}...", &name[..22]) } else { name.clone() })),
                    ResetColor
                )?;
            } else if i < cassette_index {
                // Completed cassette
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (14 + i) as u16),
                    SetForegroundColor(colors::DARK_GRAY),
                    Print(format!("✓ ◯ ◯ {}", 
                        if name.len() > 25 { format!("{}...", &name[..22]) } else { name.clone() })),
                    ResetColor
                )?;
            } else {
                // Pending cassette
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (14 + i) as u16),
                    SetForegroundColor(colors::MEDIUM_GRAY),
                    Print(format!("  ◯ ◯ {}", 
                        if name.len() > 25 { format!("{}...", &name[..22]) } else { name.clone() })),
                    ResetColor
                )?;
            }
        }

        // Show "..." if there are more cassettes
        if self.cassette_names.len() > max_display {
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, (14 + display_count) as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(format!("  ... and {} more", self.cassette_names.len() - max_display)),
                ResetColor
            )?;
        }

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 22),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("● DUBBING"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }

    pub fn show_mixing(&self, total_events: u64) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE DUBBER";
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

        // Mixing visualization - centered
        let mixer_width = 19; // ┌─────┐ ──► ┌─────┐
        let mixer_x = (term_width.saturating_sub(mixer_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(mixer_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("┌─────┐ ──► ┌─────┐"),
            cursor::MoveTo(mixer_x as u16, 4),
            Print("│  ●  │     │  ●  │"),
            cursor::MoveTo(mixer_x as u16, 5),
            Print("└─────┘     └─────┘"),
            ResetColor
        )?;

        // Status - centered
        let status_x = (term_width.saturating_sub(18)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(status_x as u16, 7),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("🎛️ MIXING & FILTERING"),
            ResetColor
        )?;

        // Processing info - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 9),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Processing {} events...", total_events)),
            cursor::MoveTo(left_margin as u16, 10),
            Print("Applying filters and deduplication..."),
            ResetColor
        )?;

        // Animated processing dots
        for i in 0..3 {
            execute!(stdout, cursor::MoveTo(left_margin as u16, 12))?;
            let dots = ".".repeat((i % 4) + 1);
            execute!(stdout, SetForegroundColor(colors::ACCENT_GREEN))?;
            print!("Mixing{:4}", dots);
            execute!(stdout, ResetColor)?;
            io::stdout().flush()?;
            std::thread::sleep(std::time::Duration::from_millis(300));
        }

        Ok(())
    }

    pub fn show_completion(&self, output_path: &str, final_event_count: u64) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE DUBBER";
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

        // Final output cassette - centered
        let cassette_width = 15; // ┌─────────────┐
        let cassette_x = (term_width.saturating_sub(cassette_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(cassette_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("┌─────────────┐"),
            cursor::MoveTo(cassette_x as u16, 4),
            Print("│    ◉ ◉      │"),
            cursor::MoveTo(cassette_x as u16, 5),
            Print("│  ▓▓▓▓▓▓▓▓   │"),
            cursor::MoveTo(cassette_x as u16, 6),
            Print("└─────────────┘"),
            ResetColor
        )?;

        // Completion status - centered
        let status_x = (term_width.saturating_sub(17)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(status_x as u16, 8),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("✓ DUBBING COMPLETE"),
            ResetColor
        )?;

        // Final stats - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 10),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("📦 OUTPUT DETAILS"),
            ResetColor
        )?;

        execute!(
            stdout,
            cursor::MoveTo((left_margin + 2) as u16, 11),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("File: {}", output_path)),
            cursor::MoveTo((left_margin + 2) as u16, 12),
            Print(format!("Events: {}", final_event_count)),
            cursor::MoveTo((left_margin + 2) as u16, 13),
            Print(format!("Sources: {} cassettes", self.cassette_names.len())),
            ResetColor
        )?;

        // Continue prompt - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 15),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("◉ Press any key to continue..."),
            ResetColor
        )?;

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 17),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("● COMPLETE"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }
}
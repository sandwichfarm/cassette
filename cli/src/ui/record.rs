use super::animations::{TapeReel, VUMeter};
use super::components::{Counter, ProgressBar, EventTypeIndicator};
use super::colors;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor, Stylize},
    cursor,
    terminal::{self, ClearType},
};
use std::io::{self, stdout};
use std::collections::HashMap;

pub struct RecordUI {
    tape_reel_left: TapeReel,
    tape_reel_right: TapeReel,
    event_counter: Counter,
    progress_bar: ProgressBar,
    vu_meter: VUMeter,
    event_types: HashMap<u64, u64>,
    start_time: std::time::Instant,
}

impl RecordUI {
    pub fn new() -> Self {
        Self {
            tape_reel_left: TapeReel::new(7),
            tape_reel_right: TapeReel::new(7),
            event_counter: Counter::new(6),
            progress_bar: ProgressBar::new(30, "Recording".to_string()),
            vu_meter: VUMeter::new(2, 20),
            event_types: HashMap::new(),
            start_time: std::time::Instant::now(),
        }
    }

    pub fn init(&self) -> io::Result<()> {
        execute!(
            stdout(),
            terminal::Clear(ClearType::All),
            cursor::Hide,
            cursor::MoveTo(0, 0)
        )?;
        Ok(())
    }

    pub fn cleanup(&self) -> io::Result<()> {
        execute!(
            stdout(),
            cursor::Show,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;
        Ok(())
    }

    pub fn update(&mut self, event_count: u64, event_kind: Option<u64>) -> io::Result<()> {
        // Update counters
        self.event_counter.set(event_count);
        
        // Update event types
        if let Some(kind) = event_kind {
            *self.event_types.entry(kind).or_insert(0) += 1;
        }
        
        // Update animations
        self.tape_reel_left.next_frame();
        self.tape_reel_right.next_frame();
        
        // Update VU meter with simulated activity
        let activity = (event_count as f32 % 10.0) / 10.0;
        self.vu_meter.update(0, activity * 0.8);
        self.vu_meter.update(1, activity * 0.6);
        
        self.render()?;
        Ok(())
    }

    fn render(&self) -> io::Result<()> {
        let mut stdout = stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60; // Standard width for our content
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        // Clear screen and reset cursor
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        // Title - centered
        let title = "CASSETTE RECORDER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_RED),
            Print(&title_full),
            ResetColor
        )?;

        // Tape reels - centered as a unit
        let left_reel = self.tape_reel_left.render(true);
        let right_reel = self.tape_reel_right.render(false);
        let tape_length = 15;
        let reel_unit_width = 7 + tape_length + 7; // left reel + tape + right reel
        let reel_x = (term_width.saturating_sub(reel_unit_width)) / 2;
        
        for (i, (left_line, right_line)) in left_reel.iter().zip(right_reel.iter()).enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(reel_x as u16, 3 + i as u16),
                Print(left_line),
                cursor::MoveTo((reel_x + 7 + tape_length) as u16, 3 + i as u16),
                Print(right_line)
            )?;
        }

        // Tape connection
        execute!(
            stdout,
            cursor::MoveTo((reel_x + 7) as u16, 5),
            SetForegroundColor(Color::Rgb { r: 100, g: 80, b: 60 }),
            Print("═".repeat(tape_length)),
            ResetColor
        )?;

        // Counter - centered
        let counter_text = format!("EVENTS: {}", self.event_counter.render(colors::ACCENT_GREEN));
        let counter_x = (term_width.saturating_sub(16)) / 2; // Approximate width
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 9),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("EVENTS: "),
            Print(&self.event_counter.render(colors::ACCENT_GREEN)),
            ResetColor
        )?;

        // VU Meter - centered
        let vu_lines = self.vu_meter.render();
        let vu_x = left_margin as u16;
        
        for (i, line) in vu_lines.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(vu_x, 11 + i as u16),
                Print(line)
            )?;
        }

        // Event type breakdown - left aligned with margin
        if !self.event_types.is_empty() {
            execute!(
                stdout,
                cursor::MoveTo(left_margin as u16, 14),
                SetForegroundColor(colors::FOREGROUND),
                Print("Event Types:"),
                ResetColor
            )?;
            
            let mut y = 15;
            let total = self.event_types.values().sum::<u64>();
            
            for (kind, count) in self.event_types.iter() {
                let indicator = EventTypeIndicator::render(*kind);
                let percentage = (*count as f32 / total as f32 * 100.0) as u16;
                
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    Print(&indicator),
                    cursor::MoveTo((left_margin + 10) as u16, y),
                    Print(format!("{:>4} ({:>3}%)", count, percentage))
                )?;
                
                y += 1;
                if y > 20 {
                    break;
                }
            }
        }

        // Recording time - left aligned with margin
        let elapsed = self.start_time.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 22),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print(format!("Recording Time: {:02}:{:02}", minutes, seconds)),
            ResetColor
        )?;

        // Status - left aligned with margin
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 24),
            SetForegroundColor(colors::ACCENT_RED),
            Print("● REC"),
            ResetColor
        )?;

        Ok(())
    }

    pub fn show_completion(&self, total_events: u64, output_path: &str) -> io::Result<()> {
        let mut stdout = stdout();
        let elapsed = self.start_time.elapsed();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        // Title - same as recording state
        let title = "CASSETTE RECORDER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_RED),
            Print(&title_full),
            ResetColor
        )?;

        // Tape reels - stopped (no animation)
        self.draw_stopped_tape_reels(term_width)?;

        // Counter with checkmark - centered
        let counter_x = (term_width.saturating_sub(18)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 9),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("✓ EVENTS: "),
            Print(&Counter::new(6).render_value(total_events, colors::ACCENT_GREEN)),
            ResetColor
        )?;

        // VU Meter - at rest, left aligned with margin
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 11),
            Print("CH1  "),
            SetForegroundColor(colors::DARK_GRAY),
            Print("░".repeat(20)),
            ResetColor,
            cursor::MoveTo(left_margin as u16, 12),
            Print("CH2  "),
            SetForegroundColor(colors::DARK_GRAY),
            Print("░".repeat(20)),
            ResetColor
        )?;

        // Event type breakdown - left aligned with margin
        if !self.event_types.is_empty() {
            execute!(
                stdout,
                cursor::MoveTo(left_margin as u16, 14),
                SetForegroundColor(colors::FOREGROUND),
                Print("Event Types:"),
                ResetColor
            )?;
            
            let mut y = 15;
            
            for (kind, count) in self.event_types.iter() {
                let indicator = EventTypeIndicator::render(*kind);
                let percentage = (*count as f32 / total_events as f32 * 100.0) as u16;
                
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    Print(&indicator),
                    cursor::MoveTo((left_margin + 10) as u16, y),
                    Print(format!("{:>4} ({:>3}%)", count, percentage))
                )?;
                
                y += 1;
                if y > 20 {
                    break;
                }
            }
        }

        // Recording time and output - left aligned with margin
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 22),
            SetForegroundColor(colors::FOREGROUND),
            Print(format!("Recording Time: {:02}:{:02}", minutes, seconds)),
            ResetColor,
            cursor::MoveTo(left_margin as u16, 23),
            SetForegroundColor(colors::ACCENT_BLUE),
            Print(format!("Output: {}", output_path)),
            ResetColor
        )?;

        // Status - COMPLETE instead of REC
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 24),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("● COMPLETE"),
            ResetColor
        )?;

        Ok(())
    }

    fn draw_stopped_tape_reels(&self, term_width: usize) -> io::Result<()> {
        let mut stdout = stdout();
        
        // Static tape reels - no rotation
        let reel = vec![
            "  ╱│╲  ",
            " ╱ │ ╲ ",
            "│  ●  │",
            " ╲ │ ╱ ",
            "  ╲│╱  "
        ];
        
        let tape_length = 15;
        let reel_unit_width = 7 + tape_length + 7;
        let reel_x = (term_width.saturating_sub(reel_unit_width)) / 2;
        
        // Draw both reels with tape
        for (i, line) in reel.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(reel_x as u16, 3 + i as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(line),
                ResetColor
            )?;
            
            // Tape connection on middle line
            if i == 2 {
                execute!(
                    stdout,
                    cursor::MoveTo((reel_x + 7) as u16, 5),
                    SetForegroundColor(Color::Rgb { r: 100, g: 80, b: 60 }),
                    Print("═".repeat(tape_length)),
                    ResetColor
                )?;
            }
            
            execute!(
                stdout,
                cursor::MoveTo((reel_x + 7 + tape_length) as u16, 3 + i as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(line),
                ResetColor
            )?;
        }
        
        Ok(())
    }

    pub fn show_compilation(&self, total_events: u64) -> io::Result<()> {
        let mut stdout = stdout();
        let elapsed = self.start_time.elapsed();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        // Title - same as recording state
        let title = "CASSETTE RECORDER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        let title_x = (term_width.saturating_sub(title_full.len())) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(title_x as u16, 1),
            SetForegroundColor(colors::ACCENT_RED),
            Print(&title_full),
            ResetColor
        )?;

        // Tape reels - stopped during compilation
        self.draw_stopped_tape_reels(term_width)?;

        // Counter with compilation symbol
        let counter_x = (term_width.saturating_sub(20)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 9),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print("⚙ EVENTS: "),
            Print(&Counter::new(6).render_value(total_events, colors::ACCENT_YELLOW)),
            ResetColor
        )?;

        // Compilation progress indicator
        let comp_time = elapsed.as_millis() % 3000; // 3 second cycle
        let dots = match comp_time / 500 {
            0 => "   ",
            1 => "●  ",
            2 => "●● ",
            3 => "●●●",
            4 => " ●●",
            _ => "  ●",
        };
        
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 11),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print(format!("COMPILING WASM {}", dots)),
            ResetColor
        )?;

        // Progress bar animation
        let progress = (elapsed.as_millis() % 2000) as f32 / 2000.0;
        let bar_width = 30;
        let filled = (progress * bar_width as f32) as usize;
        let empty = bar_width - filled;
        
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 12),
            Print("["),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print("▓".repeat(filled)),
            SetForegroundColor(colors::DARK_GRAY),
            Print("░".repeat(empty)),
            ResetColor,
            Print("]")
        )?;

        // Event type breakdown - same as completion
        if !self.event_types.is_empty() {
            execute!(
                stdout,
                cursor::MoveTo(left_margin as u16, 14),
                SetForegroundColor(colors::FOREGROUND),
                Print("Event Types:"),
                ResetColor
            )?;
            
            let mut y = 15;
            
            for (kind, count) in self.event_types.iter() {
                let indicator = EventTypeIndicator::render(*kind);
                let percentage = (*count as f32 / total_events as f32 * 100.0) as u16;
                
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, y),
                    Print(&indicator),
                    cursor::MoveTo((left_margin + 10) as u16, y),
                    Print(format!("{:>4} ({:>3}%)", count, percentage))
                )?;
                
                y += 1;
                if y > 20 {
                    break;
                }
            }
        }

        // Recording time
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 22),
            SetForegroundColor(colors::FOREGROUND),
            Print(format!("Recording Time: {:02}:{:02}", minutes, seconds)),
            ResetColor
        )?;

        // Status - COMPILING
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 24),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print("⚙ COMPILING"),
            ResetColor
        )?;

        Ok(())
    }
}
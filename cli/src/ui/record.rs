use super::animations::{TapeReel, VUMeter};
use super::components::{Counter, ProgressBar, EventTypeIndicator};
use super::colors;
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
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
        
        // Clear screen and reset cursor
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        // Title
        execute!(
            stdout,
            cursor::MoveTo(20, 1),
            SetForegroundColor(colors::ACCENT_RED),
            Print("▓▓▓ CASSETTE RECORDER ▓▓▓"),
            ResetColor
        )?;

        // Tape reels
        let left_reel = self.tape_reel_left.render(true);
        let right_reel = self.tape_reel_right.render(false);
        
        for (i, (left_line, right_line)) in left_reel.iter().zip(right_reel.iter()).enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(10, 3 + i as u16),
                Print(left_line),
                cursor::MoveTo(30, 3 + i as u16),
                Print(right_line)
            )?;
        }

        // Tape connection
        execute!(
            stdout,
            cursor::MoveTo(17, 5),
            SetForegroundColor(Color::Rgb { r: 100, g: 80, b: 60 }),
            Print("═══════════════"),
            ResetColor
        )?;

        // Counter
        execute!(
            stdout,
            cursor::MoveTo(20, 9),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("EVENTS: "),
            Print(&self.event_counter.render(colors::ACCENT_GREEN)),
            ResetColor
        )?;

        // VU Meter
        let vu_lines = self.vu_meter.render();
        for (i, line) in vu_lines.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(10, 11 + i as u16),
                Print(line)
            )?;
        }

        // Event type breakdown
        if !self.event_types.is_empty() {
            execute!(
                stdout,
                cursor::MoveTo(10, 14),
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
                    cursor::MoveTo(12, y),
                    Print(&indicator),
                    cursor::MoveTo(20, y),
                    Print(format!("{:>4} ({:>3}%)", count, percentage))
                )?;
                
                y += 1;
                if y > 20 {
                    break;
                }
            }
        }

        // Recording time
        let elapsed = self.start_time.elapsed();
        let minutes = elapsed.as_secs() / 60;
        let seconds = elapsed.as_secs() % 60;
        
        execute!(
            stdout,
            cursor::MoveTo(10, 22),
            SetForegroundColor(colors::ACCENT_YELLOW),
            Print(format!("Recording Time: {:02}:{:02}", minutes, seconds)),
            ResetColor
        )?;

        // Status
        execute!(
            stdout,
            cursor::MoveTo(10, 24),
            SetForegroundColor(colors::ACCENT_RED),
            Print("● REC"),
            ResetColor
        )?;

        Ok(())
    }

    pub fn show_completion(&self, total_events: u64, output_path: &str) -> io::Result<()> {
        let mut stdout = stdout();
        
        execute!(
            stdout,
            cursor::MoveTo(0, 0),
            terminal::Clear(ClearType::All)
        )?;

        // Success message
        execute!(
            stdout,
            cursor::MoveTo(15, 10),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("✓ RECORDING COMPLETE"),
            ResetColor
        )?;

        execute!(
            stdout,
            cursor::MoveTo(10, 12),
            SetForegroundColor(colors::FOREGROUND),
            Print(format!("Total Events: {}", total_events)),
            cursor::MoveTo(10, 13),
            Print(format!("Output: {}", output_path)),
            ResetColor
        )?;

        // Cassette art
        let cassette = super::components::CassetteArt::render();
        for (i, line) in cassette.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo(10, 15 + i as u16),
                Print(line)
            )?;
        }

        Ok(())
    }
}
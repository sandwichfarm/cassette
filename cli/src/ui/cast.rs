use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use super::{colors, components::Counter};

pub struct CastUI {
    event_counter: Counter,
    relay_count: usize,
    cassette_count: usize,
    current_relay: usize,
    relay_urls: Vec<String>,
    cassette_names: Vec<String>,
    connected_relays: Vec<bool>,
    sent_events: u64,
    total_events: u64,
}

impl CastUI {
    pub fn new(relay_urls: Vec<String>, cassette_names: Vec<String>) -> Self {
        let relay_count = relay_urls.len();
        let cassette_count = cassette_names.len();
        
        Self {
            event_counter: Counter::new(6),
            relay_count,
            cassette_count,
            current_relay: 0,
            relay_urls,
            cassette_names,
            connected_relays: vec![false; relay_count],
            sent_events: 0,
            total_events: 0,
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

    pub fn show_connecting(&self) -> io::Result<()> {
        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE CASTER";
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
            Print("📡 CONNECTING TO RELAYS"),
            ResetColor
        )?;
        
        // Cassettes info - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 5),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Cassettes to broadcast: {}", self.cassette_count)),
            ResetColor
        )?;
        
        for (i, name) in self.cassette_names.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, (6 + i) as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(format!("📼 {}", name)),
                ResetColor
            )?;
        }
        
        let relay_start_y = 6 + self.cassette_names.len() + 1;
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, relay_start_y as u16),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Target relays: {}", self.relay_count)),
            ResetColor
        )?;
        
        for (i, url) in self.relay_urls.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, (relay_start_y + 1 + i) as u16),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(format!("🌐 {}", url)),
                ResetColor
            )?;
        }

        io::stdout().flush()?;
        Ok(())
    }

    pub fn update_connection(&mut self, relay_index: usize, connected: bool) -> io::Result<()> {
        if relay_index < self.connected_relays.len() {
            self.connected_relays[relay_index] = connected;
        }

        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE CASTER";
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

        // Broadcasting visualization - centered
        let broadcast_width = 19; // 📼 ─► 📡 ─► 🌐
        let broadcast_x = (term_width.saturating_sub(broadcast_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(broadcast_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("📼 ─► 📡 ─► 🌐"),
            cursor::MoveTo(broadcast_x as u16, 4),
            Print("CASSETTE CAST RELAYS"),
            ResetColor
        )?;

        // Status - centered
        let status_x = (term_width.saturating_sub(23)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(status_x as u16, 6),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("📡 ESTABLISHING CONNECTIONS"),
            ResetColor
        )?;

        // Relay connection status - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 8),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print("RELAY STATUS:"),
            ResetColor
        )?;

        for (i, (url, &relay_connected)) in self.relay_urls.iter().zip(self.connected_relays.iter()).enumerate() {
            let status_symbol = if relay_connected {
                "🟢"
            } else if i <= relay_index {
                "🟡"
            } else {
                "⚪"
            };
            
            let status_text = if relay_connected {
                "CONNECTED"
            } else if i <= relay_index {
                "CONNECTING..."
            } else {
                "PENDING"
            };

            let color = if relay_connected {
                colors::ACCENT_GREEN
            } else if i <= relay_index {
                colors::OP1_ORANGE
            } else {
                colors::MEDIUM_GRAY
            };

            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, (9 + i) as u16),
                SetForegroundColor(color),
                Print(format!("{} {} {}", status_symbol, status_text, 
                    if url.len() > 35 { format!("{}...", &url[..32]) } else { url.clone() })),
                ResetColor
            )?;
        }

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, (12 + self.relay_urls.len()) as u16),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("● CONNECTING"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }

    pub fn update_casting(&mut self, events_sent: u64, total_events: u64, current_relay: usize) -> io::Result<()> {
        self.sent_events = events_sent;
        self.total_events = total_events;
        self.current_relay = current_relay;
        self.event_counter.set(events_sent);

        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE CASTER";
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

        // Broadcasting animation - centered
        let broadcast_width = 19; // 📼 ═══► 📡 ═══► 🌐
        let broadcast_x = (term_width.saturating_sub(broadcast_width)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(broadcast_x as u16, 3),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("📼 ═══► 📡 ═══► 🌐"),
            cursor::MoveTo(broadcast_x as u16, 4),
            Print("CASSETTE CAST RELAYS"),
            ResetColor
        )?;

        // Counter - centered
        let counter_x = (term_width.saturating_sub(20)) / 2;
        
        execute!(
            stdout,
            cursor::MoveTo(counter_x as u16, 6),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("📡 EVENTS: "),
            Print(&self.event_counter.render(colors::OP1_ORANGE)),
            ResetColor
        )?;

        // Progress bar - left aligned
        let progress = if total_events > 0 {
            events_sent as f32 / total_events as f32
        } else {
            0.0
        };

        let bar_width = 30;
        let filled = (progress * bar_width as f32) as usize;
        let empty = bar_width - filled;

        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 8),
            Print("["),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("█".repeat(filled)),
            SetForegroundColor(colors::DARK_GRAY),
            Print("─".repeat(empty)),
            ResetColor,
            Print("] "),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("{}/{} events ({:.1}%)", events_sent, total_events, progress * 100.0)),
            ResetColor
        )?;

        // Relay status - left aligned
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 10),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("🌐 RELAY STATUS"),
            ResetColor
        )?;

        for (i, (url, &connected)) in self.relay_urls.iter().zip(self.connected_relays.iter()).enumerate() {
            if i == current_relay {
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (11 + i) as u16),
                    SetForegroundColor(colors::ACCENT_GREEN),
                    Print(format!("▶ 🟢 {} (ACTIVE)", 
                        if url.len() > 30 { format!("{}...", &url[..27]) } else { url.clone() })),
                    ResetColor
                )?;
            } else if connected {
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (11 + i) as u16),
                    SetForegroundColor(colors::DARK_GRAY),
                    Print(format!("  🟢 {}", 
                        if url.len() > 30 { format!("{}...", &url[..27]) } else { url.clone() })),
                    ResetColor
                )?;
            } else {
                execute!(
                    stdout,
                    cursor::MoveTo((left_margin + 2) as u16, (11 + i) as u16),
                    SetForegroundColor(colors::MEDIUM_GRAY),
                    Print(format!("  🔴 {} (DISCONNECTED)", 
                        if url.len() > 20 { format!("{}...", &url[..17]) } else { url.clone() })),
                    ResetColor
                )?;
            }
        }

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, (14 + self.relay_urls.len()) as u16),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print("● BROADCASTING"),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }

    pub fn show_completion(&self, successful_relays: usize, total_sent: u64) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Header
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_GREEN))?;
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│                              ◉ CASSETTE CASTER ◉                           │");
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Completion status
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_GREEN))?;
        println!("  ✓ BROADCAST COMPLETE");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Success visualization
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_GREEN))?;
        println!("     📼 ═══► 📡 ═══► 🌐 ✓");
        println!("   CASSETTE   CAST   RELAYS");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Final stats
        execute!(io::stdout(), SetForegroundColor(colors::OP1_ORANGE))?;
        println!("  📊 BROADCAST SUMMARY");
        execute!(io::stdout(), ResetColor)?;

        execute!(io::stdout(), SetForegroundColor(colors::MEDIUM_GRAY))?;
        println!("     Events sent: {}", total_sent);
        println!("     Successful relays: {}/{}", successful_relays, self.relay_count);
        println!("     Cassettes processed: {}", self.cassette_count);
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Final relay status
        execute!(io::stdout(), SetForegroundColor(colors::OP1_ORANGE))?;
        println!("  🌐 FINAL RELAY STATUS");
        execute!(io::stdout(), ResetColor)?;

        for (url, &connected) in self.relay_urls.iter().zip(self.connected_relays.iter()) {
            if connected {
                execute!(io::stdout(), SetForegroundColor(colors::ACCENT_GREEN))?;
                println!("    ✓ {} SUCCESS", url);
                execute!(io::stdout(), ResetColor)?;
            } else {
                execute!(io::stdout(), SetForegroundColor(colors::ACCENT_RED))?;
                println!("    ✗ {} FAILED", url);
                execute!(io::stdout(), ResetColor)?;
            }
        }

        println!();
        execute!(io::stdout(), SetForegroundColor(colors::OP1_ORANGE))?;
        println!("  ◉ Press any key to continue...");
        execute!(io::stdout(), ResetColor)?;

        io::stdout().flush()?;
        Ok(())
    }

    pub fn show_error(&self, error_msg: &str) -> io::Result<()> {
        execute!(io::stdout(), cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Header
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_GREEN))?;
        println!("┌─────────────────────────────────────────────────────────────────────────────┐");
        println!("│                              ◉ CASSETTE CASTER ◉                           │");
        println!("└─────────────────────────────────────────────────────────────────────────────┘");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Error status
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_RED))?;
        println!("  ✗ BROADCAST FAILED");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Error visualization
        execute!(io::stdout(), SetForegroundColor(colors::ACCENT_RED))?;
        println!("     📼 ───► ✗ ───► 🌐");
        println!("   CASSETTE ERROR  RELAYS");
        execute!(io::stdout(), ResetColor)?;

        println!();

        // Error details
        execute!(io::stdout(), SetForegroundColor(colors::OP1_ORANGE))?;
        println!("  ⚠ ERROR DETAILS");
        execute!(io::stdout(), ResetColor)?;

        execute!(io::stdout(), SetForegroundColor(colors::MEDIUM_GRAY))?;
        println!("     {}", error_msg);
        execute!(io::stdout(), ResetColor)?;

        println!();
        execute!(io::stdout(), SetForegroundColor(colors::OP1_ORANGE))?;
        println!("  ◉ Press any key to continue...");
        execute!(io::stdout(), ResetColor)?;

        io::stdout().flush()?;
        Ok(())
    }
}
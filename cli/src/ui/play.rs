use crossterm::{
    cursor,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};
use std::collections::{HashMap, HashSet};
use super::{colors, components::Counter, animations::TapeReel};

pub struct PlayUI {
    tape_reel: TapeReel,
    event_counter: Counter,
    total_events: u64,
    current_event: u64,
    event_kinds: HashMap<u64, u64>,
    unique_pubkeys: HashSet<String>,
    min_timestamp: Option<u64>,
    max_timestamp: Option<u64>,
}

impl PlayUI {
    pub fn new() -> Self {
        Self {
            tape_reel: TapeReel::new(8),
            event_counter: Counter::new(0),
            total_events: 0,
            current_event: 0,
            event_kinds: HashMap::new(),
            unique_pubkeys: HashSet::new(),
            min_timestamp: None,
            max_timestamp: None,
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
        // Each █ character is 2 columns wide, so we need to calculate visual width
        let visual_width = 8 + 1 + title.len() + 1 + 8; // 8 for each border (4 chars * 2 cols)
        let title_x = (term_width.saturating_sub(visual_width)) / 2;
        
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
        
        // Collect stats from event
        if let Some(e) = event {
            // Track event kinds
            if let Some(kind) = e.get("kind").and_then(|k| k.as_u64()) {
                *self.event_kinds.entry(kind).or_insert(0) += 1;
            }
            
            // Track unique pubkeys
            if let Some(pubkey) = e.get("pubkey").and_then(|p| p.as_str()) {
                self.unique_pubkeys.insert(pubkey.to_string());
            }
            
            // Track timestamps
            if let Some(created_at) = e.get("created_at").and_then(|t| t.as_u64()) {
                match self.min_timestamp {
                    None => self.min_timestamp = Some(created_at),
                    Some(min) if created_at < min => self.min_timestamp = Some(created_at),
                    _ => {}
                }
                match self.max_timestamp {
                    None => self.max_timestamp = Some(created_at),
                    Some(max) if created_at > max => self.max_timestamp = Some(created_at),
                    _ => {}
                }
            }
        }

        let mut stdout = io::stdout();
        let term_width = terminal::size()?.0 as usize;
        let content_width = 60;
        let left_margin = (term_width.saturating_sub(content_width)) / 2;
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE PLAYER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        // Each █ character is 2 columns wide, so we need to calculate visual width
        let visual_width = 8 + 1 + title.len() + 1 + 8; // 8 for each border (4 chars * 2 cols)
        let title_x = (term_width.saturating_sub(visual_width)) / 2;
        
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
        let _counter_text = format!("▶ EVENTS: {}", self.event_counter.render(colors::ACCENT_GREEN));
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
                let preview = if content.chars().count() > 50 {
                    let truncated: String = content.chars().take(47).collect();
                    format!("{}...", truncated)
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

        // Stats display - left aligned
        let stats_y = 18;
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, stats_y),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("📊 LIVE STATS"),
            ResetColor
        )?;
        
        // Total events
        execute!(
            stdout,
            cursor::MoveTo((left_margin + 2) as u16, stats_y + 1),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Total Events: {}", self.total_events)),
            ResetColor
        )?;
        
        // Unique authors
        execute!(
            stdout,
            cursor::MoveTo((left_margin + 2) as u16, stats_y + 2),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(format!("Unique Authors: {}", self.unique_pubkeys.len())),
            ResetColor
        )?;
        
        // Date range (if we have timestamps)
        if let (Some(min), Some(max)) = (self.min_timestamp, self.max_timestamp) {
            use chrono::{DateTime, Utc};
            let min_date = DateTime::<Utc>::from_timestamp(min as i64, 0)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let max_date = DateTime::<Utc>::from_timestamp(max as i64, 0)
                .map(|dt| dt.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, stats_y + 3),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(format!("Date Range: {} to {}", min_date, max_date)),
                ResetColor
            )?;
        }
        
        // Top event kinds (show top 3)
        if !self.event_kinds.is_empty() {
            let mut kinds_vec: Vec<_> = self.event_kinds.iter().collect();
            kinds_vec.sort_by(|a, b| b.1.cmp(a.1));
            
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 2) as u16, stats_y + 4),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print("Event Kinds: "),
                ResetColor
            )?;
            
            let top_3: Vec<_> = kinds_vec.iter().take(3).collect();
            let mut kind_strs = Vec::new();
            for (kind, count) in top_3 {
                let icon = match **kind {
                    0 => "👤",
                    1 => "📝",
                    3 => "📇",
                    7 => "❤️",
                    _ => "○",
                };
                kind_strs.push(format!("{} {} ({})", icon, kind, count));
            }
            
            execute!(
                stdout,
                cursor::MoveTo((left_margin + 15) as u16, stats_y + 4),
                SetForegroundColor(colors::DARK_GRAY),
                Print(kind_strs.join(", ")),
                ResetColor
            )?;
        }

        // Status - left aligned at bottom
        execute!(
            stdout,
            cursor::MoveTo(left_margin as u16, 24),
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
        
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

        // Title - centered like record UI
        let title = "CASSETTE PLAYER";
        let border = "████";
        let title_full = format!("{} {} {}", border, title, border);
        // Each █ character is 2 columns wide, so we need to calculate visual width
        let visual_width = 8 + 1 + title.len() + 1 + 8; // 8 for each border (4 chars * 2 cols)
        let title_x = (term_width.saturating_sub(visual_width)) / 2;
        
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

        // Stats display - centered
        let mut y = 11;
        
        // Total events
        let events_text = format!("Total Events: {}", total_events);
        let events_x = (term_width.saturating_sub(events_text.len())) / 2;
        execute!(
            stdout,
            cursor::MoveTo(events_x as u16, y),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(&events_text),
            ResetColor
        )?;
        y += 1;
        
        // Unique authors
        let authors_text = format!("Unique Authors: {}", self.unique_pubkeys.len());
        let authors_x = (term_width.saturating_sub(authors_text.len())) / 2;
        execute!(
            stdout,
            cursor::MoveTo(authors_x as u16, y),
            SetForegroundColor(colors::MEDIUM_GRAY),
            Print(&authors_text),
            ResetColor
        )?;
        y += 1;
        
        // Date range
        if let (Some(min), Some(max)) = (self.min_timestamp, self.max_timestamp) {
            use chrono::{DateTime, Utc};
            let min_date = DateTime::<Utc>::from_timestamp(min as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            let max_date = DateTime::<Utc>::from_timestamp(max as i64, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            
            let date_text = format!("Date Range: {} to {}", min_date, max_date);
            let date_x = (term_width.saturating_sub(date_text.len())) / 2;
            execute!(
                stdout,
                cursor::MoveTo(date_x as u16, y),
                SetForegroundColor(colors::MEDIUM_GRAY),
                Print(&date_text),
                ResetColor
            )?;
            y += 1;
        }
        y += 1; // Extra spacing
        
        // Event kinds - top 5
        if !self.event_kinds.is_empty() {
            let kinds_title = "Event Kinds:";
            let kinds_x = (term_width.saturating_sub(20)) / 2; // Approximate center for kinds display
            execute!(
                stdout,
                cursor::MoveTo(kinds_x as u16, y),
                SetForegroundColor(colors::FOREGROUND),
                Print(kinds_title),
                ResetColor
            )?;
            y += 1;
            
            // Sort kinds by count
            let mut kinds_vec: Vec<_> = self.event_kinds.iter().collect();
            kinds_vec.sort_by(|a, b| b.1.cmp(a.1));
            
            // Show top 5
            let top_5: Vec<_> = kinds_vec.iter().take(5).collect();
            let remaining = kinds_vec.len().saturating_sub(5);
            
            for (kind, count) in top_5 {
                let kind_name = match **kind {
                    0 => "Metadata",
                    1 => "Text Note",
                    3 => "Contact List",
                    4 => "DM",
                    5 => "Delete",
                    6 => "Repost",
                    7 => "Reaction",
                    8 => "Badge Award",
                    16 => "Generic Repost",
                    40 => "Channel",
                    41 => "Channel Message",
                    42 => "Channel Hide",
                    43 => "Channel Mute",
                    44 => "Channel Mute User",
                    1984 => "Report",
                    9735 => "Zap",
                    10000 => "Mute List",
                    10001 => "Pin List",
                    10002 => "Relay List",
                    10003 => "Bookmark List",
                    10004 => "Communities List",
                    10005 => "Public Chats List",
                    10006 => "Blocked Relays List",
                    10007 => "Search Relays List",
                    10009 => "User Groups",
                    10015 => "Interests List",
                    10030 => "User Emoji List",
                    10050 => "Relay List Metadata",
                    10063 => "User Server List",
                    10096 => "File Storage Server List",
                    13194 => "Wallet Info",
                    21000 => "Lightning Pub RPC",
                    22242 => "Client Authentication",
                    23194 => "Wallet Request",
                    23195 => "Wallet Response",
                    24133 => "Nostr Connect",
                    27235 => "HTTP Auth",
                    30000 => "Follow Sets",
                    30001 => "Generic Lists",
                    30002 => "Relay Sets",
                    30003 => "Bookmark Sets",
                    30004 => "Curation Sets",
                    30005 => "Video Sets",
                    30006 => "Kind Mute Sets",
                    30007 => "Article Curation Sets",
                    30008 => "Profile Badges",
                    30009 => "Badge Definition",
                    30015 => "Interest Sets",
                    30017 => "Create/Update Stall",
                    30018 => "Create/Update Product",
                    30019 => "Marketplace UI/UX",
                    30020 => "Product Sold as Auction",
                    30023 => "Long-form Content",
                    30024 => "Draft Long-form Content",
                    30030 => "Emoji Sets",
                    30040 => "Modular Article Header",
                    30041 => "Modular Article Content",
                    30063 => "Release Artifact Sets",
                    30078 => "Application-specific Data",
                    30311 => "Live Event",
                    30315 => "User Statuses",
                    30402 => "Classified Listing",
                    30403 => "Draft Classified Listing",
                    30617 => "Repository Announcement",
                    30618 => "Repository State Announcement",
                    30819 => "Redirects",
                    31337 => "Audio Track",
                    31922 => "Date-Based Calendar Event",
                    31923 => "Time-Based Calendar Event",
                    31924 => "Calendar",
                    31925 => "Calendar Event RSVP",
                    31989 => "Handler Recommendation",
                    31990 => "Handler Information",
                    34235 => "Video Event",
                    34236 => "Short-form Portrait Video Event",
                    34237 => "Video View Event",
                    34550 => "Community Definition",
                    39000..=39009 => "Group Metadata Events",
                    _ => "Unknown",
                };
                
                let kind_line = format!("  {} kind {} - {} events", 
                    if **kind == 1 { "📝" } 
                    else if **kind == 7 { "❤️" } 
                    else if **kind == 0 { "👤" }
                    else if **kind == 3 { "📇" }
                    else { "○" }, 
                    kind, count);
                execute!(
                    stdout,
                    cursor::MoveTo(kinds_x as u16, y),
                    SetForegroundColor(colors::MEDIUM_GRAY),
                    Print(&kind_line),
                    SetForegroundColor(colors::DARK_GRAY),
                    Print(format!(" ({})", kind_name)),
                    ResetColor
                )?;
                y += 1;
            }
            
            if remaining > 0 {
                execute!(
                    stdout,
                    cursor::MoveTo(kinds_x as u16, y),
                    SetForegroundColor(colors::DARK_GRAY),
                    Print(format!("  + {} more kinds... (press m for details)", remaining)),
                    ResetColor
                )?;
                y += 1;
            }
        }
        
        y += 1; // Extra spacing
        
        // Continue prompt - centered
        let prompt_text = "Press q to exit";
        let prompt_x = (term_width.saturating_sub(prompt_text.len())) / 2;
        execute!(
            stdout,
            cursor::MoveTo(prompt_x as u16, y),
            SetForegroundColor(colors::OP1_ORANGE),
            Print("◉ "),
            Print(prompt_text),
            ResetColor
        )?;
        y += 2;

        // Status - centered
        let status_text = "● COMPLETE";
        let status_x = (term_width.saturating_sub(status_text.len())) / 2;
        execute!(
            stdout,
            cursor::MoveTo(status_x as u16, y),
            SetForegroundColor(colors::ACCENT_GREEN),
            Print(status_text),
            ResetColor
        )?;

        io::stdout().flush()?;
        Ok(())
    }
}
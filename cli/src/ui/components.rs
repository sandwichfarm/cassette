use crossterm::style::{Color, Stylize, StyledContent};
use super::colors;
use std::time::Duration;

pub struct Counter {
    value: u64,
    max_digits: usize,
}

impl Counter {
    pub fn new(max_digits: usize) -> Self {
        Self { value: 0, max_digits }
    }

    pub fn set(&mut self, value: u64) {
        self.value = value;
    }

    pub fn increment(&mut self) {
        self.value += 1;
    }

    pub fn render(&self, color: Color) -> String {
        let formatted = format!("{:0width$}", self.value, width = self.max_digits);
        let mut result = String::new();
        
        for digit in formatted.chars() {
            result.push_str(&format!(
                "{}",
                digit.to_string()
                    .with(color)
                    .on(Color::Rgb { r: 30, g: 30, b: 30 })
            ));
        }
        
        result
    }

    pub fn render_value(&self, value: u64, color: Color) -> String {
        let formatted = format!("{:0width$}", value, width = self.max_digits);
        let mut result = String::new();
        
        for digit in formatted.chars() {
            result.push_str(&format!(
                "{}",
                digit.to_string()
                    .with(color)
                    .on(Color::Rgb { r: 30, g: 30, b: 30 })
            ));
        }
        
        result
    }
}

pub struct ProgressBar {
    width: usize,
    label: String,
}

impl ProgressBar {
    pub fn new(width: usize, label: String) -> Self {
        Self { width, label }
    }

    pub fn render(&self, progress: f32, color: Color) -> String {
        let filled = (progress * self.width as f32) as usize;
        let empty = self.width - filled;
        
        format!(
            "{} [{}{}] {:.0}%",
            self.label,
            "█".repeat(filled).with(color),
            "░".repeat(empty).with(Color::Rgb { r: 60, g: 60, b: 60 }),
            progress * 100.0
        )
    }
}

pub struct EventTypeIndicator;

impl EventTypeIndicator {
    pub fn render(kind: u64) -> String {
        let (symbol, name) = match kind {
            0 => ("◆", "META"),
            1 => ("◉", "NOTE"),
            3 => ("◈", "CONT"),
            4 => ("◐", "ENCR"),
            5 => ("◮", "DELE"),
            6 => ("◎", "REPO"),
            7 => ("◉", "REAC"),
            30023 => ("◆", "LONG"),
            _ => ("○", "UNKN"),
        };
        
        let color = colors::event_kind_color(kind);
        format!("{} {}", symbol.with(color), name.with(color))
    }
}

pub struct CassetteArt;

impl CassetteArt {
    pub fn render() -> Vec<String> {
        vec![
            "┌─────────────────────────────┐",
            "│  ┌─┐              ┌─┐       │",
            "│  │●│──────────────│●│       │",
            "│  └─┘              └─┘       │",
            "│ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │",
            "│ ░░░░░░░░░░░░░░░░░░░░░░░░░ │",
            "└─────────────────────────────┘",
        ].iter().map(|&s| {
            s.chars().map(|c| {
                match c {
                    '●' => format!("{}", c.to_string().with(Color::Rgb { r: 100, g: 80, b: 60 })),
                    '▓' => format!("{}", c.to_string().with(Color::Rgb { r: 140, g: 120, b: 100 })),
                    '░' => format!("{}", c.to_string().with(Color::Rgb { r: 60, g: 60, b: 60 })),
                    _ => format!("{}", c.to_string().with(Color::Rgb { r: 180, g: 180, b: 180 })),
                }
            }).collect()
        }).collect()
    }
}

pub fn render_header(title: &str) -> String {
    let border = "═".repeat(title.len() + 4);
    format!(
        "{}\n{}\n{}",
        format!("╔{}╗", border).with(colors::ACCENT_BLUE),
        format!("║ {} ║", title.with(colors::FOREGROUND)).with(colors::ACCENT_BLUE),
        format!("╚{}╝", border).with(colors::ACCENT_BLUE)
    )
}

pub fn render_event_stats(total: u64, by_kind: &[(u64, u64)]) -> Vec<String> {
    let mut lines = vec![];
    
    lines.push(format!("Total Events: {}", 
        total.to_string().with(colors::ACCENT_GREEN)
    ));
    
    lines.push("".to_string());
    lines.push(format!("{}", "Event Types:".with(colors::FOREGROUND)));
    
    for &(kind, count) in by_kind {
        let indicator = EventTypeIndicator::render(kind);
        let bar_width = 20;
        let percentage = count as f32 / total as f32;
        let filled = (percentage * bar_width as f32) as usize;
        
        lines.push(format!(
            "  {} {} [{}{}]",
            indicator,
            format!("{:>6}", count).with(colors::FOREGROUND),
            "█".repeat(filled).with(colors::event_kind_color(kind)),
            "░".repeat(bar_width - filled).with(Color::Rgb { r: 60, g: 60, b: 60 })
        ));
    }
    
    lines
}
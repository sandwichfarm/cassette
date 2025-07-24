use std::time::Duration;
use crossterm::style::{Color, Stylize};

pub struct TapeReel {
    frame: usize,
    size: usize,
}

impl TapeReel {
    pub fn new(size: usize) -> Self {
        Self { frame: 0, size }
    }

    pub fn next_frame(&mut self) -> String {
        self.frame = (self.frame + 1) % 8;
        self.get_frame_symbol()
    }
    
    pub fn get_frame_symbol(&self) -> String {
        let symbols = ["◯", "◐", "◑", "◒", "●", "◓", "◔", "◕"];
        symbols[self.frame].to_string()
    }

    pub fn render(&self, filled: bool) -> Vec<String> {
        let patterns = [
            vec!["  ╱─┐  ", " ╱   ┐ ", "│  ●  │", " ╲   ┘ ", "  ╲─┘  "],
            vec!["   ╱┐  ", "  ╱ ┐  ", " │ ● │ ", "  ╲ ┘  ", "   ╲┘  "],
            vec!["   ─┐  ", "  ╱ │  ", " │ ● │ ", "  ╲ │  ", "   ─┘  "],
            vec!["   ┐   ", "  ╱│╲  ", " │ ● │ ", "  ╲│╱  ", "   ┘   "],
            vec!["  ┌─╲  ", " ┌   ╲ ", "│  ●  │", " └   ╱ ", "  └─╱  "],
            vec!["  ┌╲   ", "  ┌ ╲  ", " │ ● │ ", "  └ ╱  ", "  └╱   "],
            vec!["  ┌─   ", "  │ ╲  ", " │ ● │ ", "  │ ╱  ", "  └─   "],
            vec!["   ┌   ", "  ╱│╲  ", " │ ● │ ", "  ╲│╱  ", "   └   "],
        ];

        let reel = &patterns[self.frame];
        if filled {
            reel.iter().map(|line| {
                line.chars().map(|c| {
                    if c == '●' {
                        format!("{}", c.with(Color::Rgb { r: 255, g: 255, b: 255 }).on(Color::Rgb { r: 100, g: 80, b: 60 }))
                    } else if c != ' ' {
                        format!("{}", c.to_string().with(Color::Rgb { r: 140, g: 120, b: 100 }))
                    } else {
                        c.to_string()
                    }
                }).collect()
            }).collect()
        } else {
            reel.iter().map(|line| {
                line.chars().map(|c| {
                    if c == '●' {
                        format!("{}", c.to_string().with(Color::Rgb { r: 60, g: 60, b: 60 }))
                    } else if c != ' ' {
                        format!("{}", c.to_string().with(Color::Rgb { r: 100, g: 100, b: 100 }))
                    } else {
                        c.to_string()
                    }
                }).collect()
            }).collect()
        }
    }
}

pub struct VUMeter {
    levels: Vec<f32>,
    max_level: usize,
}

impl VUMeter {
    pub fn new(channels: usize, max_level: usize) -> Self {
        Self {
            levels: vec![0.0; channels],
            max_level,
        }
    }

    pub fn update(&mut self, channel: usize, level: f32) {
        if channel < self.levels.len() {
            self.levels[channel] = level.clamp(0.0, 1.0);
        }
    }

    pub fn render(&self) -> Vec<String> {
        let mut lines = vec![];
        
        for (i, &level) in self.levels.iter().enumerate() {
            let filled = (level * self.max_level as f32) as usize;
            let mut meter = String::new();
            
            for j in 0..self.max_level {
                if j < filled {
                    let color = if j < self.max_level * 60 / 100 {
                        Color::Rgb { r: 0, g: 255, b: 136 }
                    } else if j < self.max_level * 85 / 100 {
                        Color::Rgb { r: 255, g: 204, b: 0 }
                    } else {
                        Color::Rgb { r: 255, g: 68, b: 68 }
                    };
                    meter.push_str(&format!("{}", "█".with(color)));
                } else {
                    meter.push_str(&format!("{}", "░".with(Color::Rgb { r: 60, g: 60, b: 60 })));
                }
            }
            
            lines.push(format!("CH{} {}", i + 1, meter));
        }
        
        lines
    }
}

pub struct Waveform {
    data: Vec<f32>,
    width: usize,
    height: usize,
}

impl Waveform {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            data: vec![0.0; width],
            width,
            height,
        }
    }

    pub fn push(&mut self, value: f32) {
        self.data.rotate_left(1);
        self.data[self.width - 1] = value.clamp(-1.0, 1.0);
    }

    pub fn render(&self, color: Color) -> Vec<String> {
        let mut lines = vec![String::new(); self.height];
        
        for (x, &value) in self.data.iter().enumerate() {
            let y = ((1.0 - value) * self.height as f32 / 2.0) as usize;
            let y = y.clamp(0, self.height - 1);
            
            for line_y in 0..self.height {
                if line_y == y {
                    lines[line_y].push_str(&format!("{}", "▄".with(color)));
                } else if line_y == self.height / 2 {
                    lines[line_y].push_str(&format!("{}", "─".with(Color::Rgb { r: 60, g: 60, b: 60 })));
                } else {
                    lines[line_y].push(' ');
                }
            }
        }
        
        lines
    }
}
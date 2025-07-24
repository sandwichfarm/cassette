use crossterm::style::Color;

// OP-1 inspired color palette
pub const BACKGROUND: Color = Color::Rgb { r: 20, g: 20, b: 20 };
pub const FOREGROUND: Color = Color::Rgb { r: 220, g: 220, b: 220 };
pub const ACCENT_GREEN: Color = Color::Rgb { r: 0, g: 255, b: 136 };
pub const ACCENT_BLUE: Color = Color::Rgb { r: 0, g: 136, b: 255 };
pub const ACCENT_RED: Color = Color::Rgb { r: 255, g: 68, b: 68 };
pub const ACCENT_YELLOW: Color = Color::Rgb { r: 255, g: 204, b: 0 };
pub const ACCENT_PURPLE: Color = Color::Rgb { r: 187, g: 134, b: 252 };
pub const ACCENT_ORANGE: Color = Color::Rgb { r: 255, g: 136, b: 0 };

// OP-1 specific color aliases
pub const OP1_ORANGE: Color = ACCENT_ORANGE;
pub const OP1_BLUE: Color = ACCENT_BLUE;
pub const OP1_RED: Color = ACCENT_RED;

// Grayscale
pub const DARK_GRAY: Color = Color::Rgb { r: 60, g: 60, b: 60 };
pub const MEDIUM_GRAY: Color = Color::Rgb { r: 140, g: 140, b: 140 };

// Event kind colors
pub fn event_kind_color(kind: u64) -> Color {
    match kind {
        0 => ACCENT_PURPLE,      // Metadata
        1 => ACCENT_GREEN,       // Short Text Note
        3 => ACCENT_BLUE,        // Contacts
        4 => ACCENT_YELLOW,      // Encrypted Direct Messages
        5 => ACCENT_RED,         // Event Deletion
        6 => ACCENT_ORANGE,      // Repost
        7 => ACCENT_GREEN,       // Reaction
        30023 => ACCENT_PURPLE,  // Long-form Content
        _ => FOREGROUND,
    }
}
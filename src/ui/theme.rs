use ratatui::style::Color;

/// Named palette used across all views.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub accent: Color,
    pub ok: Color,
    pub warn: Color,
    pub error: Color,
    pub text: Color,
    pub dim: Color,
    pub highlight_bg: Color,
    pub frozen: Color,
}

pub const DEFAULT: Theme = Theme {
    accent: Color::Cyan,
    ok: Color::Green,
    warn: Color::Yellow,
    error: Color::Red,
    text: Color::Reset,
    dim: Color::DarkGray,
    highlight_bg: Color::Blue,
    frozen: Color::LightMagenta,
};

pub const OCEAN: Theme = Theme {
    accent: Color::LightBlue,
    ok: Color::Rgb(80, 200, 190),
    warn: Color::Rgb(230, 190, 90),
    error: Color::LightRed,
    text: Color::White,
    dim: Color::Rgb(90, 110, 140),
    highlight_bg: Color::Rgb(30, 60, 100),
    frozen: Color::LightCyan,
};

pub const MONO: Theme = Theme {
    accent: Color::Gray,
    ok: Color::Gray,
    warn: Color::White,
    error: Color::White,
    text: Color::Gray,
    dim: Color::DarkGray,
    highlight_bg: Color::Rgb(70, 70, 70),
    frozen: Color::White,
};

/// Look up a theme by config name; unknown names fall back to `DEFAULT`.
pub fn theme(name: &str) -> Theme {
    match name {
        "ocean" => OCEAN,
        "mono" => MONO,
        _ => DEFAULT,
    }
}

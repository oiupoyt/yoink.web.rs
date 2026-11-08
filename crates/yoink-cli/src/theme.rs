use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Auto,
    Dark,
    Light,
}

#[derive(Debug, Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    pub background: Color,
    pub surface: Color,
    pub border: Color,
    pub primary: Color,
    pub secondary: Color,
    pub accent_emerald: Color,
    pub accent_cyan: Color,
}

impl Theme {
    pub fn new(mode: ThemeMode) -> Self {
        if std::env::var("NO_COLOR").is_ok() {
            return Self {
                mode,
                background: Color::Reset,
                surface: Color::Reset,
                border: Color::Reset,
                primary: Color::Reset,
                secondary: Color::Reset,
                accent_emerald: Color::Reset,
                accent_cyan: Color::Reset,
            };
        }

        match mode {
            ThemeMode::Auto | ThemeMode::Dark => Self {
                mode,
                // Monochrome & Minimalist
                background: Color::Rgb(0, 0, 0),        // #000000
                surface: Color::Rgb(15, 15, 15),       // #0f0f0f
                border: Color::Rgb(40, 40, 40),        // #282828
                primary: Color::Rgb(255, 255, 255),    // white
                secondary: Color::Rgb(120, 120, 120),  // gray
                accent_emerald: Color::Rgb(255, 255, 255), // white
                accent_cyan: Color::Rgb(255, 255, 255),    // white
            },
            ThemeMode::Light => Self {
                mode,
                background: Color::Rgb(245, 245, 245),
                surface: Color::Rgb(235, 235, 235),
                border: Color::Rgb(200, 200, 200),
                primary: Color::Rgb(20, 20, 20),
                secondary: Color::Rgb(100, 100, 100),
                accent_emerald: Color::Rgb(5, 150, 105),
                accent_cyan: Color::Rgb(2, 132, 199),
            },
        }
    }

    pub fn cycle(&self) -> Self {
        let next = match self.mode {
            ThemeMode::Auto => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Auto,
        };
        Self::new(next)
    }
}

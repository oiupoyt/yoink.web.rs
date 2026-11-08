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
                // KAT Square Minimalist Obsidian palette
                background: Color::Rgb(11, 11, 11),  // #0b0b0b
                surface: Color::Rgb(17, 17, 17),     // #111111
                border: Color::Rgb(34, 34, 34),      // #222222
                primary: Color::Rgb(245, 245, 245),  // crisp white
                secondary: Color::Rgb(140, 140, 140),// muted gray
                accent_emerald: Color::Rgb(16, 185, 129), // #10b981
                accent_cyan: Color::Rgb(56, 189, 248),    // #38bdf8
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

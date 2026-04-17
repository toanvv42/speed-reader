#[cfg(not(target_arch = "wasm32"))]
use clap::ValueEnum;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), derive(ValueEnum))]
pub enum ThemeChoice {
    Dark,
    Light,
    System,
}

impl ThemeChoice {
    pub fn cycle(self) -> Self {
        match self {
            ThemeChoice::Dark => ThemeChoice::Light,
            ThemeChoice::Light => ThemeChoice::System,
            ThemeChoice::System => ThemeChoice::Dark,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
            ThemeChoice::System => "system",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Theme {
    Dark,
    Light,
}

#[cfg(not(target_arch = "wasm32"))]
mod tui {
    use ratatui::style::Color;

    use super::Theme;

    #[derive(Copy, Clone, Debug)]
    pub struct Palette {
        pub fg: Color,
        pub bg: Color,
        pub dim: Color,
        pub accent: Color,
        pub status_fg: Color,
        pub status_bg: Color,
        pub modal_bg: Color,
        pub list_hl_bg: Color,
        pub divider: Color,
    }

    impl Palette {
        pub fn for_theme(t: Theme) -> Self {
            match t {
                Theme::Dark => Palette {
                    fg: Color::Rgb(230, 230, 230),
                    bg: Color::Rgb(16, 16, 20),
                    dim: Color::Rgb(140, 140, 140),
                    accent: Color::Rgb(255, 80, 80),
                    status_fg: Color::Rgb(180, 180, 180),
                    status_bg: Color::Rgb(20, 20, 30),
                    modal_bg: Color::Rgb(18, 18, 24),
                    list_hl_bg: Color::Rgb(40, 40, 60),
                    divider: Color::Rgb(60, 60, 80),
                },
                Theme::Light => Palette {
                    fg: Color::Rgb(30, 30, 35),
                    bg: Color::Rgb(250, 249, 245),
                    dim: Color::Rgb(120, 120, 130),
                    accent: Color::Rgb(200, 40, 40),
                    status_fg: Color::Rgb(70, 70, 80),
                    status_bg: Color::Rgb(232, 230, 222),
                    modal_bg: Color::Rgb(255, 254, 250),
                    list_hl_bg: Color::Rgb(215, 225, 245),
                    divider: Color::Rgb(200, 198, 190),
                },
            }
        }
    }

    pub fn resolve(choice: super::ThemeChoice) -> Theme {
        match choice {
            super::ThemeChoice::Dark => Theme::Dark,
            super::ThemeChoice::Light => Theme::Light,
            super::ThemeChoice::System => detect_system(),
        }
    }

    /// Query the terminal for its background color (OSC 11) and decide.
    /// Falls back to Dark if the terminal doesn't respond (pipe, dumb tty, etc.).
    /// Must be called BEFORE entering raw mode / alt screen.
    pub fn detect_system() -> Theme {
        match terminal_light::luma() {
            Ok(luma) if luma > 0.5 => Theme::Light,
            _ => Theme::Dark,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use tui::{Palette, detect_system, resolve};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_rotates_dark_light_system() {
        assert_eq!(ThemeChoice::Dark.cycle(), ThemeChoice::Light);
        assert_eq!(ThemeChoice::Light.cycle(), ThemeChoice::System);
        assert_eq!(ThemeChoice::System.cycle(), ThemeChoice::Dark);
    }

    #[test]
    fn label_matches_variant() {
        assert_eq!(ThemeChoice::Dark.label(), "dark");
        assert_eq!(ThemeChoice::Light.label(), "light");
        assert_eq!(ThemeChoice::System.label(), "system");
    }

    #[test]
    fn resolve_fixed_choices_bypass_detection() {
        assert_eq!(resolve(ThemeChoice::Dark), Theme::Dark);
        assert_eq!(resolve(ThemeChoice::Light), Theme::Light);
    }
}

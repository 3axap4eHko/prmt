use std::fmt::Write;
use std::str::FromStr;
use std::sync::atomic::{AtomicU8, Ordering};

const COLOR_UNKNOWN: u8 = 0;
const COLOR_FALSE: u8 = 1;
const COLOR_TRUE: u8 = 2;

static NO_COLOR_STATE: AtomicU8 = AtomicU8::new(COLOR_UNKNOWN);

pub fn global_no_color() -> bool {
    match NO_COLOR_STATE.load(Ordering::Relaxed) {
        COLOR_TRUE => true,
        COLOR_FALSE => false,
        _ => {
            let detected = std::env::var_os("NO_COLOR").is_some();
            NO_COLOR_STATE.store(
                if detected { COLOR_TRUE } else { COLOR_FALSE },
                Ordering::Relaxed,
            );
            detected
        }
    }
}

#[cfg(test)]
pub fn reset_global_no_color_for_tests() {
    NO_COLOR_STATE.store(COLOR_UNKNOWN, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shell {
    #[default]
    None,
    Zsh,
    Bash,
}

impl Shell {
    fn delimiters(self) -> (&'static str, &'static str) {
        match self {
            Shell::Zsh => ("%{", "%}"),
            Shell::Bash => ("\x01", "\x02"),
            Shell::None => ("", ""),
        }
    }
}

impl FromStr for Shell {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "zsh" => Ok(Shell::Zsh),
            "bash" => Ok(Shell::Bash),
            "none" | "" => Ok(Shell::None),
            other => Err(format!(
                "Unknown shell: {} (supported values: bash, zsh, none)",
                other
            )),
        }
    }
}

pub trait ModuleStyle: Sized {
    fn parse(style_str: &str) -> Result<Self, String>;
    fn apply(&self, text: &str) -> String;

    fn apply_with_shell(&self, text: &str, shell: Shell) -> String {
        let _ = shell;
        self.apply(text)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Purple,
    Cyan,
    White,
    Hex(String),
}

impl Color {
    fn push_ansi_code(&self, buf: &mut String) {
        match self {
            Color::Black => buf.push_str("\x1b[30m"),
            Color::Red => buf.push_str("\x1b[31m"),
            Color::Green => buf.push_str("\x1b[32m"),
            Color::Yellow => buf.push_str("\x1b[33m"),
            Color::Blue => buf.push_str("\x1b[34m"),
            Color::Purple => buf.push_str("\x1b[35m"),
            Color::Cyan => buf.push_str("\x1b[36m"),
            Color::White => buf.push_str("\x1b[37m"),
            Color::Hex(hex) => {
                if let Ok((r, g, b)) = parse_hex_color(hex) {
                    let _ = write!(buf, "\x1b[38;2;{};{};{}m", r, g, b);
                }
            }
        }
    }

    fn push_ansi_bg_code(&self, buf: &mut String) {
        match self {
            Color::Black => buf.push_str("\x1b[40m"),
            Color::Red => buf.push_str("\x1b[41m"),
            Color::Green => buf.push_str("\x1b[42m"),
            Color::Yellow => buf.push_str("\x1b[43m"),
            Color::Blue => buf.push_str("\x1b[44m"),
            Color::Purple => buf.push_str("\x1b[45m"),
            Color::Cyan => buf.push_str("\x1b[46m"),
            Color::White => buf.push_str("\x1b[47m"),
            Color::Hex(hex) => {
                if let Ok((r, g, b)) = parse_hex_color(hex) {
                    let _ = write!(buf, "\x1b[48;2;{};{};{}m", r, g, b);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnsiStyle {
    pub color: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

impl ModuleStyle for AnsiStyle {
    fn parse(style_str: &str) -> Result<Self, String> {
        let mut style = AnsiStyle::default();

        if style_str.is_empty() {
            return Ok(style);
        }

        for part in style_str.split('.') {
            match part {
                "bold" => style.bold = true,
                "italic" => style.italic = true,
                "underline" => style.underline = true,
                "dim" => style.dim = true,
                "reverse" => style.reverse = true,
                "strikethrough" => style.strikethrough = true,
                _ => {
                    if part.contains('+') {
                        let mut split = part.splitn(2, '+');
                        let fg = split.next().unwrap_or("");
                        let bg = split.next().unwrap_or("");
                        if !fg.is_empty() {
                            style.color = Some(parse_color(fg)?);
                        }
                        if bg.is_empty() {
                            return Err(format!("Unknown style component: {}", part));
                        }
                        style.background = Some(parse_color(bg)?);
                    } else {
                        style.color = Some(parse_color(part)?);
                    }
                }
            }
        }

        Ok(style)
    }

    fn apply(&self, text: &str) -> String {
        self.apply_with_shell(text, Shell::None)
    }

    fn apply_with_shell(&self, text: &str, shell: Shell) -> String {
        if !self.has_style() {
            return text.to_string();
        }

        let mut output = String::with_capacity(text.len() + 16);
        self.write_start_codes(&mut output, shell);
        output.push_str(text);
        self.write_reset(&mut output, shell);
        output
    }
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), String> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        return Err(format!("Invalid hex color: {}", hex));
    }

    let r =
        u8::from_str_radix(&hex[0..2], 16).map_err(|_| format!("Invalid hex color: {}", hex))?;
    let g =
        u8::from_str_radix(&hex[2..4], 16).map_err(|_| format!("Invalid hex color: {}", hex))?;
    let b =
        u8::from_str_radix(&hex[4..6], 16).map_err(|_| format!("Invalid hex color: {}", hex))?;

    Ok((r, g, b))
}

impl AnsiStyle {
    fn has_style(&self) -> bool {
        self.color.is_some()
            || self.background.is_some()
            || self.bold
            || self.italic
            || self.underline
            || self.dim
            || self.reverse
            || self.strikethrough
    }

    fn write_raw_codes(&self, buf: &mut String) {
        if let Some(ref color) = self.color {
            color.push_ansi_code(buf);
        }
        if let Some(ref background) = self.background {
            background.push_ansi_bg_code(buf);
        }
        if self.bold {
            buf.push_str("\x1b[1m");
        }
        if self.dim {
            buf.push_str("\x1b[2m");
        }
        if self.italic {
            buf.push_str("\x1b[3m");
        }
        if self.underline {
            buf.push_str("\x1b[4m");
        }
        if self.reverse {
            buf.push_str("\x1b[7m");
        }
        if self.strikethrough {
            buf.push_str("\x1b[9m");
        }
    }

    pub fn write_start_codes(&self, buf: &mut String, shell: Shell) {
        if !self.has_style() {
            return;
        }

        if shell == Shell::None {
            self.write_raw_codes(buf);
        } else {
            let (start, end) = shell.delimiters();
            buf.push_str(start);
            self.write_raw_codes(buf);
            buf.push_str(end);
        }
    }

    pub fn write_reset(&self, buf: &mut String, shell: Shell) {
        if !self.has_style() {
            return;
        }

        if shell == Shell::None {
            buf.push_str("\x1b[0m");
        } else {
            let (start, end) = shell.delimiters();
            buf.push_str(start);
            buf.push_str("\x1b[0m");
            buf.push_str(end);
        }
    }
}

fn parse_color(value: &str) -> Result<Color, String> {
    match value {
        "black" => Ok(Color::Black),
        "red" => Ok(Color::Red),
        "green" => Ok(Color::Green),
        "yellow" => Ok(Color::Yellow),
        "blue" => Ok(Color::Blue),
        "purple" | "magenta" => Ok(Color::Purple),
        "cyan" => Ok(Color::Cyan),
        "white" => Ok(Color::White),
        hex if hex.starts_with('#') => Ok(Color::Hex(hex.to_string())),
        _ => Err(format!("Unknown style component: {}", value)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    fn unset_no_color() {
        unsafe {
            env::remove_var("NO_COLOR");
        }
    }

    fn set_no_color() {
        unsafe {
            env::set_var("NO_COLOR", "1");
        }
    }

    #[test]
    fn test_parse_simple_color() {
        let style = AnsiStyle::parse("red").unwrap();
        assert_eq!(style.color, Some(Color::Red));
        assert!(!style.bold);
    }

    #[test]
    fn test_parse_color_with_modifiers() {
        let style = AnsiStyle::parse("cyan.bold.italic").unwrap();
        assert_eq!(style.color, Some(Color::Cyan));
        assert!(style.bold);
        assert!(style.italic);
    }

    #[test]
    fn test_parse_hex_color() {
        let style = AnsiStyle::parse("#00ff00").unwrap();
        assert!(matches!(style.color, Some(Color::Hex(_))));
    }

    #[test]
    fn test_parse_fg_bg_colors() {
        let style = AnsiStyle::parse("red+#00ff00").unwrap();
        assert_eq!(style.color, Some(Color::Red));
        assert_eq!(style.background, Some(Color::Hex("#00ff00".to_string())));
    }

    #[test]
    fn test_parse_bg_only() {
        let style = AnsiStyle::parse("+#112233").unwrap();
        assert_eq!(style.color, None);
        assert_eq!(style.background, Some(Color::Hex("#112233".to_string())));
    }

    #[test]
    fn test_apply_style() {
        let style = AnsiStyle::parse("red.bold").unwrap();
        let result = style.apply("test");
        assert!(result.starts_with("\x1b[31m"));
        assert!(result.contains("\x1b[1m"));
        assert!(result.ends_with("test\x1b[0m"));
    }

    #[test]
    fn test_empty_style() {
        let style = AnsiStyle::parse("").unwrap();
        let result = style.apply("test");
        assert_eq!(result, "test");
    }

    #[test]
    fn test_apply_with_background() {
        let style = AnsiStyle::parse("red+#00ff00").unwrap();
        let result = style.apply("test");
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("\x1b[48;2;0;255;0m"));
        assert!(result.ends_with("test\x1b[0m"));
    }

    #[test]
    fn test_apply_with_shell_wraps_bash_sequences() {
        let style = AnsiStyle::parse("red.bold").unwrap();
        let result = style.apply_with_shell("ok", Shell::Bash);
        assert!(result.starts_with("\x01\x1b[31m\x1b[1m\x02"));
        assert!(result.ends_with("ok\x01\x1b[0m\x02"));
    }

    #[test]
    fn test_shell_from_str() {
        assert_eq!(Shell::from_str("bash").unwrap(), Shell::Bash);
        assert_eq!(Shell::from_str("ZSH").unwrap(), Shell::Zsh);
        assert_eq!(Shell::from_str("none").unwrap(), Shell::None);
        assert!(Shell::from_str("fish").is_err());
    }

    #[test]
    #[serial]
    fn global_no_color_respects_env() {
        unset_no_color();
        reset_global_no_color_for_tests();
        assert!(!global_no_color());

        set_no_color();
        reset_global_no_color_for_tests();
        assert!(global_no_color());

        unset_no_color();
        reset_global_no_color_for_tests();
    }

    #[test]
    #[serial]
    fn global_no_color_caches_until_reset() {
        unset_no_color();
        reset_global_no_color_for_tests();
        assert!(!global_no_color());

        set_no_color();
        // Without reset we still expect false due to caching
        assert!(!global_no_color());

        reset_global_no_color_for_tests();
        assert!(global_no_color());

        unset_no_color();
        reset_global_no_color_for_tests();
    }
}

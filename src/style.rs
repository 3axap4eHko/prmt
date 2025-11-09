use std::fmt::Write;

pub trait ModuleStyle: Sized {
    fn parse(style_str: &str) -> Result<Self, String>;
    fn apply_with_shell(&self, text: &str, shell: Shell) -> String;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shell {
    #[default]
    None,
    Zsh,
    Bash,
}

impl Shell {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "zsh" => Ok(Shell::Zsh),
            "bash" => Ok(Shell::Bash),
            "none" | "" => Ok(Shell::None),
            _ => Err(format!("Unknown shell: {} (supported: zsh, bash, none)", s)),
        }
    }

    fn delimiters(&self) -> (&'static str, &'static str) {
        match self {
            Shell::Zsh => ("%{", "%}"),
            Shell::Bash => ("\\[", "\\]"),
            Shell::None => ("", ""),
        }
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
    fn push_ansi_code(&self, buf: &mut String, shell: Shell) {
        let (start, end) = shell.delimiters();
        buf.push_str(start);
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
        buf.push_str(end);
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AnsiStyle {
    pub color: Option<Color>,
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
                "black" => style.color = Some(Color::Black),
                "red" => style.color = Some(Color::Red),
                "green" => style.color = Some(Color::Green),
                "yellow" => style.color = Some(Color::Yellow),
                "blue" => style.color = Some(Color::Blue),
                "purple" | "magenta" => style.color = Some(Color::Purple),
                "cyan" => style.color = Some(Color::Cyan),
                "white" => style.color = Some(Color::White),
                hex if hex.starts_with('#') => {
                    style.color = Some(Color::Hex(hex.to_string()));
                }
                _ => return Err(format!("Unknown style component: {}", part)),
            }
        }

        Ok(style)
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
            || self.bold
            || self.italic
            || self.underline
            || self.dim
            || self.reverse
            || self.strikethrough
    }

    #[allow(dead_code)]
    pub fn apply(&self, text: &str) -> String {
        self.apply_with_shell(text, Shell::None)
    }

    pub fn write_start_codes(&self, buf: &mut String, shell: Shell) {
        let (start, end) = shell.delimiters();
        
        if let Some(ref color) = self.color {
            color.push_ansi_code(buf, shell);
        }
        if self.bold {
            buf.push_str(start);
            buf.push_str("\x1b[1m");
            buf.push_str(end);
        }
        if self.dim {
            buf.push_str(start);
            buf.push_str("\x1b[2m");
            buf.push_str(end);
        }
        if self.italic {
            buf.push_str(start);
            buf.push_str("\x1b[3m");
            buf.push_str(end);
        }
        if self.underline {
            buf.push_str(start);
            buf.push_str("\x1b[4m");
            buf.push_str(end);
        }
        if self.reverse {
            buf.push_str(start);
            buf.push_str("\x1b[7m");
            buf.push_str(end);
        }
        if self.strikethrough {
            buf.push_str(start);
            buf.push_str("\x1b[9m");
            buf.push_str(end);
        }
    }

    pub fn write_reset(&self, buf: &mut String, shell: Shell) {
        if self.has_style() {
            let (start, end) = shell.delimiters();
            buf.push_str(start);
            buf.push_str("\x1b[0m");
            buf.push_str(end);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

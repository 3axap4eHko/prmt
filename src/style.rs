pub trait ModuleStyle: Sized {
    fn parse(style_str: &str) -> Result<Self, String>;
    fn apply(&self, text: &str) -> String;
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
    fn to_ansi_code(&self) -> String {
        match self {
            Color::Black => "\x1b[30m".to_string(),
            Color::Red => "\x1b[31m".to_string(),
            Color::Green => "\x1b[32m".to_string(),
            Color::Yellow => "\x1b[33m".to_string(),
            Color::Blue => "\x1b[34m".to_string(),
            Color::Purple => "\x1b[35m".to_string(),
            Color::Cyan => "\x1b[36m".to_string(),
            Color::White => "\x1b[37m".to_string(),
            Color::Hex(hex) => {
                if let Ok(rgb) = parse_hex_color(hex) {
                    format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2)
                } else {
                    String::new()
                }
            }
        }
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
    
    fn apply(&self, text: &str) -> String {
        let mut codes = Vec::new();
        
        if let Some(ref color) = self.color {
            codes.push(color.to_ansi_code());
        }
        if self.bold { codes.push("\x1b[1m".to_string()); }
        if self.dim { codes.push("\x1b[2m".to_string()); }
        if self.italic { codes.push("\x1b[3m".to_string()); }
        if self.underline { codes.push("\x1b[4m".to_string()); }
        if self.reverse { codes.push("\x1b[7m".to_string()); }
        if self.strikethrough { codes.push("\x1b[9m".to_string()); }
        
        if codes.is_empty() {
            text.to_string()
        } else {
            format!("{}{}\x1b[0m", codes.join(""), text)
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), String> {
    let hex = hex.trim_start_matches('#');
    
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: {}", hex));
    }
    
    let r = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| format!("Invalid hex color: {}", hex))?;
    
    Ok((r, g, b))
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
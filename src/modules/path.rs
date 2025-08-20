use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use std::env;
use unicode_width::UnicodeWidthStr;

pub struct PathModule;

impl Default for PathModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PathModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for PathModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        let current_dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        match format {
            "" | "relative" | "r" => {
                let path_str = current_dir.to_string_lossy();

                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    if path_str.starts_with(home_str.as_ref()) {
                        let replaced = path_str.replacen(home_str.as_ref(), "~", 1);
                        // On Windows, normalize path separators to forward slashes
                        #[cfg(target_os = "windows")]
                        let replaced = replaced.replace('\\', "/");
                        Ok(Some(replaced))
                    } else {
                        #[cfg(target_os = "windows")]
                        return Ok(Some(path_str.replace('\\', "/")));
                        #[cfg(not(target_os = "windows"))]
                        Ok(Some(path_str.to_string()))
                    }
                } else {
                    #[cfg(target_os = "windows")]
                    return Ok(Some(path_str.replace('\\', "/")));
                    #[cfg(not(target_os = "windows"))]
                    Ok(Some(path_str.to_string()))
                }
            }
            "absolute" | "a" | "f" => Ok(Some(current_dir.to_string_lossy().to_string())),
            "short" | "s" => Ok(current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .or_else(|| Some(".".to_string()))),
            format if format.starts_with("truncate:") => {
                let max_width: usize = format
                    .strip_prefix("truncate:")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);

                let path = if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    let path_str = current_dir.to_string_lossy();
                    if path_str.starts_with(home_str.as_ref()) {
                        path_str.replacen(home_str.as_ref(), "~", 1)
                    } else {
                        path_str.to_string()
                    }
                } else {
                    current_dir.to_string_lossy().to_string()
                };

                // Use unicode width for proper truncation
                let width = UnicodeWidthStr::width(path.as_str());
                if width <= max_width {
                    Ok(Some(path))
                } else {
                    // Truncate with ellipsis
                    let ellipsis = "...";
                    let ellipsis_width = 3;
                    let target_width = max_width.saturating_sub(ellipsis_width);

                    let mut truncated = String::new();
                    let mut current_width = 0;

                    for ch in path.chars() {
                        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
                        if current_width + ch_width > target_width {
                            break;
                        }
                        truncated.push(ch);
                        current_width += ch_width;
                    }

                    truncated.push_str(ellipsis);
                    Ok(Some(truncated))
                }
            }
            _ => Err(PromptError::InvalidFormat {
                module: "path".to_string(),
                format: format.to_string(),
                valid_formats: "relative, r, absolute, a, f, short, s, truncate:N".to_string(),
            }),
        }
    }
}

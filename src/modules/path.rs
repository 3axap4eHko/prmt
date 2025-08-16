use crate::module_trait::{Module, ModuleContext};
use std::env;

pub struct PathModule;

impl PathModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for PathModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Option<String> {
        let current_dir = env::current_dir().ok()?;
        
        match format {
            "" | "relative" | "r" => {
                let path_str = current_dir.to_string_lossy();
                
                if let Some(home) = dirs::home_dir() {
                    let home_str = home.to_string_lossy();
                    if path_str.starts_with(home_str.as_ref()) {
                        Some(path_str.replacen(home_str.as_ref(), "~", 1))
                    } else {
                        Some(path_str.to_string())
                    }
                } else {
                    Some(path_str.to_string())
                }
            }
            "absolute" | "a" => {
                Some(current_dir.to_string_lossy().to_string())
            }
            "short" | "s" => {
                current_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string())
                    .or_else(|| Some(".".to_string()))
            }
            _ => None,
        }
    }
}
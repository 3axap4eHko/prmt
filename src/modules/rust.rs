use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use crate::cache::VERSION_CACHE;
use std::process::Command;
use std::time::Duration;

pub struct RustModule;

impl Default for RustModule {
    fn default() -> Self {
        Self::new()
    }
}

impl RustModule {
    pub fn new() -> Self {
        Self
    }
}

#[cold]
fn get_rust_version() -> Option<String> {
    let output = Command::new("rustc")
        .arg("--version")
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let version_str = String::from_utf8_lossy(&output.stdout);
    version_str
        .split_whitespace()
        .nth(1)
        .map(|s| s.to_string())
}

impl Module for RustModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        utils::find_upward("Cargo.toml")?;
        
        if context.no_version {
            return Some("rust".to_string());
        }
        
        // Check cache first
        let cache_key = "rust_version";
        let version = if let Some(cached) = VERSION_CACHE.get(cache_key) {
            cached?
        } else {
            // Get version with timeout consideration
            let version = get_rust_version();
            VERSION_CACHE.insert(cache_key.to_string(), version.clone(), Duration::from_secs(300));
            version?
        };
        
        match format {
            "" | "full" => Some(version),
            "short" => {
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 2 {
                    Some(format!("{}.{}", parts[0], parts[1]))
                } else {
                    Some(version)
                }
            }
            "major" => {
                version.split('.').next().map(|s| s.to_string())
            }
            _ => None,
        }
    }
}
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct GoModule;

impl Default for GoModule {
    fn default() -> Self {
        Self::new()
    }
}

impl GoModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for GoModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        utils::find_upward("go.mod")?;
        
        if context.no_version {
            return Some("go".to_string());
        }
        
        let output = Command::new("go")
            .arg("version")
            .output()
            .ok()?;
        
        if !output.status.success() {
            return None;
        }
        
        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = version_str
            .split_whitespace()
            .nth(2)?
            .trim_start_matches("go")
            .to_string();
        
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
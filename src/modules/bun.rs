use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct BunModule;

impl Default for BunModule {
    fn default() -> Self {
        Self::new()
    }
}

impl BunModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for BunModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        utils::find_upward("bun.lockb").or_else(|| utils::find_upward("bunfig.toml"))?;

        if context.no_version {
            return Some("bun".to_string());
        }

        let output = Command::new("bun").arg("--version").output().ok()?;

        if !output.status.success() {
            return None;
        }

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

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
            "major" => version.split('.').next().map(|s| s.to_string()),
            _ => None,
        }
    }
}

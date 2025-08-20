use crate::error::Result;
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
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("bun.lockb")
            .or_else(|| utils::find_upward("bunfig.toml"))
            .is_none()
        {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("bun".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "bun")?;

        let output = match Command::new("bun").arg("--version").output() {
            Ok(o) if o.status.success() => o,
            _ => return Ok(None),
        };

        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();

        match normalized_format {
            "full" => Ok(Some(version)),
            "short" => {
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 2 {
                    Ok(Some(format!("{}.{}", parts[0], parts[1])))
                } else {
                    Ok(Some(version))
                }
            }
            "major" => Ok(version.split('.').next().map(|s| s.to_string())),
            _ => unreachable!("validate_version_format should have caught this"),
        }
    }
}

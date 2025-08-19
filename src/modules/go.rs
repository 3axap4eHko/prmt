use crate::error::Result;
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
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("go.mod").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("go".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "go")?;

        let output = match Command::new("go").arg("version").output() {
            Ok(o) if o.status.success() => o,
            _ => return Ok(None),
        };

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = match version_str.split_whitespace().nth(2) {
            Some(v) => v.trim_start_matches("go").to_string(),
            None => return Ok(None),
        };

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

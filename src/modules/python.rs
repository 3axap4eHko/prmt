use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct PythonModule;

impl Default for PythonModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for PythonModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("requirements.txt")
            .or_else(|| utils::find_upward("pyproject.toml"))
            .or_else(|| utils::find_upward("setup.py"))
            .is_none()
        {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("python".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "python")?;

        let output = Command::new("python3")
            .arg("--version")
            .output()
            .or_else(|_| Command::new("python").arg("--version").output())
            .ok();

        let output = match output {
            Some(o) if o.status.success() => o,
            _ => return Ok(None),
        };

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version = match version_str.split_whitespace().nth(1) {
            Some(v) => v.to_string(),
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

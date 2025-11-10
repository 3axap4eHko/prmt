use crate::error::Result;
use crate::memo::{BUN_VERSION, memoized_version};
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
    fn fs_markers(&self) -> &'static [&'static str] {
        &["bun.lockb", "bunfig.toml"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let has_marker = ["bun.lockb", "bunfig.toml"]
            .into_iter()
            .any(|marker| context.marker_path(marker).is_some());
        if !has_marker {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("bun".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "bun")?;

        let version = match memoized_version(&BUN_VERSION, get_bun_version) {
            Some(v) => v,
            None => return Ok(None),
        };
        let version_str = version.as_ref();

        match normalized_format {
            "full" => Ok(Some(version_str.to_string())),
            "short" => {
                let parts: Vec<&str> = version_str.split('.').collect();
                if parts.len() >= 2 {
                    Ok(Some(format!("{}.{}", parts[0], parts[1])))
                } else {
                    Ok(Some(version_str.to_string()))
                }
            }
            "major" => Ok(version_str.split('.').next().map(|s| s.to_string())),
            _ => unreachable!("validate_version_format should have caught this"),
        }
    }
}

#[cold]
fn get_bun_version() -> Option<String> {
    let output = Command::new("bun").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

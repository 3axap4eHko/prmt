use crate::cache::VERSION_CACHE;
use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct NodeModule;

impl Default for NodeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeModule {
    pub fn new() -> Self {
        Self
    }
}

#[cold]
fn get_node_version() -> Option<String> {
    let output = Command::new("node").arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);
    Some(version_str.trim().trim_start_matches('v').to_string())
}

impl Module for NodeModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("package.json").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("node".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "node")?;

        // Check cache first
        let cache_key = "node_version";
        let version = if let Some(cached) = VERSION_CACHE.get(cache_key) {
            match cached {
                Some(v) => v,
                None => return Ok(None),
            }
        } else {
            let version = get_node_version();
            VERSION_CACHE.insert(cache_key.to_string(), version.clone());
            match version {
                Some(v) => v,
                None => return Ok(None),
            }
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

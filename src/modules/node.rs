use crate::error::Result;
use crate::memo::{NODE_VERSION, memoized_version};
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
    fn fs_markers(&self) -> &'static [&'static str] {
        &["package.json"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if context.marker_path("package.json").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("node".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "node")?;

        // Check memoized value first
        let version = match memoized_version(&NODE_VERSION, get_node_version) {
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

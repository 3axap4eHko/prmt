use crate::error::Result;
use crate::memo::{DENO_VERSION, memoized_version};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct DenoModule;

impl Default for DenoModule {
    fn default() -> Self {
        Self::new()
    }
}

impl DenoModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for DenoModule {
    fn fs_markers(&self) -> &'static [&'static str] {
        &["deno.json", "deno.jsonc"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let has_marker = ["deno.json", "deno.jsonc"]
            .into_iter()
            .any(|marker| context.marker_path(marker).is_some());
        if !has_marker {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("deno".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "deno")?;

        let version = match memoized_version(&DENO_VERSION, get_deno_version) {
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
fn get_deno_version() -> Option<String> {
    let output = Command::new("deno").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version_str = String::from_utf8_lossy(&output.stdout);
    version_str
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .map(|v| v.to_string())
}

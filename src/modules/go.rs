use crate::error::Result;
use crate::memo::{GO_VERSION, memoized_version};
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
    fn fs_markers(&self) -> &'static [&'static str] {
        &["go.mod"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if context.marker_path("go.mod").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some(String::new()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "go")?;

        let version = match memoized_version(&GO_VERSION, get_go_version) {
            Some(v) => v,
            None => return Ok(None),
        };
        let version_str = version.as_ref();

        match normalized_format {
            "full" => Ok(Some(version_str.to_string())),
            "short" => Ok(Some(utils::shorten_version(version_str))),
            "major" => Ok(version_str.split('.').next().map(|s| s.to_string())),
            _ => unreachable!("validate_version_format should have caught this"),
        }
    }
}

#[cold]
fn get_go_version() -> Option<String> {
    let output = Command::new("go").arg("version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version_str = String::from_utf8_lossy(&output.stdout);
    version_str
        .split_whitespace()
        .nth(2)
        .map(|v| v.trim_start_matches("go").to_string())
}

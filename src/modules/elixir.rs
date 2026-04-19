use crate::error::Result;
use crate::memo::{ELIXIR_VERSION, memoized_version};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

pub struct ElixirModule;

impl Default for ElixirModule {
    fn default() -> Self {
        Self::new()
    }
}

impl ElixirModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for ElixirModule {
    fn fs_markers(&self) -> &'static [&'static str] {
        &["mix.exs"]
    }

    fn is_blocking(&self) -> bool {
        true
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if context.marker_path("mix.exs").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some(String::new()));
        }

        let normalized_format = utils::validate_version_format(format, "elixir")?;

        let version = match memoized_version(&ELIXIR_VERSION, get_elixir_version) {
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
fn get_elixir_version() -> Option<String> {
    let output = Command::new("elixir").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("Elixir ") {
            return Some(rest.split_whitespace().next()?.to_string());
        }
    }
    None
}

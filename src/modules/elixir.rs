use crate::cache::VERSION_CACHE;
use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;
use std::time::Duration;

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

#[cold]
fn get_elixir_version() -> Option<String> {
    let output = Command::new("elixir").arg("--version").output().ok()?;

    if !output.status.success() {
        return None;
    }

    let version_str = String::from_utf8_lossy(&output.stdout);

    for line in version_str.lines() {
        if line.starts_with("Elixir ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return Some(parts[1].to_string());
            }
        }
    }

    None
}

impl Module for ElixirModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("mix.exs").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("elixir".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "elixir")?;

        // Check cache first
        let cache_key = "elixir_version";
        let version = if let Some(cached) = VERSION_CACHE.get(cache_key) {
            match cached {
                Some(v) => v,
                None => return Ok(None),
            }
        } else {
            // Get version with timeout consideration
            let version = get_elixir_version();
            VERSION_CACHE.insert(
                cache_key.to_string(),
                version.clone(),
                Duration::from_secs(300),
            );
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

use crate::error::Result;
use crate::memo::{PYTHON_VERSION, memoized_version};
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
    fn fs_markers(&self) -> &'static [&'static str] {
        &["requirements.txt", "pyproject.toml", "setup.py"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let has_marker = ["requirements.txt", "pyproject.toml", "setup.py"]
            .into_iter()
            .any(|marker| context.marker_path(marker).is_some());
        if !has_marker {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some(String::new()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "python")?;

        let version = match memoized_version(&PYTHON_VERSION, get_python_version) {
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
fn get_python_version() -> Option<String> {
    let output = Command::new("python3")
        .arg("--version")
        .output()
        .or_else(|_| Command::new("python").arg("--version").output())
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let version_bytes = if output.stdout.is_empty() {
        output.stderr.as_slice()
    } else {
        output.stdout.as_slice()
    };
    let version_str = String::from_utf8_lossy(version_bytes);
    version_str.split_whitespace().nth(1).map(|v| v.to_string())
}

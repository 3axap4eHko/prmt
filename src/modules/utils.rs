use crate::error::{PromptError, Result};
use std::env;
use std::path::{Path, PathBuf};

pub fn find_upward(name: &str) -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;
    find_upward_from(&current_dir, name)
}

pub fn find_upward_from(start_dir: &Path, name: &str) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();

    loop {
        let potential = current.join(name);
        if potential.exists() {
            return Some(potential);
        }

        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

pub fn validate_version_format<'a>(format: &'a str, module_name: &str) -> Result<&'a str> {
    match format {
        "" | "full" | "f" => Ok("full"),
        "short" | "s" => Ok("short"),
        "major" | "m" => Ok("major"),
        _ => Err(PromptError::InvalidFormat {
            module: module_name.to_string(),
            format: format.to_string(),
            valid_formats: "full, f, short, s, major, m".to_string(),
        }),
    }
}

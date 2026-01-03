use crate::error::{PromptError, Result};

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

pub fn shorten_version(version: &str) -> String {
    if let Some((major, rest)) = version.split_once('.')
        && let Some(minor) = rest.split('.').next()
    {
        format!("{major}.{minor}")
    } else {
        version.to_string()
    }
}

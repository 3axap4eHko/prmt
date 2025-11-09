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

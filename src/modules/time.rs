use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use chrono::Local;

pub struct TimeModule;

impl Default for TimeModule {
    fn default() -> Self {
        Self
    }
}

impl Module for TimeModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        let now = Local::now();

        let formatted = match format {
            "" | "24h" => now.format("%H:%M"),
            "12h" | "12H" => now.format("%I:%M%p"),
            "12hs" | "12HS" => now.format("%I:%M:%S%p"),
            "24hs" | "24HS" => now.format("%H:%M:%S"),
            _ => {
                return Err(PromptError::InvalidFormat {
                    module: "time".to_string(),
                    format: format.to_string(),
                    valid_formats: "24h (default), 12h, 12H, 12hs, 12HS, 24hs, 24HS".to_string(),
                });
            }
        };

        Ok(Some(formatted.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    #[test]
    fn test_time_module_default_format() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let result = module.render("", &context).unwrap();
        assert!(result.is_some());
        let time = result.unwrap();
        assert_eq!(time.len(), 5);
        assert!(time.contains(':'));

        let re = Regex::new(r"^\d{2}:\d{2}$").unwrap();
        assert!(re.is_match(&time), "Expected HH:MM format, got: {}", time);
    }

    #[test]
    fn test_time_module_24h_format() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let result = module.render("24h", &context).unwrap();
        assert!(result.is_some());
        let time = result.unwrap();
        assert_eq!(time.len(), 5);

        let re = Regex::new(r"^\d{2}:\d{2}$").unwrap();
        assert!(re.is_match(&time), "Expected HH:MM format, got: {}", time);
    }

    #[test]
    fn test_time_module_24hs_format() {
        let module = TimeModule;
        let context = ModuleContext::default();

        for format in &["24hs", "24HS"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();
            assert_eq!(time.len(), 8);

            let re = Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
            assert!(
                re.is_match(&time),
                "Expected HH:MM:SS format for {}, got: {}",
                format,
                time
            );
        }
    }

    #[test]
    fn test_time_module_12h_format() {
        let module = TimeModule;
        let context = ModuleContext::default();

        for format in &["12h", "12H"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();

            let re = Regex::new(r"^\d{2}:\d{2}(AM|PM)$").unwrap();
            assert!(
                re.is_match(&time),
                "Expected hh:MMAM/PM format for {}, got: {}",
                format,
                time
            );

            assert!(time.ends_with("AM") || time.ends_with("PM"));
        }
    }

    #[test]
    fn test_time_module_12hs_format() {
        let module = TimeModule;
        let context = ModuleContext::default();

        for format in &["12hs", "12HS"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();

            let re = Regex::new(r"^\d{2}:\d{2}:\d{2}(AM|PM)$").unwrap();
            assert!(
                re.is_match(&time),
                "Expected hh:MM:SSAM/PM format for {}, got: {}",
                format,
                time
            );

            assert!(time.ends_with("AM") || time.ends_with("PM"));
        }
    }

    #[test]
    fn test_time_module_unknown_format_returns_error() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let unknown_formats = vec!["invalid", "xyz", "13h", "25h", "random"];

        for format in unknown_formats {
            let result = module.render(format, &context);
            assert!(
                result.is_err(),
                "Unknown format '{}' should return error",
                format
            );
        }
    }

    #[test]
    fn test_time_module_valid_and_invalid_formats() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let valid_formats = vec!["", "24h", "24hs", "24HS", "12h", "12H", "12hs", "12HS"];
        let invalid_formats = vec!["invalid", "test", "13h", "random"];

        for format in valid_formats {
            let result = module.render(format, &context);
            assert!(result.is_ok(), "Valid format '{}' should succeed", format);
            let value = result.unwrap();
            assert!(
                value.is_some(),
                "Time module should return Some for valid format: {}",
                format
            );
        }

        for format in invalid_formats {
            let result = module.render(format, &context);
            assert!(
                result.is_err(),
                "Invalid format '{}' should return error",
                format
            );
        }
    }

    #[test]
    fn test_time_module_hour_range() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let result_24h = module.render("24h", &context).unwrap();
        assert!(result_24h.is_some());
        let time_24h = result_24h.unwrap();
        let hour = &time_24h[0..2].parse::<u32>().unwrap();
        assert!(*hour <= 23, "24h format hour should be 0-23, got: {}", hour);

        let result_12h = module.render("12h", &context).unwrap();
        assert!(result_12h.is_some());
        let time_12h = result_12h.unwrap();
        let hour = &time_12h[0..2].parse::<u32>().unwrap();
        assert!(
            *hour >= 1 && *hour <= 12,
            "12h format hour should be 1-12, got: {}",
            hour
        );
    }
}

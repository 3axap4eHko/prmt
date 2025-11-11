use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use libc::c_int;
use std::convert::TryInto;
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TimeModule;

impl Default for TimeModule {
    fn default() -> Self {
        Self
    }
}

enum FormatSpec {
    Hm24,
    Hms24,
    Hm12,
    Hms12,
}

impl FormatSpec {
    fn render(&self, parts: &TimeParts) -> String {
        match self {
            FormatSpec::Hm24 => format!("{:02}:{:02}", parts.hour24, parts.minute),
            FormatSpec::Hms24 => format!(
                "{:02}:{:02}:{:02}",
                parts.hour24, parts.minute, parts.second
            ),
            FormatSpec::Hm12 => {
                let (hour, suffix) = parts.hour12();
                format!("{:02}:{:02}{suffix}", hour, parts.minute)
            }
            FormatSpec::Hms12 => {
                let (hour, suffix) = parts.hour12();
                format!(
                    "{:02}:{:02}:{:02}{suffix}",
                    hour, parts.minute, parts.second
                )
            }
        }
    }
}

impl Module for TimeModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        let spec = match format {
            "" | "24h" => FormatSpec::Hm24,
            "24hs" | "24HS" => FormatSpec::Hms24,
            "12h" | "12H" => FormatSpec::Hm12,
            "12hs" | "12HS" => FormatSpec::Hms12,
            _ => {
                return Err(PromptError::InvalidFormat {
                    module: "time".to_string(),
                    format: format.to_string(),
                    valid_formats: "24h (default), 12h, 12H, 12hs, 12HS, 24hs, 24HS".to_string(),
                });
            }
        };

        let parts = current_local_time()?;
        Ok(Some(spec.render(&parts)))
    }
}

#[derive(Clone, Copy)]
struct TimeParts {
    hour24: u8,
    minute: u8,
    second: u8,
}

impl TimeParts {
    fn hour12(&self) -> (u8, &'static str) {
        let suffix = if self.hour24 >= 12 { "PM" } else { "AM" };
        let mut hour = self.hour24 % 12;
        if hour == 0 {
            hour = 12;
        }
        (hour, suffix)
    }
}

fn current_local_time() -> Result<TimeParts> {
    let timestamp = system_time_to_time_t()?;
    let tm = platform_local_tm(timestamp)?;
    Ok(TimeParts {
        hour24: clamp_component(tm.tm_hour, 23),
        minute: clamp_component(tm.tm_min, 59),
        second: clamp_component(tm.tm_sec, 60),
    })
}

fn system_time_to_time_t() -> Result<libc::time_t> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| PromptError::IoError(io::Error::other(err)))?;

    duration
        .as_secs()
        .try_into()
        .map_err(|err| PromptError::IoError(io::Error::other(err)))
}

fn clamp_component(value: c_int, max: u8) -> u8 {
    value.clamp(0, max as c_int) as u8
}

#[cfg(unix)]
fn platform_local_tm(timestamp: libc::time_t) -> Result<libc::tm> {
    use std::mem::MaybeUninit;

    unsafe {
        let mut tm = MaybeUninit::<libc::tm>::uninit();
        if libc::localtime_r(&timestamp as *const _, tm.as_mut_ptr()).is_null() {
            return Err(PromptError::IoError(io::Error::last_os_error()));
        }
        Ok(tm.assume_init())
    }
}

#[cfg(windows)]
fn platform_local_tm(timestamp: libc::time_t) -> Result<libc::tm> {
    use std::mem::MaybeUninit;

    unsafe {
        let mut tm = MaybeUninit::<libc::tm>::uninit();
        let err = libc::localtime_s(tm.as_mut_ptr(), &timestamp as *const _);
        if err != 0 {
            return Err(PromptError::IoError(io::Error::from_raw_os_error(err)));
        }
        Ok(tm.assume_init())
    }
}

#[cfg(not(any(unix, windows)))]
fn platform_local_tm(_timestamp: libc::time_t) -> Result<libc::tm> {
    Err(PromptError::IoError(io::Error::new(
        io::ErrorKind::Other,
        "time module is not supported on this platform",
    )))
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

        let re = Regex::new(r"^\d{2}:\d{2}:\d{2}$").unwrap();
        for format in &["24hs", "24HS"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();
            assert_eq!(time.len(), 8);

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

        let re = Regex::new(r"^\d{2}:\d{2}(AM|PM)$").unwrap();
        for format in &["12h", "12H"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();

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

        let re = Regex::new(r"^\d{2}:\d{2}:\d{2}(AM|PM)$").unwrap();
        for format in &["12hs", "12HS"] {
            let result = module.render(format, &context).unwrap();
            assert!(result.is_some());
            let time = result.unwrap();

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

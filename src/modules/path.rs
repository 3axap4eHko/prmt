use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use std::env;
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub struct PathModule;

impl Default for PathModule {
    fn default() -> Self {
        Self::new()
    }
}

impl PathModule {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
fn normalize_separators(value: String) -> String {
    value.replace('\\', "/")
}

#[cfg(not(target_os = "windows"))]
fn normalize_separators(value: String) -> String {
    value
}

fn normalize_relative_path(current_dir: &Path) -> String {
    let current_canon = current_dir
        .canonicalize()
        .unwrap_or_else(|_| current_dir.to_path_buf());

    if let Some(home) = dirs::home_dir() {
        let home_canon = home.canonicalize().unwrap_or(home);
        if let Ok(stripped) = current_canon.strip_prefix(&home_canon) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }

            let mut result = String::from("~");
            result.push(std::path::MAIN_SEPARATOR);
            result.push_str(&stripped.to_string_lossy());
            return normalize_separators(result);
        }
    }

    normalize_separators(current_dir.to_string_lossy().to_string())
}

fn strip_vowels(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    let mut segment_start = true;

    for ch in path.chars() {
        if ch == '/' {
            segment_start = true;
            result.push(ch);
            continue;
        }

        if segment_start {
            segment_start = false;
            result.push(ch);
            continue;
        }

        let lower = ch.to_ascii_lowercase();
        if matches!(lower, 'a' | 'e' | 'i' | 'o' | 'u') {
            continue;
        }

        result.push(ch);
    }

    result
}

impl Module for PathModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        let current_dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => return Ok(None),
        };

        match format {
            "" | "relative" | "r" => Ok(Some(normalize_relative_path(&current_dir))),
            "absolute" | "a" | "f" => Ok(Some(current_dir.to_string_lossy().to_string())),
            "strip" | "stripped" => Ok(Some(strip_vowels(&normalize_relative_path(&current_dir)))),
            "short" | "s" => Ok(current_dir
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string())
                .or_else(|| Some(".".to_string()))),
            format if format.starts_with("truncate:") => {
                let max_width: usize = format
                    .strip_prefix("truncate:")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(30);

                let path = normalize_relative_path(&current_dir);

                // Use unicode width for proper truncation
                let width = UnicodeWidthStr::width(path.as_str());
                if width <= max_width {
                    Ok(Some(path))
                } else {
                    // Truncate with ellipsis
                    let ellipsis = "...";
                    let ellipsis_width = 3;
                    let target_width = max_width.saturating_sub(ellipsis_width);

                    let mut truncated = String::new();
                    let mut current_width = 0;

                    for ch in path.chars() {
                        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
                        if current_width + ch_width > target_width {
                            break;
                        }
                        truncated.push(ch);
                        current_width += ch_width;
                    }

                    truncated.push_str(ellipsis);
                    Ok(Some(truncated))
                }
            }
            _ => Err(PromptError::InvalidFormat {
                module: "path".to_string(),
                format: format.to_string(),
                valid_formats: "relative, r, absolute, a, f, short, s, strip, stripped, truncate:N"
                    .to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct DirGuard {
        original: std::path::PathBuf,
    }

    impl DirGuard {
        fn change_to(path: &Path) -> Self {
            let original = env::current_dir().expect("current dir");
            env::set_current_dir(path).expect("change current dir");
            Self { original }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    fn unique_name() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos()
            .to_string()
    }

    #[test]
    #[serial]
    fn relative_path_inside_home_renders_tilde() {
        let module = PathModule::new();
        let home = dirs::home_dir().expect("home dir should exist");
        let project = home.join(format!("prmt_test_project_{}", unique_name()));
        match fs::create_dir_all(&project) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping test: {}", err);
                return;
            }
            Err(err) => panic!("create project dir: {}", err),
        }

        let _dir_guard = DirGuard::change_to(&project);

        let value = module
            .render("", &ModuleContext::default())
            .expect("render")
            .expect("some");

        assert!(
            value.starts_with("~/prmt_test_project_"),
            "Expected path to start with ~/prmt_test_project_, got: {}",
            value
        );

        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    #[serial]
    fn relative_path_with_shared_prefix_is_not_tilde() {
        let module = PathModule::new();
        let home = dirs::home_dir().expect("home dir should exist");

        let unique = unique_name();
        let base = home.join(format!("prmt_test_base_{}", unique));
        let home_like = base.join("al");
        let similar = base.join("alpine");

        match fs::create_dir_all(&home_like) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping test: {}", err);
                return;
            }
            Err(err) => panic!("create home_like: {}", err),
        }
        match fs::create_dir_all(&similar) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                eprintln!("Skipping test: {}", err);
                return;
            }
            Err(err) => panic!("create similar: {}", err),
        }

        let _dir_guard = DirGuard::change_to(&similar);

        let value = module
            .render("", &ModuleContext::default())
            .expect("render")
            .expect("some");

        assert!(
            value.starts_with("~/prmt_test_base_"),
            "Expected path to start with ~/prmt_test_base_, got: {}",
            value
        );
        assert!(
            value.ends_with("/alpine"),
            "Expected path to end with /alpine, got: {}",
            value
        );

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn strip_vowels_helper_removes_vowels_after_first_char() {
        assert_eq!(strip_vowels("~/project"), "~/prjct");
        assert_eq!(strip_vowels("/alpha/beta"), "/alph/bt");
        assert_eq!(strip_vowels("simple"), "smpl");
    }
}

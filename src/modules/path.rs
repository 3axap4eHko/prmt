use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use std::env;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

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
    if let Some(home) = dirs::home_dir()
        && let Ok(stripped) = current_dir.strip_prefix(&home)
    {
        if stripped.as_os_str().is_empty() {
            return "~".to_string();
        }

        let mut result = String::from("~");
        result.push(std::path::MAIN_SEPARATOR);
        result.push_str(&stripped.to_string_lossy());
        return normalize_separators(result);
    }

    normalize_separators(current_dir.to_string_lossy().to_string())
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
                        let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
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
                valid_formats: "relative, r, absolute, a, f, short, s, truncate:N".to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::ffi::OsString;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct HomeEnvGuard {
        home: Option<OsString>,
        #[cfg(windows)]
        userprofile: Option<OsString>,
    }

    impl HomeEnvGuard {
        fn set(path: &Path) -> Self {
            let home = env::var_os("HOME");
            unsafe {
                env::set_var("HOME", path);
            }

            #[cfg(windows)]
            {
                let userprofile = env::var_os("USERPROFILE");
                unsafe {
                    env::set_var("USERPROFILE", path);
                }
                Self { home, userprofile }
            }

            #[cfg(not(windows))]
            {
                Self { home }
            }
        }
    }

    impl Drop for HomeEnvGuard {
        fn drop(&mut self) {
            match &self.home {
                Some(val) => unsafe {
                    env::set_var("HOME", val);
                },
                None => unsafe {
                    env::remove_var("HOME");
                },
            }

            #[cfg(windows)]
            match &self.userprofile {
                Some(val) => unsafe {
                    env::set_var("USERPROFILE", val);
                },
                None => unsafe {
                    env::remove_var("USERPROFILE");
                },
            }
        }
    }

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

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let dir = env::temp_dir().join(format!("prmt_path_test_{label}_{unique}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn normalize_expected(path: &Path) -> String {
        let as_string = path.to_string_lossy().to_string();
        #[cfg(windows)]
        return as_string.replace('\\', "/");
        #[cfg(not(windows))]
        return as_string;
    }

    #[test]
    #[serial]
    fn relative_path_inside_home_renders_tilde() {
        let module = PathModule::new();
        let home = temp_dir("home");
        let project = home.join("project");
        fs::create_dir_all(&project).expect("create project dir");

        let _home_guard = HomeEnvGuard::set(&home);
        let dir_guard = DirGuard::change_to(&project);

        let value = module
            .render("", &ModuleContext::default())
            .expect("render")
            .expect("some");

        drop(dir_guard);

        assert_eq!(value, "~/project");

        let _ = fs::remove_dir_all(&home);
    }

    #[test]
    #[serial]
    fn relative_path_with_shared_prefix_is_not_tilde() {
        let module = PathModule::new();
        let base = temp_dir("base");
        let home = base.join("al");
        let similar = base.join("alpine");
        fs::create_dir_all(&home).expect("create home");
        fs::create_dir_all(&similar).expect("create similar");

        let _home_guard = HomeEnvGuard::set(&home);
        let dir_guard = DirGuard::change_to(&similar);

        let value = module
            .render("", &ModuleContext::default())
            .expect("render")
            .expect("some");

        drop(dir_guard);

        assert_eq!(value, normalize_expected(&similar));

        let _ = fs::remove_dir_all(&base);
    }
}

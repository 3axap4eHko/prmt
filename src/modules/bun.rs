use crate::error::Result;
use crate::memo::{BUN_VERSION, memoized_version};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use std::process::Command;

const BUN_MARKERS: &[&str] = &["bun.lock", "bun.lockb", "bunfig.toml"];

pub struct BunModule;

impl Default for BunModule {
    fn default() -> Self {
        Self::new()
    }
}

impl BunModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for BunModule {
    fn fs_markers(&self) -> &'static [&'static str] {
        BUN_MARKERS
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let has_marker = BUN_MARKERS
            .iter()
            .copied()
            .any(|marker| context.marker_path(marker).is_some());
        if !has_marker {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some(String::new()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "bun")?;

        let version = match memoized_version(&BUN_VERSION, get_bun_version) {
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
fn get_bun_version() -> Option<String> {
    let output = Command::new("bun").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::detector::detect;
    use serial_test::serial;
    use std::collections::HashSet;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    struct DirGuard {
        original: PathBuf,
    }

    impl DirGuard {
        fn enter(path: &Path) -> Self {
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

    #[test]
    fn fs_markers_include_current_and_legacy_lockfiles() {
        let module = BunModule::new();

        assert!(module.fs_markers().contains(&"bun.lock"));
        assert!(module.fs_markers().contains(&"bun.lockb"));
        assert!(module.fs_markers().contains(&"bunfig.toml"));
    }

    #[test]
    #[serial]
    fn bun_lock_activates_module_in_no_version_mode() {
        let module = BunModule::new();
        let tmp = tempdir().expect("tempdir");
        fs::write(tmp.path().join("bun.lock"), "").expect("create bun.lock");
        let _guard = DirGuard::enter(tmp.path());
        let required: HashSet<&'static str> = module.fs_markers().iter().copied().collect();
        let context = ModuleContext {
            no_version: true,
            detection: detect(&required),
            ..ModuleContext::default()
        };

        let result = module.render("", &context).expect("render");

        assert_eq!(result, Some(String::new()));
    }
}

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const MAX_TRAVERSAL_DEPTH: usize = 64;

#[derive(Debug, Clone)]
pub struct DetectionContext {
    markers: Arc<HashMap<&'static str, PathBuf>>,
}

impl DetectionContext {
    pub fn empty() -> Self {
        Self {
            markers: Arc::new(HashMap::new()),
        }
    }

    pub fn get(&self, name: &str) -> Option<&Path> {
        self.markers.get(name).map(|path| path.as_path())
    }
}

impl Default for DetectionContext {
    fn default() -> Self {
        DetectionContext::empty()
    }
}

pub fn detect(required: &HashSet<&'static str>) -> DetectionContext {
    if required.is_empty() {
        return DetectionContext::empty();
    }

    let Ok(mut current_dir) = env::current_dir() else {
        return DetectionContext::empty();
    };

    let mut found: HashMap<&'static str, PathBuf> = HashMap::with_capacity(required.len());
    let mut depth = 0usize;
    let mut candidate = PathBuf::new();

    loop {
        for &marker in required {
            match found.entry(marker) {
                Entry::Occupied(_) => continue,
                Entry::Vacant(slot) => {
                    candidate.clear();
                    candidate.push(&current_dir);
                    candidate.push(marker);
                    if let Ok(true) = candidate.try_exists() {
                        slot.insert(candidate.clone());
                    }
                }
            }
        }

        if found.len() == required.len() {
            break;
        }

        if depth >= MAX_TRAVERSAL_DEPTH {
            break;
        }

        if !current_dir.pop() {
            break;
        }
        depth += 1;
    }

    DetectionContext {
        markers: Arc::new(found),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    struct DirGuard {
        original: PathBuf,
    }

    impl DirGuard {
        fn enter(path: &Path) -> Self {
            let original = env::current_dir().unwrap();
            env::set_current_dir(path).unwrap();
            Self { original }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = env::set_current_dir(&self.original);
        }
    }

    #[test]
    fn detect_returns_empty_for_no_requirements() {
        let ctx = detect(&HashSet::new());
        assert!(ctx.get("Cargo.toml").is_none());
    }

    #[test]
    #[serial]
    fn detect_finds_markers_up_the_tree() {
        let tmp = tempdir().unwrap();
        let project = tmp.path().join("project");
        let nested = project.join("src/bin");
        fs::create_dir_all(&nested).unwrap();
        fs::write(project.join("Cargo.toml"), b"[package]").unwrap();
        fs::create_dir_all(project.join(".git")).unwrap();

        let _guard = DirGuard::enter(&nested);

        let required: HashSet<&'static str> = [".git", "Cargo.toml"].into_iter().collect();
        let ctx = detect(&required);

        let cargo = ctx
            .get("Cargo.toml")
            .expect("detector should find Cargo.toml");
        assert!(
            cargo.ends_with("Cargo.toml"),
            "expected Cargo.toml to be the detected file"
        );
        assert_eq!(
            cargo.parent().and_then(|p| p.file_name()),
            project.file_name()
        );

        let git = ctx.get(".git").expect("detector should find .git");
        assert!(git.ends_with(".git"), "expected .git directory to match");
        assert_eq!(
            git.parent().and_then(|p| p.file_name()),
            project.file_name()
        );
    }

    #[test]
    #[serial]
    fn detect_handles_missing_markers() {
        let tmp = tempdir().unwrap();
        let nested = tmp.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();

        let _guard = DirGuard::enter(&nested);

        let required: HashSet<&'static str> = ["Cargo.toml"].into_iter().collect();
        let ctx = detect(&required);

        assert!(ctx.get("Cargo.toml").is_none());
    }
}

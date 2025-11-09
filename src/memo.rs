use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock};

pub type VersionSlot = OnceLock<Option<Arc<str>>>;

pub static NODE_VERSION: VersionSlot = OnceLock::new();
pub static RUST_VERSION: VersionSlot = OnceLock::new();
pub static PYTHON_VERSION: VersionSlot = OnceLock::new();
pub static GO_VERSION: VersionSlot = OnceLock::new();
pub static DENO_VERSION: VersionSlot = OnceLock::new();
pub static BUN_VERSION: VersionSlot = OnceLock::new();

pub fn memoized_version<F>(slot: &VersionSlot, fetch: F) -> Option<Arc<str>>
where
    F: FnOnce() -> Option<String>,
{
    if let Some(value) = slot.get() {
        return value.clone();
    }

    let value = fetch().map(|v| Arc::<str>::from(v.into_boxed_str()));
    let _ = slot.set(value.clone());
    value
}

/// Per-process memoization for Git metadata gathered during a render.
pub struct GitMemo {
    entries: RwLock<HashMap<PathBuf, GitInfo>>,
}

#[derive(Clone)]
pub struct GitInfo {
    pub branch: String,
    pub has_changes: bool,
    pub has_staged: bool,
    pub has_untracked: bool,
}

impl Default for GitMemo {
    fn default() -> Self {
        Self::new()
    }
}

impl GitMemo {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, path: &Path) -> Option<GitInfo> {
        let entries = self.entries.read().ok()?;
        entries.get(path).cloned()
    }

    pub fn insert(&self, path: PathBuf, info: GitInfo) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(path, info);
        }
    }
}

pub static GIT_MEMO: Lazy<GitMemo> = Lazy::new(GitMemo::new);

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn memoized_version_caches_successful_fetches() {
        let slot: VersionSlot = OnceLock::new();
        let calls = AtomicUsize::new(0);
        let value = memoized_version(&slot, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Some("1.2.3".to_string())
        })
        .expect("expected version");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(value.as_ref(), "1.2.3");

        let second = memoized_version(&slot, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Some("should not run".to_string())
        })
        .expect("expected cached version");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(Arc::ptr_eq(&value, &second));
    }

    #[test]
    fn memoized_version_caches_absence() {
        let slot: VersionSlot = OnceLock::new();
        let calls = AtomicUsize::new(0);
        let value = memoized_version(&slot, || {
            calls.fetch_add(1, Ordering::SeqCst);
            None
        });
        assert!(value.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let second = memoized_version(&slot, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Some("unexpected".to_string())
        });
        assert!(second.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

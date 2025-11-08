use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub struct VersionCache {
    entries: RwLock<HashMap<String, Option<String>>>,
}

impl Default for VersionCache {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionCache {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(&self, key: &str) -> Option<Option<String>> {
        let entries = self.entries.read().ok()?;
        entries.get(key).cloned()
    }

    pub fn insert(&self, key: String, value: Option<String>) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(key, value);
        }
    }
}

pub static VERSION_CACHE: Lazy<VersionCache> = Lazy::new(VersionCache::new);

pub struct GitCache {
    entries: RwLock<HashMap<PathBuf, GitInfo>>,
}

#[derive(Clone)]
pub struct GitInfo {
    pub branch: String,
    pub has_changes: bool,
    pub has_staged: bool,
    pub has_untracked: bool,
}

impl Default for GitCache {
    fn default() -> Self {
        Self::new()
    }
}

impl GitCache {
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

pub static GIT_CACHE: Lazy<GitCache> = Lazy::new(GitCache::new);

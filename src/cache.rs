use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub struct CacheEntry<T> {
    value: T,
    timestamp: Instant,
    ttl: Duration,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            ttl,
        }
    }
    
    fn is_valid(&self) -> bool {
        self.timestamp.elapsed() < self.ttl
    }
    
    fn get(&self) -> Option<T> {
        if self.is_valid() {
            Some(self.value.clone())
        } else {
            None
        }
    }
}

pub struct VersionCache {
    entries: RwLock<HashMap<String, CacheEntry<Option<String>>>>,
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
        entries.get(key)?.get()
    }
    
    pub fn insert(&self, key: String, value: Option<String>, ttl: Duration) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(key, CacheEntry::new(value, ttl));
        }
    }
}

pub static VERSION_CACHE: Lazy<VersionCache> = Lazy::new(VersionCache::new);

pub struct GitCache {
    entries: RwLock<HashMap<PathBuf, CacheEntry<GitInfo>>>,
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
        entries.get(path)?.get()
    }
    
    pub fn insert(&self, path: PathBuf, info: GitInfo) {
        if let Ok(mut entries) = self.entries.write() {
            // Git info cached for 1 second (frequent prompt refreshes)
            entries.insert(path, CacheEntry::new(info, Duration::from_millis(1000)));
        }
    }
}

pub static GIT_CACHE: Lazy<GitCache> = Lazy::new(GitCache::new);


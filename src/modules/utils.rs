use std::env;
use std::path::{Path, PathBuf};

pub fn find_upward(name: &str) -> Option<PathBuf> {
    let current_dir = env::current_dir().ok()?;
    find_upward_from(&current_dir, name)
}

pub fn find_upward_from(start_dir: &Path, name: &str) -> Option<PathBuf> {
    let mut current = start_dir.to_path_buf();
    
    loop {
        let potential = current.join(name);
        if potential.exists() {
            return Some(potential);
        }
        
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}
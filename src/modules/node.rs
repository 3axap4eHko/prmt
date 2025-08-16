use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use crate::cache::VERSION_CACHE;
use std::process::Command;
use std::time::Duration;

pub struct NodeModule;

impl Default for NodeModule {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeModule {
    pub fn new() -> Self {
        Self
    }
}

#[cold]
fn get_node_version() -> Option<String> {
    let output = Command::new("node")
        .arg("--version")
        .output()
        .ok()?;
    
    if !output.status.success() {
        return None;
    }
    
    let version_str = String::from_utf8_lossy(&output.stdout);
    Some(version_str
        .trim()
        .trim_start_matches('v')
        .to_string())
}

impl Module for NodeModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        utils::find_upward("package.json")?;
        
        if context.no_version {
            return Some("node".to_string());
        }
        
        // Check cache first
        let cache_key = "node_version";
        let version = if let Some(cached) = VERSION_CACHE.get(cache_key) {
            cached?
        } else {
            let version = get_node_version();
            VERSION_CACHE.insert(cache_key.to_string(), version.clone(), Duration::from_secs(300));
            version?
        };
        
        match format {
            "" | "full" => Some(version),
            "short" => {
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 2 {
                    Some(format!("{}.{}", parts[0], parts[1]))
                } else {
                    Some(version)
                }
            }
            "major" => {
                version.split('.').next().map(|s| s.to_string())
            }
            _ => None,
        }
    }
}
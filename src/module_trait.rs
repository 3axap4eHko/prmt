use crate::detector::DetectionContext;
use crate::error::Result;
use crate::style::Shell;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModuleContext {
    pub no_version: bool,
    pub exit_code: Option<i32>,
    pub detection: DetectionContext,
    pub shell: Shell,
    pub stdin_data: Option<Arc<serde_json::Value>>,
    pub cwd: Option<PathBuf>,
}

impl Default for ModuleContext {
    fn default() -> Self {
        Self {
            no_version: false,
            exit_code: None,
            detection: DetectionContext::default(),
            shell: Shell::None,
            stdin_data: None,
            cwd: env::current_dir().ok(),
        }
    }
}

impl ModuleContext {
    pub fn marker_path(&self, marker: &str) -> Option<&Path> {
        self.detection.get(marker)
    }

    pub fn current_dir(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

pub trait Module: Send + Sync {
    fn fs_markers(&self) -> &'static [&'static str] {
        &[]
    }

    fn is_blocking(&self) -> bool {
        false
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>>;
}

pub type ModuleRef = Arc<dyn Module>;

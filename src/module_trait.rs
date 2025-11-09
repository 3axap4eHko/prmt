use crate::detector::DetectionContext;
use crate::error::Result;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct ModuleContext {
    pub no_version: bool,
    pub exit_code: Option<i32>,
    pub detection: DetectionContext,
}

impl ModuleContext {
    pub fn marker_path(&self, marker: &str) -> Option<&Path> {
        self.detection.get(marker)
    }
}

pub trait Module: Send + Sync {
    fn fs_markers(&self) -> &'static [&'static str] {
        &[]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>>;
}

pub type ModuleRef = Arc<dyn Module>;

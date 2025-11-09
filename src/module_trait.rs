use crate::error::Result;
use crate::style::Shell;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct ModuleContext {
    pub no_version: bool,
    pub exit_code: Option<i32>,
    pub shell: Shell,
}

pub trait Module: Send + Sync {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>>;
}

pub type ModuleRef = Arc<dyn Module>;

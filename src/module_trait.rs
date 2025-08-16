use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModuleContext {
    pub no_version: bool,
    pub exit_code: Option<i32>,
}

impl Default for ModuleContext {
    fn default() -> Self {
        Self {
            no_version: false,
            exit_code: None,
        }
    }
}

pub trait Module: Send + Sync {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String>;
}

pub type ModuleRef = Arc<dyn Module>;
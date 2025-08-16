use std::sync::Arc;

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct ModuleContext {
    pub no_version: bool,
    pub exit_code: Option<i32>,
}


pub trait Module: Send + Sync {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String>;
}

pub type ModuleRef = Arc<dyn Module>;
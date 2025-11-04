use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use whoami::username;


pub struct WhoamiModule;

impl Default for WhoamiModule {
    fn default() -> Self {
        Self::new()
    }
}

impl WhoamiModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for WhoamiModule {
    #[allow(unused)]
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        Ok(Some(username()))
    }
}

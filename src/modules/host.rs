use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use whoami::fallible::hostname;


pub struct HostModule;

impl Default for HostModule {
    fn default() -> Self {
        Self::new()
    }
}

impl HostModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for HostModule {
    #[allow(unused)]
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        Ok(hostname().ok())
    }
}



use std::collections::HashMap;
use crate::module_trait::ModuleRef;

pub struct ModuleRegistry {
    modules: HashMap<String, ModuleRef>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }
    
    pub fn register(&mut self, name: impl Into<String>, module: ModuleRef) {
        self.modules.insert(name.into(), module);
    }
    
    pub fn get(&self, name: &str) -> Option<ModuleRef> {
        self.modules.get(name).cloned()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
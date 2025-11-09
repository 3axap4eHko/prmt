use crate::module_trait::ModuleRef;
use std::collections::{HashMap, HashSet};

struct ModuleEntry {
    module: ModuleRef,
    markers: &'static [&'static str],
}

pub struct ModuleRegistry {
    modules: HashMap<String, ModuleEntry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: impl Into<String>, module: ModuleRef) {
        let markers = module.fs_markers();
        self.modules
            .insert(name.into(), ModuleEntry { module, markers });
    }

    pub fn get(&self, name: &str) -> Option<ModuleRef> {
        self.modules.get(name).map(|entry| entry.module.clone())
    }

    pub fn required_markers(&self) -> HashSet<&'static str> {
        let estimated = self.modules.values().map(|entry| entry.markers.len()).sum();
        let mut markers = HashSet::with_capacity(estimated);
        for entry in self.modules.values() {
            for &marker in entry.markers {
                markers.insert(marker);
            }
        }
        markers
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#![no_main]
use libfuzzer_sys::fuzz_target;
use prmt::detector::DetectionContext;
use prmt::style::Shell;
use prmt::{ModuleContext, ModuleRegistry, Template};
use std::sync::Arc;

fn setup_registry() -> ModuleRegistry {
    use prmt::modules::*;
    
    let mut registry = ModuleRegistry::new();
    registry.register("path", Arc::new(path::PathModule));
    registry.register("git", Arc::new(git::GitModule));
    registry
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Fuzz the template renderer with arbitrary UTF-8 input
        let template = Template::new(s);
        let registry = setup_registry();
        let context = ModuleContext {
            no_version: true,
            exit_code: Some(0),
            detection: DetectionContext::default(),
            shell: Shell::None,
        };
        
        let _ = template.render(&registry, &context);
    }
});

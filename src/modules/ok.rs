use crate::module_trait::{Module, ModuleContext};

pub struct OkModule;

impl Default for OkModule {
    fn default() -> Self {
        Self::new()
    }
}

impl OkModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for OkModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        if context.exit_code != Some(0) {
            return None;
        }
        
        let symbol = match format {
            "" => "❯",
            "code" => "0",
            custom => custom,
        };
        
        Some(symbol.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ok_on_zero_exit_code() {
        let module = OkModule::new();
        let context = ModuleContext {
            exit_code: Some(0),
            no_version: false,
        };
        let result = module.render("", &context);
        assert_eq!(result, Some("❯".to_string()));
    }
    
    #[test]
    fn test_ok_hidden_on_error() {
        let module = OkModule::new();
        let context = ModuleContext {
            exit_code: Some(1),
            no_version: false,
        };
        let result = module.render("", &context);
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_ok_custom_symbol() {
        let module = OkModule::new();
        let context = ModuleContext {
            exit_code: Some(0),
            no_version: false,
        };
        let result = module.render("✓", &context);
        assert_eq!(result, Some("✓".to_string()));
    }
    
    #[test]
    fn test_ok_code_format() {
        let module = OkModule::new();
        let context = ModuleContext {
            exit_code: Some(0),
            no_version: false,
        };
        let result = module.render("code", &context);
        assert_eq!(result, Some("0".to_string()));
    }
}

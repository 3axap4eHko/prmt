use crate::module_trait::{Module, ModuleContext};

pub struct FailModule;

impl FailModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for FailModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Option<String> {
        let exit_code = context.exit_code.unwrap_or(0);
        if exit_code == 0 {
            return None;
        }
        
        let symbol = match format {
            "" | "full" => "❯",
            "code" => &exit_code.to_string(),
            custom => custom,
        };
        
        Some(symbol.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_fail_on_non_zero_exit_code() {
        let module = FailModule::new();
        let context = ModuleContext {
            exit_code: Some(127),
            no_version: false,
        };
        let result = module.render("", &context);
        assert_eq!(result, Some("❯".to_string()));
    }
    
    #[test]
    fn test_fail_hidden_on_success() {
        let module = FailModule::new();
        let context = ModuleContext {
            exit_code: Some(0),
            no_version: false,
        };
        let result = module.render("", &context);
        assert_eq!(result, None);
    }
    
    #[test]
    fn test_fail_shows_exit_code() {
        let module = FailModule::new();
        let context = ModuleContext {
            exit_code: Some(42),
            no_version: false,
        };
        let result = module.render("code", &context);
        assert_eq!(result, Some("42".to_string()));
    }
    
    #[test]
    fn test_fail_custom_symbol() {
        let module = FailModule::new();
        let context = ModuleContext {
            exit_code: Some(1),
            no_version: false,
        };
        let result = module.render("✗", &context);
        assert_eq!(result, Some("✗".to_string()));
    }
}
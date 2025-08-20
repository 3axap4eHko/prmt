use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};

pub struct FailModule;

impl Default for FailModule {
    fn default() -> Self {
        Self::new()
    }
}

impl FailModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for FailModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let exit_code = context.exit_code.unwrap_or(0);
        if exit_code == 0 {
            return Ok(None);
        }

        let symbol = match format {
            "" | "full" => "❯".to_string(),
            "code" => exit_code.to_string(),
            custom => custom.to_string(),
        };

        Ok(Some(symbol))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fail_on_non_zero_exit_code() {
        let module = FailModule::new();
        let mut context = ModuleContext::default();
        context.exit_code = Some(127);
        let result = module.render("", &context).unwrap();
        assert_eq!(result, Some("❯".to_string()));
    }

    #[test]
    fn test_fail_hidden_on_success() {
        let module = FailModule::new();
        let mut context = ModuleContext::default();
        context.exit_code = Some(0);
        let result = module.render("", &context).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_fail_shows_exit_code() {
        let module = FailModule::new();
        let mut context = ModuleContext::default();
        context.exit_code = Some(42);
        let result = module.render("code", &context).unwrap();
        assert_eq!(result, Some("42".to_string()));
    }

    #[test]
    fn test_fail_custom_symbol() {
        let module = FailModule::new();
        let mut context = ModuleContext::default();
        context.exit_code = Some(1);
        let result = module.render("✗", &context).unwrap();
        assert_eq!(result, Some("✗".to_string()));
    }
}

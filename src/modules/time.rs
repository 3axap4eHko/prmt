use crate::module_trait::{Module, ModuleContext};
use chrono::Local;

pub struct TimeModule;

impl Default for TimeModule {
    fn default() -> Self {
        Self
    }
}

impl Module for TimeModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Option<String> {
        let now = Local::now();
        
        let formatted = match format {
            "12h" | "12H" => now.format("%I:%M%p"),
            "12hs" | "12HS" => now.format("%I:%M:%S%p"),
            "24hs" | "24HS" => now.format("%H:%M:%S"),
            _ => now.format("%H:%M"),
        };
        
        Some(formatted.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_module_formats() {
        let module = TimeModule;
        let context = ModuleContext::default();

        let result_24h = module.render("24h", &context);
        assert!(result_24h.is_some());
        let time_24h = result_24h.unwrap();
        assert!(time_24h.contains(':'));
        assert_eq!(time_24h.len(), 5);

        let result_12h = module.render("12h", &context);
        assert!(result_12h.is_some());
        let time_12h = result_12h.unwrap();
        assert!(time_12h.contains(':'));
        assert!(time_12h.ends_with("AM") || time_12h.ends_with("PM"));

        let result_default = module.render("", &context);
        assert!(result_default.is_some());
        assert_eq!(result_default.unwrap().len(), 5);
        
        let result_12hs = module.render("12hs", &context);
        assert!(result_12hs.is_some());
        let time_12hs = result_12hs.unwrap();
        assert!(time_12hs.contains(':'));
        assert!(time_12hs.ends_with("AM") || time_12hs.ends_with("PM"));
        
        let result_24hs = module.render("24hs", &context);
        assert!(result_24hs.is_some());
        let time_24hs = result_24hs.unwrap();
        assert!(time_24hs.contains(':'));
        assert_eq!(time_24hs.len(), 8);
    }
}
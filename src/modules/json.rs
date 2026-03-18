use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};

pub struct JsonModule;

impl Default for JsonModule {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for JsonModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if format.is_empty() {
            return Err(PromptError::InvalidFormat {
                module: "json".to_string(),
                format: format.to_string(),
                valid_formats: "Provide a dot-path, e.g., {json::.field.nested}".to_string(),
            });
        }

        let Some(root) = context.stdin_data.as_deref() else {
            return Ok(None);
        };

        let value = resolve_path(root, format);

        match value {
            Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
            Some(serde_json::Value::Null) | None => Ok(None),
            Some(serde_json::Value::Bool(b)) => Ok(Some(b.to_string())),
            Some(serde_json::Value::Number(n)) => Ok(Some(n.to_string())),
            Some(other) => Ok(Some(other.to_string())),
        }
    }
}

fn resolve_path<'a>(root: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let path = path.strip_prefix('.').unwrap_or(path);
    let mut current = root;
    for key in path.split('.') {
        if key.is_empty() {
            continue;
        }
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(key)?;
            }
            serde_json::Value::Array(arr) => {
                let index: usize = key.parse().ok()?;
                current = arr.get(index)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;

    fn ctx(value: serde_json::Value) -> ModuleContext {
        ModuleContext {
            stdin_data: Some(Arc::new(value)),
            ..ModuleContext::default()
        }
    }

    #[test]
    fn resolves_nested_string() {
        let module = JsonModule::new();
        let context = ctx(json!({"model": {"display_name": "Opus"}}));

        let result = module.render(".model.display_name", &context).unwrap();

        assert_eq!(result, Some("Opus".to_string()));
    }

    #[test]
    fn resolves_number() {
        let module = JsonModule::new();
        let context = ctx(json!({"context_window": {"used_percentage": 42}}));

        let result = module
            .render(".context_window.used_percentage", &context)
            .unwrap();

        assert_eq!(result, Some("42".to_string()));
    }

    #[test]
    fn resolves_boolean() {
        let module = JsonModule::new();
        let context = ctx(json!({"active": true}));

        let result = module.render(".active", &context).unwrap();

        assert_eq!(result, Some("true".to_string()));
    }

    #[test]
    fn resolves_array_index() {
        let module = JsonModule::new();
        let context = ctx(json!({"items": ["a", "b", "c"]}));

        let result = module.render(".items.1", &context).unwrap();

        assert_eq!(result, Some("b".to_string()));
    }

    #[test]
    fn returns_none_for_missing_path() {
        let module = JsonModule::new();
        let context = ctx(json!({"model": {}}));

        let result = module.render(".model.display_name", &context).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn returns_none_for_null_value() {
        let module = JsonModule::new();
        let context = ctx(json!({"value": null}));

        let result = module.render(".value", &context).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn returns_none_when_no_stdin() {
        let module = JsonModule::new();
        let context = ModuleContext::default();

        let result = module.render(".anything", &context).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn errors_on_empty_format() {
        let module = JsonModule::new();
        let context = ModuleContext::default();

        assert!(module.render("", &context).is_err());
    }

    #[test]
    fn works_without_leading_dot() {
        let module = JsonModule::new();
        let context = ctx(json!({"name": "test"}));

        let result = module.render("name", &context).unwrap();

        assert_eq!(result, Some("test".to_string()));
    }
}

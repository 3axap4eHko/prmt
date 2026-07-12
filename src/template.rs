use crate::error::Result;
use crate::executor::validate_module_occurrences;
use crate::module_trait::ModuleContext;
use crate::parser::{Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{AnsiStyle, ModuleStyle, global_no_color};
use is_terminal::IsTerminal;

/// A parsed template intended for a single render.
pub struct Template<'a> {
    tokens: Vec<Token<'a>>,
    estimated_size: usize,
}

impl<'a> Template<'a> {
    /// Parse a template string.
    #[inline]
    pub fn new(template: &'a str) -> Self {
        let tokens = parse(template);
        let estimated_size = template.len() + (template.len() / 2) + 128;
        Self {
            tokens,
            estimated_size,
        }
    }

    /// Render the template with the given registry and context
    pub fn render(&self, registry: &ModuleRegistry, context: &ModuleContext) -> Result<String> {
        validate_module_occurrences(&self.tokens)?;

        let mut output = String::with_capacity(self.estimated_size);

        let no_color = global_no_color() || !IsTerminal::is_terminal(&std::io::stdout());

        for token in &self.tokens {
            match token {
                Token::Text(text) => {
                    output.push_str(text);
                }
                Token::Placeholder(params) => {
                    let module = registry.get(&params.module).ok_or_else(|| {
                        crate::error::PromptError::UnknownModule(params.module.to_string())
                    })?;

                    if let Some(text) = module.render(&params.format, context)?
                        && !text.is_empty()
                    {
                        let has_prefix = !params.prefix.is_empty();
                        let has_suffix = !params.suffix.is_empty();
                        let styled = !params.style.is_empty() && !no_color;

                        if styled {
                            let style = AnsiStyle::parse(&params.style).map_err(|error| {
                                crate::error::PromptError::StyleError {
                                    module: params.module.to_string(),
                                    error,
                                }
                            })?;

                            style.write_start_codes(&mut output, context.shell);
                            if has_prefix {
                                output.push_str(&params.prefix);
                            }
                            output.push_str(&text);
                            if has_suffix {
                                output.push_str(&params.suffix);
                            }
                            style.write_reset(&mut output, context.shell);
                        } else {
                            if has_prefix {
                                output.push_str(&params.prefix);
                            }
                            output.push_str(&text);
                            if has_suffix {
                                output.push_str(&params.suffix);
                            }
                        }
                    }
                }
            }
        }

        Ok(output)
    }

    /// Get an iterator over the tokens in this template
    pub fn tokens(&self) -> impl Iterator<Item = &Token<'a>> {
        self.tokens.iter()
    }

    /// Get the number of tokens in this template
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PromptError;
    use crate::module_trait::Module;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingModule {
        calls: Arc<AtomicUsize>,
    }

    impl Module for CountingModule {
        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some("value".to_string()))
        }
    }

    #[test]
    fn duplicate_singleton_is_rejected_before_rendering() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut registry = ModuleRegistry::new();
        registry.register(
            "test",
            Arc::new(CountingModule {
                calls: Arc::clone(&calls),
            }),
        );
        let template = Template::new("{test}{test}");

        let error = template
            .render(&registry, &ModuleContext::default())
            .expect_err("duplicate module should fail");

        assert!(matches!(
            error,
            PromptError::DuplicateModule(module) if module == "test"
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

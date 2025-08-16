use crate::error::Result;
use crate::module_trait::ModuleContext;
use crate::parser::{Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{AnsiStyle, ModuleStyle};

/// A parsed template that can be rendered multiple times efficiently
pub struct Template<'a> {
    tokens: Vec<Token<'a>>,
    estimated_size: usize,
}

impl<'a> Template<'a> {
    /// Parse a template string into a reusable Template
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
        let mut output = String::with_capacity(self.estimated_size);

        // Check for NO_COLOR environment variable
        let no_color = std::env::var("NO_COLOR").is_ok() || !atty::is(atty::Stream::Stdout);

        for token in &self.tokens {
            match token {
                Token::Text(text) => {
                    output.push_str(text);
                }
                Token::Placeholder(params) => {
                    let module = registry.get(&params.module).ok_or_else(|| {
                        crate::error::PromptError::UnknownModule(params.module.clone())
                    })?;

                    if let Some(text) = module.render(&params.format, context)
                        && !text.is_empty()
                    {
                        // Build the complete segment with minimal allocations
                        if !params.prefix.is_empty() {
                            output.push_str(&params.prefix);
                        }

                        if !params.style.is_empty() && !no_color {
                            let style = AnsiStyle::parse(&params.style).map_err(|error| {
                                crate::error::PromptError::StyleError {
                                    module: params.module.clone(),
                                    error,
                                }
                            })?;
                            let styled = style.apply(&text);
                            output.push_str(&styled);
                        } else {
                            output.push_str(&text);
                        }

                        if !params.suffix.is_empty() {
                            output.push_str(&params.suffix);
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

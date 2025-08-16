use crate::error::{PromptError, Result};
use crate::module_trait::ModuleContext;
use crate::parser::{Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{AnsiStyle, ModuleStyle};

#[inline]
fn estimate_output_size(template: &str) -> usize {
    // Estimate: template length + 50% overhead for module outputs and ANSI codes
    template.len() + (template.len() / 2) + 128
}

pub fn render_template(
    template: &str,
    registry: &ModuleRegistry,
    context: &ModuleContext,
) -> Result<String> {
    let tokens = parse(template);
    let mut output = String::with_capacity(estimate_output_size(template));

    // Check for NO_COLOR environment variable
    let no_color = std::env::var("NO_COLOR").is_ok() || !atty::is(atty::Stream::Stdout);

    for token in tokens {
        match token {
            Token::Text(text) => {
                output.push_str(&text);
            }
            Token::Placeholder(params) => {
                let module = registry
                    .get(&params.module)
                    .ok_or_else(|| PromptError::UnknownModule(params.module.clone()))?;

                if let Some(text) = module.render(&params.format, context)
                    && !text.is_empty()
                {
                    // Build the complete segment with minimal allocations
                    if !params.prefix.is_empty() {
                        output.push_str(&params.prefix);
                    }

                    if !params.style.is_empty() && !no_color {
                        let style = AnsiStyle::parse(&params.style).map_err(|error| {
                            PromptError::StyleError {
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

pub fn execute(format_str: &str, no_version: bool, exit_code: Option<i32>) -> Result<String> {
    let context = ModuleContext {
        no_version,
        exit_code,
    };

    let mut registry = ModuleRegistry::new();
    register_builtin_modules(&mut registry);

    render_template(format_str, &registry, &context)
}

fn register_builtin_modules(registry: &mut ModuleRegistry) {
    use crate::modules::*;
    use std::sync::Arc;

    // Configure Rayon thread pool for prompt generation
    // Limit threads to avoid cold-start overhead in shells
    let max_threads = std::cmp::min(rayon::current_num_threads(), 4);

    if rayon::ThreadPoolBuilder::new()
        .num_threads(max_threads)
        .build_global()
        .is_err()
    {
        // Pool already initialized, that's fine
    }

    // Register modules - these are lightweight operations
    registry.register("path", Arc::new(path::PathModule));
    registry.register("git", Arc::new(git::GitModule));
    registry.register("ok", Arc::new(ok::OkModule));
    registry.register("fail", Arc::new(fail::FailModule));
    registry.register("rust", Arc::new(rust::RustModule));
    registry.register("node", Arc::new(node::NodeModule));
    registry.register("python", Arc::new(python::PythonModule));
    registry.register("go", Arc::new(go::GoModule));
    registry.register("deno", Arc::new(deno::DenoModule));
    registry.register("bun", Arc::new(bun::BunModule))
}

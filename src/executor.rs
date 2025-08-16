use crate::module_trait::ModuleContext;
use crate::parser::{Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{ModuleStyle, AnsiStyle};

pub fn render_template(
    template: &str,
    registry: &ModuleRegistry,
    context: &ModuleContext,
) -> Result<String, String> {
    let mut output = String::new();
    let tokens = parse(template);
    
    for token in tokens {
        match token {
            Token::Text(text) => {
                output.push_str(&text);
            }
            Token::Placeholder(params) => {
                let module = registry.get(&params.module)
                    .ok_or_else(|| format!("Unknown module: {}", params.module))?;
                
                if let Some(text) = module.render(&params.format, context) {
                    if !text.is_empty() {
                        let mut result = String::new();
                        result.push_str(&params.prefix);
                        result.push_str(&text);
                        result.push_str(&params.suffix);
                        
                        if !params.style.is_empty() {
                            let style = AnsiStyle::parse(&params.style)
                                .map_err(|e| format!("Style error for module '{}': {}", params.module, e))?;
                            result = style.apply(&result);
                        }
                        
                        output.push_str(&result);
                    }
                }
            }
        }
    }
    
    Ok(output)
}

pub fn execute(
    format_str: &str,
    no_version: bool,
    exit_code: Option<i32>,
) -> Result<String, String> {
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
    
    registry.register("path", Arc::new(path::PathModule::new()));
    registry.register("git", Arc::new(git::GitModule::new()));
    registry.register("ok", Arc::new(ok::OkModule::new()));
    registry.register("fail", Arc::new(fail::FailModule::new()));
    registry.register("rust", Arc::new(rust::RustModule::new()));
    registry.register("node", Arc::new(node::NodeModule::new()));
    registry.register("python", Arc::new(python::PythonModule::new()));
    registry.register("go", Arc::new(go::GoModule::new()));
    registry.register("deno", Arc::new(deno::DenoModule::new()));
    registry.register("bun", Arc::new(bun::BunModule::new()))
}
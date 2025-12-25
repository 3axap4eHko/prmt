use crate::detector::{DetectionContext, detect};
use crate::error::{PromptError, Result};
use crate::module_trait::{ModuleContext, ModuleRef};
use crate::parser::{Params, Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{AnsiStyle, ModuleStyle, Shell, global_no_color};
use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

#[inline]
fn estimate_output_size(template_len: usize) -> usize {
    template_len + (template_len / 2) + 128
}

enum RenderSlot<'a> {
    Static(Cow<'a, str>),
    Dynamic {
        params: Params,
        module: ModuleRef,
        output: OnceLock<Option<String>>,
    },
}

impl<'a> RenderSlot<'a> {
    fn len(&self) -> usize {
        match self {
            RenderSlot::Static(text) => text.len(),
            RenderSlot::Dynamic { output, .. } => output
                .get()
                .and_then(|value| value.as_ref())
                .map(|text| text.len())
                .unwrap_or(0),
        }
    }
}

#[allow(dead_code)]
pub fn render_template(
    template: &str,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
) -> Result<String> {
    let tokens = parse(template);
    let placeholder_count = count_placeholders(&tokens);
    render_tokens(
        tokens,
        registry,
        context,
        no_color,
        template.len(),
        placeholder_count,
    )
}

fn render_tokens<'a>(
    tokens: Vec<Token<'a>>,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
    template_len: usize,
    placeholder_count: usize,
) -> Result<String> {
    if placeholder_count <= 1 {
        return render_tokens_sequential(tokens, registry, context, no_color, template_len);
    }

    render_tokens_parallel(tokens, registry, context, no_color)
}

fn render_tokens_sequential<'a>(
    tokens: Vec<Token<'a>>,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
    template_len: usize,
) -> Result<String> {
    let mut output = String::with_capacity(estimate_output_size(template_len));

    for token in tokens {
        match token {
            Token::Text(text) => output.push_str(&text),
            Token::Placeholder(params) => {
                let module = registry
                    .get(&params.module)
                    .ok_or_else(|| PromptError::UnknownModule(params.module.clone()))?;

                if let Some(value) = render_placeholder(&module, &params, context, no_color)? {
                    output.push_str(&value);
                }
            }
        }
    }

    Ok(output)
}

fn render_tokens_parallel<'a>(
    tokens: Vec<Token<'a>>,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
) -> Result<String> {
    let mut slots = Vec::with_capacity(tokens.len());
    let mut dynamic_indices = Vec::new();

    for token in tokens.into_iter() {
        match token {
            Token::Text(text) => {
                slots.push(RenderSlot::Static(text));
            }
            Token::Placeholder(params) => {
                let module = registry
                    .get(&params.module)
                    .ok_or_else(|| PromptError::UnknownModule(params.module.clone()))?;

                let index = slots.len();
                slots.push(RenderSlot::Dynamic {
                    params,
                    module,
                    output: OnceLock::new(),
                });
                dynamic_indices.push(index);
            }
        }
    }

    if dynamic_indices.len() <= 1 {
        for &index in &dynamic_indices {
            compute_slot(&slots[index], context, no_color)?;
        }
    } else {
        ensure_thread_pool();
        dynamic_indices
            .par_iter()
            .try_for_each(|&index| compute_slot(&slots[index], context, no_color))?;
    }

    let total_len: usize = slots.iter().map(RenderSlot::len).sum();
    let mut output = String::with_capacity(total_len);

    for slot in slots.into_iter() {
        match slot {
            RenderSlot::Static(text) => output.push_str(&text),
            RenderSlot::Dynamic {
                output: slot_output,
                ..
            } => {
                if let Some(Some(text)) = slot_output.into_inner() {
                    output.push_str(&text);
                }
            }
        }
    }

    Ok(output)
}

#[allow(dead_code)]
pub fn execute(
    format_str: &str,
    no_version: bool,
    exit_code: Option<i32>,
    no_color: bool,
) -> Result<String> {
    execute_with_shell(format_str, no_version, exit_code, no_color, Shell::None)
}

pub fn execute_with_shell(
    format_str: &str,
    no_version: bool,
    exit_code: Option<i32>,
    no_color: bool,
    shell: Shell,
) -> Result<String> {
    let tokens = parse(format_str);
    let (registry, placeholder_count) = build_registry(&tokens)?;
    let required_markers = registry.required_markers();
    let detection = if required_markers.is_empty() {
        DetectionContext::default()
    } else {
        detect(&required_markers)
    };
    let context = ModuleContext {
        no_version,
        exit_code,
        detection,
        shell,
    };
    let resolved_no_color = no_color || global_no_color();
    render_tokens(
        tokens,
        &registry,
        &context,
        resolved_no_color,
        format_str.len(),
        placeholder_count,
    )
}

fn render_placeholder(
    module: &ModuleRef,
    params: &Params,
    context: &ModuleContext,
    no_color: bool,
) -> Result<Option<String>> {
    let Some(text) = module.render(&params.format, context)? else {
        return Ok(None);
    };

    if text.is_empty() && params.prefix.is_empty() && params.suffix.is_empty() {
        return Ok(None);
    }

    // Build the complete segment (prefix + text + suffix)
    let estimated_len = params.prefix.len() + text.len() + params.suffix.len();
    let mut segment = String::with_capacity(estimated_len);

    if !params.prefix.is_empty() {
        segment.push_str(&params.prefix);
    }
    segment.push_str(&text);
    if !params.suffix.is_empty() {
        segment.push_str(&params.suffix);
    }

    // Apply style to the entire segment
    if params.style.is_empty() || no_color {
        return Ok(Some(segment));
    }

    let style = AnsiStyle::parse(&params.style).map_err(|error| PromptError::StyleError {
        module: params.module.clone(),
        error,
    })?;
    let styled = style.apply_with_shell(&segment, context.shell);
    Ok(Some(styled))
}

fn compute_slot(slot: &RenderSlot<'_>, context: &ModuleContext, no_color: bool) -> Result<()> {
    let RenderSlot::Dynamic {
        params,
        module,
        output,
    } = slot
    else {
        return Ok(());
    };

    let value = render_placeholder(module, params, context, no_color)?;
    output
        .set(value)
        .expect("placeholder result should only be computed once");
    Ok(())
}

fn count_placeholders(tokens: &[Token<'_>]) -> usize {
    tokens
        .iter()
        .filter(|token| matches!(token, Token::Placeholder(_)))
        .count()
}

fn build_registry(tokens: &[Token<'_>]) -> Result<(ModuleRegistry, usize)> {
    let mut registry = ModuleRegistry::new();
    let mut required: HashSet<&str> = HashSet::new();
    let mut placeholder_count = 0usize;

    for token in tokens {
        if let Token::Placeholder(params) = token {
            placeholder_count += 1;
            let name = params.module.as_str();
            if required.insert(name) {
                let module = instantiate_module(name)
                    .ok_or_else(|| PromptError::UnknownModule(name.to_string()))?;
                registry.register(name.to_string(), module);
            }
        }
    }

    Ok((registry, placeholder_count))
}

fn instantiate_module(name: &str) -> Option<ModuleRef> {
    use crate::modules::*;
    Some(match name {
        "path" => Arc::new(path::PathModule::new()),
        "git" => Arc::new(git::GitModule::new()),
        "env" => Arc::new(env::EnvModule::new()),
        "ok" => Arc::new(ok::OkModule::new()),
        "fail" => Arc::new(fail::FailModule::new()),
        "rust" => Arc::new(rust::RustModule::new()),
        "node" => Arc::new(node::NodeModule::new()),
        "python" => Arc::new(python::PythonModule::new()),
        "go" => Arc::new(go::GoModule::new()),
        "deno" => Arc::new(deno::DenoModule::new()),
        "bun" => Arc::new(bun::BunModule::new()),
        "time" => Arc::new(time::TimeModule),
        _ => return None,
    })
}

fn ensure_thread_pool() {
    static THREAD_POOL_INIT: OnceLock<()> = OnceLock::new();
    THREAD_POOL_INIT.get_or_init(|| {
        let max_threads = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1)
            .clamp(1, 4);
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(max_threads)
            .build_global();
    });
}

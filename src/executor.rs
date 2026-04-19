use crate::detector::{DetectionContext, detect};
use crate::error::{PromptError, Result};
use crate::module_trait::{ModuleContext, ModuleRef};
use crate::parser::{Params, Token, parse};
use crate::registry::ModuleRegistry;
use crate::style::{AnsiStyle, ModuleStyle, Shell, global_no_color};
use std::borrow::Cow;
use std::collections::HashSet;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

const TIMEOUT_PLACEHOLDER: &str = "...";

#[inline]
fn estimate_output_size(template_len: usize) -> usize {
    template_len + (template_len / 2) + 128
}

enum SlotResult {
    Value(Option<String>),
    Error(PromptError),
}

struct WorkerReply {
    index: usize,
    result: SlotResult,
}

fn spawn_slot_render(
    index: usize,
    module_name: &str,
    module: &ModuleRef,
    format: &str,
    context: &ModuleContext,
    done: &mpsc::Sender<WorkerReply>,
) {
    let module = Arc::clone(module);
    let module_name = module_name.to_string();
    let format = format.to_string();
    let context = context.clone();
    let done = done.clone();

    thread::spawn(move || {
        let result =
            match panic::catch_unwind(AssertUnwindSafe(|| module.render(&format, &context))) {
                Ok(Ok(text)) => SlotResult::Value(text),
                Ok(Err(error)) => SlotResult::Error(error),
                Err(_) => SlotResult::Error(PromptError::ModulePanic(module_name)),
            };

        let _ = done.send(WorkerReply { index, result });
    });
}

fn recv_reply_until(
    done_rx: &mpsc::Receiver<WorkerReply>,
    deadline: Option<Instant>,
) -> Option<WorkerReply> {
    match deadline {
        Some(deadline) => {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_default();
            if remaining.is_zero() {
                return None;
            }
            done_rx.recv_timeout(remaining).ok()
        }
        None => done_rx.recv().ok(),
    }
}

fn collect_pending(
    done_rx: &mpsc::Receiver<WorkerReply>,
    slots: &mut [Slot<'_>],
    pending_count: usize,
    deadline: Option<Instant>,
) -> Result<()> {
    let mut completed = 0usize;

    while completed < pending_count {
        let Some(reply) = recv_reply_until(done_rx, deadline) else {
            break;
        };
        completed += 1;

        match reply.result {
            SlotResult::Value(text) => {
                if let Some(Slot::Pending { result, .. }) = slots.get_mut(reply.index) {
                    *result = Some(SlotResult::Value(text));
                }
            }
            SlotResult::Error(error) => return Err(error),
        }
    }

    Ok(())
}

fn style_output(
    text: Option<String>,
    params: &Params,
    context: &ModuleContext,
    no_color: bool,
) -> Result<Option<String>> {
    let Some(text) = text else {
        return Ok(None);
    };

    if text.is_empty() && params.prefix.is_empty() && params.suffix.is_empty() {
        return Ok(None);
    }

    let estimated_len = params.prefix.len() + text.len() + params.suffix.len();
    let mut segment = String::with_capacity(estimated_len);

    if !params.prefix.is_empty() {
        segment.push_str(&params.prefix);
    }
    segment.push_str(&text);
    if !params.suffix.is_empty() {
        segment.push_str(&params.suffix);
    }

    if params.style.is_empty() || no_color {
        return Ok(Some(segment));
    }

    let style = AnsiStyle::parse(&params.style).map_err(|error| PromptError::StyleError {
        module: params.module.to_string(),
        error,
    })?;
    let styled = style.apply_with_shell(&segment, context.shell);
    Ok(Some(styled))
}

#[allow(dead_code)]
pub fn render_template(
    template: &str,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
) -> Result<String> {
    let tokens = parse(template);
    render_tokens(tokens, registry, context, no_color, template.len(), None)
}

enum PlanItem<'a> {
    Static(Cow<'a, str>),
    Fast {
        params: Params<'a>,
        module: ModuleRef,
    },
    Blocking {
        params: Params<'a>,
        module: ModuleRef,
    },
}

enum Slot<'a> {
    Static(Cow<'a, str>),
    Rendered(Option<String>),
    Pending {
        params: Params<'a>,
        result: Option<SlotResult>,
    },
}

fn render_tokens<'a>(
    tokens: Vec<Token<'a>>,
    registry: &ModuleRegistry,
    context: &ModuleContext,
    no_color: bool,
    template_len: usize,
    timeout: Option<Duration>,
) -> Result<String> {
    let mut plan: Vec<PlanItem<'a>> = Vec::with_capacity(tokens.len());
    let mut blocking_count = 0usize;

    for token in tokens {
        match token {
            Token::Text(text) => plan.push(PlanItem::Static(text)),
            Token::Placeholder(params) => {
                let module = registry
                    .get(&params.module)
                    .ok_or_else(|| PromptError::UnknownModule(params.module.to_string()))?;
                if module.is_blocking() {
                    blocking_count += 1;
                    plan.push(PlanItem::Blocking { params, module });
                } else {
                    plan.push(PlanItem::Fast { params, module });
                }
            }
        }
    }

    let use_threads = blocking_count > 1 || (blocking_count == 1 && timeout.is_some());

    if !use_threads {
        return render_plan_inline(plan, context, no_color, template_len);
    }

    render_plan_parallel(
        plan,
        context,
        no_color,
        template_len,
        timeout,
        blocking_count,
    )
}

fn render_plan_inline<'a>(
    plan: Vec<PlanItem<'a>>,
    context: &ModuleContext,
    no_color: bool,
    template_len: usize,
) -> Result<String> {
    let mut output = String::with_capacity(estimate_output_size(template_len));

    for item in plan {
        match item {
            PlanItem::Static(text) => output.push_str(&text),
            PlanItem::Fast { params, module } | PlanItem::Blocking { params, module } => {
                let text = module.render(&params.format, context)?;
                if let Some(value) = style_output(text, &params, context, no_color)? {
                    output.push_str(&value);
                }
            }
        }
    }

    Ok(output)
}

fn render_plan_parallel<'a>(
    plan: Vec<PlanItem<'a>>,
    context: &ModuleContext,
    no_color: bool,
    template_len: usize,
    timeout: Option<Duration>,
    blocking_count: usize,
) -> Result<String> {
    let deadline = timeout.map(|timeout| Instant::now() + timeout);
    let (done_tx, done_rx) = mpsc::channel();

    for (index, item) in plan.iter().enumerate() {
        if let PlanItem::Blocking { params, module } = item {
            spawn_slot_render(
                index,
                &params.module,
                module,
                &params.format,
                context,
                &done_tx,
            );
        }
    }
    drop(done_tx);

    let mut slots: Vec<Slot<'a>> = Vec::with_capacity(plan.len());
    for item in plan {
        match item {
            PlanItem::Static(text) => slots.push(Slot::Static(text)),
            PlanItem::Fast { params, module } => {
                let text = module.render(&params.format, context)?;
                let rendered = style_output(text, &params, context, no_color)?;
                slots.push(Slot::Rendered(rendered));
            }
            PlanItem::Blocking { params, .. } => {
                slots.push(Slot::Pending {
                    params,
                    result: None,
                });
            }
        }
    }

    collect_pending(&done_rx, &mut slots, blocking_count, deadline)?;

    let mut output = String::with_capacity(estimate_output_size(template_len));
    for slot in slots {
        match slot {
            Slot::Static(text) => output.push_str(&text),
            Slot::Rendered(Some(value)) => output.push_str(&value),
            Slot::Rendered(None) => {}
            Slot::Pending { params, result } => {
                let text = match result {
                    Some(SlotResult::Value(text)) => text,
                    Some(SlotResult::Error(error)) => return Err(error),
                    None => {
                        if timeout.is_some() {
                            Some(TIMEOUT_PLACEHOLDER.to_string())
                        } else {
                            return Err(PromptError::ModulePanic(params.module.to_string()));
                        }
                    }
                };
                if let Some(value) = style_output(text, &params, context, no_color)? {
                    output.push_str(&value);
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
    execute_with_shell(
        format_str,
        no_version,
        exit_code,
        no_color,
        Shell::None,
        None,
        None,
    )
}

pub fn execute_with_shell(
    format_str: &str,
    no_version: bool,
    exit_code: Option<i32>,
    no_color: bool,
    shell: Shell,
    stdin_data: Option<Arc<serde_json::Value>>,
    timeout: Option<Duration>,
) -> Result<String> {
    let tokens = parse(format_str);
    let registry = build_registry(&tokens)?;
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
        stdin_data,
    };
    let resolved_no_color = no_color || global_no_color();
    render_tokens(
        tokens,
        &registry,
        &context,
        resolved_no_color,
        format_str.len(),
        timeout,
    )
}

#[cfg(test)]
fn render_module_with_timeout(
    module_name: &str,
    module: &ModuleRef,
    format: &str,
    context: &ModuleContext,
    timeout: Option<Duration>,
) -> Result<Option<String>> {
    let (done_tx, done_rx) = mpsc::channel();
    spawn_slot_render(0, module_name, module, format, context, &done_tx);
    drop(done_tx);
    let reply = recv_reply_until(&done_rx, timeout.map(|timeout| Instant::now() + timeout));

    match reply.map(|reply| reply.result) {
        Some(SlotResult::Value(text)) => Ok(text),
        Some(SlotResult::Error(error)) => Err(error),
        None => Ok(Some(TIMEOUT_PLACEHOLDER.to_string())),
    }
}

#[cfg(test)]
fn render_placeholder(
    module: &ModuleRef,
    params: &Params,
    context: &ModuleContext,
    no_color: bool,
    timeout: Option<Duration>,
) -> Result<Option<String>> {
    let text = if timeout.is_some() {
        render_module_with_timeout(&params.module, module, &params.format, context, timeout)?
    } else {
        module.render(&params.format, context)?
    };
    style_output(text, params, context, no_color)
}

fn build_registry(tokens: &[Token<'_>]) -> Result<ModuleRegistry> {
    let mut registry = ModuleRegistry::new();
    let mut required: HashSet<&str> = HashSet::new();

    for token in tokens {
        if let Token::Placeholder(params) = token {
            let name: &str = &params.module;
            if required.insert(name) {
                let module = instantiate_module(name)
                    .ok_or_else(|| PromptError::UnknownModule(name.to_string()))?;
                registry.register(name.to_string(), module);
            }
        }
    }

    Ok(registry)
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
        "elixir" => Arc::new(elixir::ElixirModule::new()),
        "deno" => Arc::new(deno::DenoModule::new()),
        "bun" => Arc::new(bun::BunModule::new()),
        "time" => Arc::new(time::TimeModule),
        "json" => Arc::new(json::JsonModule::new()),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::module_trait::Module;
    use serial_test::serial;

    struct SlowModule {
        delay: Duration,
        value: &'static str,
    }

    impl Module for SlowModule {
        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            std::thread::sleep(self.delay);
            Ok(Some(self.value.to_string()))
        }
    }

    struct NoneModule;

    impl Module for NoneModule {
        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            Ok(None)
        }
    }

    struct PanicModule;

    impl Module for PanicModule {
        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            panic!("boom")
        }
    }

    struct SignalingModule {
        ready: std::sync::mpsc::SyncSender<()>,
        value: &'static str,
    }

    impl Module for SignalingModule {
        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            let _ = self.ready.send(());
            Ok(Some(self.value.to_string()))
        }
    }

    struct ErrorModule;

    impl Module for ErrorModule {
        fn is_blocking(&self) -> bool {
            true
        }

        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            Err(PromptError::InvalidFormat {
                module: "err".to_string(),
                format: "bad".to_string(),
                valid_formats: "valid".to_string(),
            })
        }
    }

    struct BlockingModule {
        started: std::sync::mpsc::SyncSender<()>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl Module for BlockingModule {
        fn is_blocking(&self) -> bool {
            true
        }

        fn render(&self, _format: &str, _context: &ModuleContext) -> Result<Option<String>> {
            let _ = self.started.send(());
            if let Ok(rx) = self.release.lock() {
                let _ = rx.recv();
            }
            Ok(Some("slow".to_string()))
        }
    }

    fn test_context() -> ModuleContext {
        ModuleContext::default()
    }

    fn test_params() -> Params<'static> {
        Params {
            module: Cow::Borrowed("test"),
            style: Cow::Borrowed(""),
            format: Cow::Borrowed(""),
            prefix: Cow::Borrowed(""),
            suffix: Cow::Borrowed(""),
        }
    }

    #[test]
    #[serial]
    fn slow_module_times_out_with_placeholder() {
        let module: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(50),
            value: "should_not_see",
        });
        let ctx = test_context();
        let result =
            render_module_with_timeout("test", &module, "", &ctx, Some(Duration::from_millis(5)))
                .unwrap();
        assert_eq!(result, Some(TIMEOUT_PLACEHOLDER.to_string()));
    }

    #[test]
    #[serial]
    fn fast_module_completes_within_timeout() {
        let module: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(0),
            value: "fast_result",
        });
        let ctx = test_context();
        let result =
            render_module_with_timeout("test", &module, "", &ctx, Some(Duration::from_millis(100)))
                .unwrap();
        assert_eq!(result, Some("fast_result".to_string()));
    }

    #[test]
    #[serial]
    fn none_module_returns_none_with_timeout() {
        let module: ModuleRef = Arc::new(NoneModule);
        let ctx = test_context();
        let result =
            render_module_with_timeout("test", &module, "", &ctx, Some(Duration::from_millis(100)))
                .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    #[serial]
    fn render_placeholder_uses_timeout_when_set() {
        let module: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(50),
            value: "slow",
        });
        let ctx = test_context();
        let params = test_params();
        let result =
            render_placeholder(&module, &params, &ctx, true, Some(Duration::from_millis(5)))
                .unwrap();
        assert_eq!(result, Some(TIMEOUT_PLACEHOLDER.to_string()));
    }

    #[test]
    #[serial]
    fn render_placeholder_skips_timeout_when_none() {
        let module: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(0),
            value: "direct",
        });
        let ctx = test_context();
        let params = test_params();
        let result = render_placeholder(&module, &params, &ctx, true, None).unwrap();
        assert_eq!(result, Some("direct".to_string()));
    }

    #[test]
    #[serial]
    fn timeout_result_gets_prefix_and_suffix() {
        let module: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(50),
            value: "slow",
        });
        let ctx = test_context();
        let params = Params {
            module: Cow::Borrowed("test"),
            style: Cow::Borrowed(""),
            format: Cow::Borrowed(""),
            prefix: Cow::Borrowed("["),
            suffix: Cow::Borrowed("]"),
        };
        let result =
            render_placeholder(&module, &params, &ctx, true, Some(Duration::from_millis(5)))
                .unwrap();
        assert_eq!(result, Some(format!("[{}]", TIMEOUT_PLACEHOLDER)));
    }

    #[test]
    #[serial]
    fn panic_module_returns_error() {
        let module: ModuleRef = Arc::new(PanicModule);
        let ctx = test_context();
        let err = render_module_with_timeout("panic", &module, "", &ctx, None).unwrap_err();
        assert!(matches!(err, PromptError::ModulePanic(name) if name == "panic"));
    }

    #[test]
    #[serial]
    fn global_timeout_keeps_ready_later_slot() {
        let slow: ModuleRef = Arc::new(SlowModule {
            delay: Duration::from_millis(50),
            value: "slow",
        });
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let fast: ModuleRef = Arc::new(SignalingModule {
            ready: ready_tx,
            value: "fast",
        });
        let ctx = test_context();
        let (done_tx, done_rx) = mpsc::channel();
        spawn_slot_render(0, "slow", &slow, "", &ctx, &done_tx);
        spawn_slot_render(1, "fast", &fast, "", &ctx, &done_tx);
        drop(done_tx);
        let mut slots = vec![
            Slot::Pending {
                params: test_params(),
                result: None,
            },
            Slot::Pending {
                params: test_params(),
                result: None,
            },
        ];

        ready_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("fast module should finish before slow timeout");

        collect_pending(
            &done_rx,
            &mut slots,
            2,
            Some(Instant::now() + Duration::from_millis(5)),
        )
        .unwrap();

        match &slots[0] {
            Slot::Pending { result: None, .. } => {}
            Slot::Pending {
                result: Some(SlotResult::Value(_)),
                ..
            } => panic!("slow slot should still be unresolved"),
            Slot::Pending {
                result: Some(SlotResult::Error(error)),
                ..
            } => panic!("unexpected error: {error}"),
            Slot::Static(_) | Slot::Rendered(_) => panic!("expected pending slot"),
        }
        match &slots[1] {
            Slot::Pending {
                result: Some(SlotResult::Value(Some(text))),
                ..
            } => assert_eq!(text, "fast"),
            Slot::Pending {
                result: Some(SlotResult::Value(None)),
                ..
            } => panic!("fast slot should have a value"),
            Slot::Pending {
                result: Some(SlotResult::Error(error)),
                ..
            } => panic!("unexpected error: {error}"),
            Slot::Pending { result: None, .. } => {
                panic!("fast slot should have completed before timeout")
            }
            Slot::Static(_) | Slot::Rendered(_) => panic!("expected pending slot"),
        }
    }

    #[test]
    #[serial]
    fn immediate_error_does_not_wait_for_slow_slot() {
        let (started_tx, started_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release_rx) = std::sync::mpsc::sync_channel(1);
        let slow: ModuleRef = Arc::new(BlockingModule {
            started: started_tx,
            release: std::sync::Mutex::new(release_rx),
        });
        let err: ModuleRef = Arc::new(ErrorModule);
        let mut registry = ModuleRegistry::new();
        registry.register("slow", Arc::clone(&slow));
        registry.register("err", Arc::clone(&err));
        let ctx = test_context();
        let tokens = parse("{slow} {err}");
        let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);

        let handle = thread::spawn(move || {
            let result = render_tokens(tokens, &registry, &ctx, true, 12, None);
            let _ = result_tx.send(result);
        });

        started_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("slow module should start");

        let result = result_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("immediate error should not wait for slow slot");

        let _ = release_tx.send(());
        handle.join().expect("render thread should join");

        assert!(matches!(
            result,
            Err(PromptError::InvalidFormat { module, format, .. })
                if module == "err" && format == "bad"
        ));
    }
}

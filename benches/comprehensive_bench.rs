use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use prmt::{ModuleContext, ModuleRegistry, Template, execute, parse};
use std::hint::black_box;
use std::time::Duration;

fn setup_registry() -> ModuleRegistry {
    use prmt::modules::*;
    use std::sync::Arc;

    let mut registry = ModuleRegistry::new();
    registry.register("path", Arc::new(path::PathModule));
    registry.register("git", Arc::new(git::GitModule));
    registry.register("rust", Arc::new(rust::RustModule));
    registry.register("node", Arc::new(node::NodeModule));
    registry.register("python", Arc::new(python::PythonModule));
    registry.register("go", Arc::new(go::GoModule));
    registry.register("deno", Arc::new(deno::DenoModule));
    registry.register("bun", Arc::new(bun::BunModule));
    registry.register("elixir", Arc::new(elixir::ElixirModule));
    registry.register("ok", Arc::new(ok::OkModule));
    registry.register("fail", Arc::new(fail::FailModule));
    registry
}

fn bench_parser_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_scenarios");

    // Different input sizes
    let scenarios = vec![
        ("empty", ""),
        ("tiny", "{path}"),
        ("small", "{path:cyan} {git:purple}"),
        (
            "medium",
            "{path:cyan:short:[:]} {rust:red} {node:green} {git:purple:full}",
        ),
        (
            "large",
            "{path:cyan:truncate:30:>>:<<} {rust:red:full} {node:green:major} {python:yellow:short} {go:blue} {deno:magenta} {bun:white} {elixir:purple} {git:purple:full:🌿:} {ok:green:✓} {fail:red:✗}",
        ),
        (
            "escaped_heavy",
            "\\{escaped\\} {real} \\n\\t\\: {another:with\\:colon} \\\\backslash",
        ),
        (
            "text_heavy",
            "This is a long text prefix before {path} and then more text {git} and even more text at the end",
        ),
        ("placeholder_only", "{a}{b}{c}{d}{e}{f}{g}{h}{i}{j}"),
    ];

    for (name, template) in scenarios {
        group.throughput(Throughput::Bytes(template.len() as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &template,
            |b, &template| {
                b.iter(|| parse(black_box(template)));
            },
        );
    }

    group.finish();
}

fn bench_template_rendering(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_rendering");

    let registry = setup_registry();
    let context_with_version = ModuleContext {
        no_version: false,
        exit_code: Some(0),
    };
    let context_no_version = ModuleContext {
        no_version: true,
        exit_code: Some(0),
    };
    let context_error = ModuleContext {
        no_version: true,
        exit_code: Some(1),
    };

    let scenarios = vec![
        ("minimal", "{path}", context_no_version.clone()),
        (
            "typical_success",
            "{path:cyan} {git:purple} {ok:green:✓}",
            context_no_version.clone(),
        ),
        (
            "typical_error",
            "{path:cyan} {git:purple} {fail:red:✗}",
            context_error.clone(),
        ),
        (
            "with_versions",
            "{path} {rust} {node} {git}",
            context_with_version.clone(),
        ),
        (
            "complex_styled",
            "{path:cyan.bold:short:[:]} {git:purple.italic::on :}",
            context_no_version.clone(),
        ),
        (
            "all_modules",
            "{path} {rust} {node} {python} {go} {deno} {bun} {elixir} {git} {ok}",
            context_no_version.clone(),
        ),
    ];

    for (name, template_str, context) in scenarios {
        let template = Template::new(template_str);
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &(&template, &registry, &context),
            |b, &(template, registry, context)| {
                b.iter(|| template.render(black_box(registry), black_box(context)));
            },
        );
    }

    group.finish();
}

fn bench_end_to_end_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");
    group.measurement_time(Duration::from_secs(10));

    let scenarios = vec![
        ("minimal", "{path}"),
        ("shell_bash", "\\u{250c}[{path:cyan}]\\n\\u{2514}> "),
        ("shell_fish", "{path:cyan} {git:purple}❯ "),
        (
            "shell_zsh",
            "{path:blue:short} on {git:yellow::🌿:} {rust} ",
        ),
        (
            "powerline",
            "{path:cyan::: }{git:purple.bold::: }{ok:green:❯:}{fail:red:❯:}",
        ),
        (
            "verbose",
            "{path:cyan:absolute} ({rust:red:full} {node:green:full}) [{git:purple:full}] ",
        ),
        ("corporate", "[{path}] <{git:short}> {ok:$:}{fail:$:} "),
    ];

    for (name, format) in scenarios {
        group.bench_with_input(BenchmarkId::from_parameter(name), &format, |b, &format| {
            b.iter(|| execute(black_box(format), true, Some(0), false));
        });
    }

    group.finish();
}

fn bench_cache_effectiveness(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_effectiveness");

    // First call (cold cache)
    group.bench_function("git_cold_cache", |b| {
        use prmt::Module;
        use prmt::modules::git::GitModule;

        let module = GitModule;
        let context = ModuleContext {
            no_version: false,
            exit_code: None,
        };

        b.iter(|| {
            // Clear cache would go here if we had a method for it
            module.render(black_box("full"), black_box(&context))
        });
    });

    // Warmed cache
    group.bench_function("git_warm_cache", |b| {
        use prmt::Module;
        use prmt::modules::git::GitModule;

        let module = GitModule;
        let context = ModuleContext {
            no_version: false,
            exit_code: None,
        };

        // Warm the cache
        let _ = module.render("full", &context);

        b.iter(|| module.render(black_box("full"), black_box(&context)));
    });

    // Version module cold
    group.bench_function("rust_version_cold", |b| {
        use prmt::Module;
        use prmt::modules::rust::RustModule;

        let module = RustModule;
        let context = ModuleContext {
            no_version: false,
            exit_code: None,
        };

        b.iter(|| module.render(black_box("full"), black_box(&context)));
    });

    // Version module with no_version flag
    group.bench_function("rust_no_version_flag", |b| {
        use prmt::Module;
        use prmt::modules::rust::RustModule;

        let module = RustModule;
        let context = ModuleContext {
            no_version: true,
            exit_code: None,
        };

        b.iter(|| module.render(black_box("full"), black_box(&context)));
    });

    group.finish();
}

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    // Benchmark different string building strategies
    group.bench_function("string_push_str", |b| {
        b.iter(|| {
            let mut s = String::with_capacity(100);
            for _ in 0..10 {
                s.push_str("hello ");
                s.push_str("world ");
            }
            black_box(s)
        });
    });

    group.bench_function("string_format", |b| {
        b.iter(|| {
            let mut s = String::new();
            for _ in 0..10 {
                s = format!("{} hello world ", s);
            }
            black_box(s)
        });
    });

    // Benchmark Cow operations
    group.bench_function("cow_borrowed", |b| {
        use std::borrow::Cow;
        b.iter(|| {
            let text = "hello world";
            let cow: Cow<str> = Cow::Borrowed(text);
            black_box(cow)
        });
    });

    group.bench_function("cow_owned", |b| {
        use std::borrow::Cow;
        b.iter(|| {
            let text = "hello world".to_string();
            let cow: Cow<str> = Cow::Owned(text);
            black_box(cow)
        });
    });

    group.finish();
}

fn bench_unicode_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("unicode");

    use unicode_width::UnicodeWidthStr;

    let strings = vec![
        ("ascii", "hello world"),
        ("emoji", "👋 🌍 Hello World! 🎉"),
        ("cjk", "你好世界 こんにちは世界"),
        ("mixed", "Hello 世界 🌍 мир"),
    ];

    for (name, text) in strings {
        group.bench_with_input(
            BenchmarkId::new("width_calculation", name),
            &text,
            |b, &text| {
                b.iter(|| UnicodeWidthStr::width(black_box(text)));
            },
        );
    }

    group.finish();
}

fn bench_style_parsing(c: &mut Criterion) {
    use prmt::style::{AnsiStyle, ModuleStyle};

    let mut group = c.benchmark_group("style_parsing");

    let styles = vec![
        ("simple", "red"),
        ("with_modifiers", "cyan.bold.italic"),
        ("hex_color", "#00ff00"),
        ("complex", "yellow.bold.italic.underline.dim"),
    ];

    for (name, style_str) in styles {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &style_str,
            |b, &style_str| {
                b.iter(|| AnsiStyle::parse(black_box(style_str)));
            },
        );
    }

    group.finish();
}

fn bench_worst_case_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case");

    // Deeply nested escapes
    let nested_escapes = "\\\\\\{\\\\\\}\\\\\\{\\\\\\}\\\\\\n\\\\\\t";
    group.bench_function("deeply_nested_escapes", |b| {
        b.iter(|| parse(black_box(nested_escapes)));
    });

    // Many small placeholders
    let many_placeholders = (0..50).map(|i| format!("{{p{}}}", i)).collect::<String>();
    group.bench_function("many_placeholders", |b| {
        b.iter(|| parse(black_box(&many_placeholders)));
    });

    // Very long single placeholder
    let long_placeholder = format!(
        "{{{}:{}:{}:{}:{}}}",
        "m".repeat(100),
        "s".repeat(100),
        "f".repeat(100),
        "p".repeat(100),
        "x".repeat(100)
    );
    group.bench_function("long_placeholder", |b| {
        b.iter(|| parse(black_box(&long_placeholder)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_scenarios,
    bench_template_rendering,
    bench_end_to_end_scenarios,
    bench_cache_effectiveness,
    bench_string_operations,
    bench_unicode_operations,
    bench_style_parsing,
    bench_worst_case_scenarios
);
criterion_main!(benches);

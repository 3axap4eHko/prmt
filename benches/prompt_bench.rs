use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use prmt::detector::{DetectionContext, detect};
use prmt::style::Shell;
use prmt::{ModuleContext, ModuleRegistry, Template, execute};
use std::collections::HashSet;
use std::hint::black_box;

fn setup_registry() -> ModuleRegistry {
    use prmt::modules::*;
    use std::sync::Arc;

    let mut registry = ModuleRegistry::new();
    registry.register("path", Arc::new(path::PathModule));
    registry.register("git", Arc::new(git::GitModule));
    registry.register("rust", Arc::new(rust::RustModule));
    registry.register("node", Arc::new(node::NodeModule));
    registry.register("ok", Arc::new(ok::OkModule));
    registry.register("fail", Arc::new(fail::FailModule));
    registry
}

fn detection_for(markers: &[&'static str]) -> DetectionContext {
    if markers.is_empty() {
        return DetectionContext::default();
    }

    let required: HashSet<&str> = markers.iter().copied().collect();

    detect(&required)
}

fn ctx(no_version: bool, exit_code: Option<i32>, markers: &[&'static str]) -> ModuleContext {
    ModuleContext {
        no_version,
        exit_code,
        detection: detection_for(markers),
        shell: Shell::None,
    }
}

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    group.bench_function("simple_text", |b| {
        b.iter(|| {
            prmt::parse(black_box("Hello, World! This is a simple text"));
        });
    });

    group.bench_function("single_placeholder", |b| {
        b.iter(|| {
            prmt::parse(black_box("{path:cyan:short:[:]"));
        });
    });

    group.bench_function("mixed_content", |b| {
        b.iter(|| {
            prmt::parse(black_box(
                "Welcome {user:yellow} to {path:cyan:short} [{git:purple}]",
            ));
        });
    });

    group.bench_function("complex_format", |b| {
        let format = "{path:cyan} {rust:red} {node:green} {git:purple}";
        b.iter(|| {
            prmt::parse(black_box(format));
        });
    });

    group.finish();
}

fn bench_renderer(c: &mut Criterion) {
    let mut group = c.benchmark_group("renderer");

    let registry = setup_registry();
    let ctx_path = ctx(true, Some(0), &[]);
    let ctx_git = ctx(true, Some(0), &[".git"]);
    let ctx_full = ctx(true, Some(0), &[".git", "Cargo.toml", "package.json"]);

    group.bench_function("path_only", |b| {
        let template = Template::new("{path:cyan}");
        b.iter(|| template.render(black_box(&registry), black_box(&ctx_path)));
    });

    group.bench_function("path_and_git", |b| {
        let template = Template::new("{path:cyan} {git:purple}");
        b.iter(|| template.render(black_box(&registry), black_box(&ctx_git)));
    });

    group.bench_function("all_modules", |b| {
        let template = Template::new("{path:cyan} {rust:red} {node:green} {git:purple}");
        b.iter(|| template.render(black_box(&registry), black_box(&ctx_full)));
    });

    group.finish();
}

fn bench_end_to_end(c: &mut Criterion) {
    let mut group = c.benchmark_group("end_to_end");

    let formats = vec![
        ("minimal", "{path}"),
        ("typical", "{path:cyan} {git:purple}"),
        (
            "complex",
            "{path:cyan:short} {rust:red} {node:green} {git:purple:full}",
        ),
    ];

    for (name, format) in formats {
        group.bench_with_input(BenchmarkId::from_parameter(name), &format, |b, &format| {
            b.iter(|| execute(black_box(format), true, Some(0), false));
        });
    }

    group.finish();
}

fn bench_git_module(c: &mut Criterion) {
    use prmt::Module;
    use prmt::modules::git::GitModule;

    let mut group = c.benchmark_group("git_module");

    let module = GitModule::new();
    let context = ctx(false, None, &[".git"]);

    group.bench_function("branch_only", |b| {
        b.iter(|| module.render(black_box("short"), black_box(&context)));
    });

    group.bench_function("with_status", |b| {
        b.iter(|| module.render(black_box("full"), black_box(&context)));
    });

    group.finish();
}

fn bench_version_modules(c: &mut Criterion) {
    use prmt::Module;

    let mut group = c.benchmark_group("version_modules");

    let context_no_version = ctx(true, None, &["Cargo.toml"]);
    let context_with_version = ctx(false, None, &["Cargo.toml"]);

    // Benchmark Rust module
    {
        use prmt::modules::rust::RustModule;
        let module = RustModule::new();

        group.bench_function("rust_no_version", |b| {
            b.iter(|| module.render(black_box(""), black_box(&context_no_version)));
        });

        group.bench_function("rust_with_version_memoized", |b| {
            // Warm up memoized value
            let _ = module.render("", &context_with_version);

            b.iter(|| module.render(black_box(""), black_box(&context_with_version)));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parser,
    bench_renderer,
    bench_end_to_end,
    bench_git_module,
    bench_version_modules
);
criterion_main!(benches);

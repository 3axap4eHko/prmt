use anyhow::Result;
use clap::Parser;
use std::env;
use std::process::ExitCode;
use std::time::Instant;

mod cache;
mod error;
mod executor;
mod module_trait;
mod modules;
mod parser;
mod registry;
mod style;

#[derive(Parser)]
#[command(name = "prmt")]
#[command(about = "Ultra-fast customizable shell prompt generator")]
#[command(version)]
struct Cli {
    format: Option<String>,

    #[arg(short = 'f', long)]
    format_flag: Option<String>,

    #[arg(short = 'n', long)]
    no_version: bool,

    #[arg(short = 'd', long)]
    debug: bool,

    #[arg(short = 'b', long)]
    bench: bool,

    #[arg(long)]
    code: Option<i32>,

    #[arg(long)]
    no_color: bool,

    /// Target shell for proper escape sequence handling (zsh, bash, none)
    #[arg(long, value_name = "SHELL")]
    shell: Option<String>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let format = cli
        .format
        .or(cli.format_flag)
        .or_else(|| env::var("PRMT_FORMAT").ok())
        .unwrap_or_else(|| "{path:cyan} {node:green} {git:purple}".to_string());

    // Parse shell type
    let shell = match cli.shell.as_deref() {
        Some(shell_str) => match style::Shell::from_str(shell_str) {
            Ok(shell) => shell,
            Err(e) => {
                eprintln!("Error: {}", e);
                return ExitCode::FAILURE;
            }
        },
        None => style::Shell::None,
    };

    let result = if cli.bench {
        handle_bench(&format, cli.no_version, cli.code, cli.no_color, shell)
    } else {
        handle_format(&format, cli.no_version, cli.debug, cli.code, cli.no_color, shell)
    };

    match result {
        Ok(output) => {
            print!("{}", output);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn handle_format(
    format: &str,
    no_version: bool,
    debug: bool,
    exit_code: Option<i32>,
    no_color: bool,
    shell: style::Shell,
) -> Result<String> {
    if debug {
        let start = Instant::now();
        let output = executor::execute(format, no_version, exit_code, no_color, shell)?;
        let elapsed = start.elapsed();

        eprintln!("Format: {}", format);
        eprintln!("Execution time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

        Ok(output)
    } else {
        executor::execute(format, no_version, exit_code, no_color, shell).map_err(|e| anyhow::anyhow!(e))
    }
}

fn handle_bench(
    format: &str,
    no_version: bool,
    exit_code: Option<i32>,
    no_color: bool,
    shell: style::Shell,
) -> Result<String> {
    let mut times = Vec::new();

    for _ in 0..100 {
        let start = Instant::now();
        let _ = executor::execute(format, no_version, exit_code, no_color, shell)
            .map_err(|e| anyhow::anyhow!(e))?;
        times.push(start.elapsed());
    }

    times.sort();
    let min = times[0];
    let max = times[99];
    let avg: std::time::Duration = times.iter().sum::<std::time::Duration>() / 100;
    let p99 = times[98];

    Ok(format!(
        "100 runs: min={:.2}ms avg={:.2}ms max={:.2}ms p99={:.2}ms\n",
        min.as_secs_f64() * 1000.0,
        avg.as_secs_f64() * 1000.0,
        max.as_secs_f64() * 1000.0,
        p99.as_secs_f64() * 1000.0
    ))
}

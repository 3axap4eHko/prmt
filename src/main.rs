use std::env;
#[cfg(target_os = "linux")]
use std::fs;
use std::process::ExitCode;
use std::str::FromStr;
use std::time::Instant;

mod detector;
mod error;
mod executor;
mod memo;
mod module_trait;
mod modules;
mod parser;
mod registry;
mod style;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const HELP: &str = "\
prmt - Ultra-fast customizable shell prompt generator

USAGE:
    prmt [OPTIONS] [FORMAT]

ARGS:
    <FORMAT>           Format string (default from PRMT_FORMAT env var)

OPTIONS:
    -f, --format <FORMAT>    Format string
    -n, --no-version        Skip version detection for speed
    -d, --debug             Show debug information and timing
    -b, --bench             Run benchmark (100 iterations)
        --code <CODE>       Exit code of the last command (for ok/fail modules)
        --no-color          Disable colored output
        --shell <SHELL>     Wrap ANSI escapes for the specified shell (bash, zsh, none)
    -h, --help             Print help
    -V, --version          Print version
";

struct Cli {
    format: Option<String>,
    no_version: bool,
    debug: bool,
    bench: bool,
    code: Option<i32>,
    no_color: bool,
    shell: Option<style::Shell>,
}

fn parse_args() -> Result<Cli, lexopt::Error> {
    use lexopt::prelude::*;

    let mut format = None;
    let mut no_version = false;
    let mut debug = false;
    let mut bench = false;
    let mut code = None;
    let mut no_color = false;
    let mut shell = None;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('h') | Long("help") => {
                print!("{}", HELP);
                std::process::exit(0);
            }
            Short('V') | Long("version") => {
                println!("prmt {}", VERSION);
                std::process::exit(0);
            }
            Short('f') | Long("format") => {
                format = Some(parser.value()?.string()?);
            }
            Short('n') | Long("no-version") => {
                no_version = true;
            }
            Short('d') | Long("debug") => {
                debug = true;
            }
            Short('b') | Long("bench") => {
                bench = true;
            }
            Long("code") => {
                code = Some(parser.value()?.parse()?);
            }
            Long("no-color") => {
                no_color = true;
            }
            Long("shell") => {
                let value = parser.value()?.string()?;
                shell = Some(style::Shell::from_str(&value)?);
            }
            Value(val) => {
                if format.is_none() {
                    format = Some(val.string()?);
                }
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Cli {
        format,
        no_version,
        debug,
        bench,
        code,
        no_color,
        shell,
    })
}

fn shell_from_name(value: &str) -> Option<style::Shell> {
    let trimmed = value.trim().trim_end_matches('\0').trim_start_matches('-');
    let name = trimmed.rsplit('/').next().unwrap_or(trimmed);
    match name {
        "zsh" => Some(style::Shell::Zsh),
        "bash" => Some(style::Shell::Bash),
        _ => None,
    }
}

fn detect_shell_from_env() -> Option<style::Shell> {
    if env::var("ZSH_VERSION").is_ok() {
        return Some(style::Shell::Zsh);
    }

    if env::var("BASH_VERSION").is_ok() {
        return Some(style::Shell::Bash);
    }

    if let Ok(shell_path) = env::var("SHELL")
        && let Some(shell) = shell_from_name(&shell_path)
    {
        return Some(shell);
    }

    None
}

#[cfg(target_os = "linux")]
fn detect_shell_from_parent_process() -> Option<style::Shell> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let ppid_line = status.lines().find(|line| line.starts_with("PPid:"))?;
    let ppid = ppid_line.split_whitespace().nth(1)?.parse::<u32>().ok()?;
    if ppid == 0 {
        return None;
    }

    let comm = fs::read_to_string(format!("/proc/{}/comm", ppid)).ok()?;
    if let Some(shell) = shell_from_name(&comm) {
        return Some(shell);
    }

    let cmdline = fs::read_to_string(format!("/proc/{}/cmdline", ppid)).ok()?;
    let first = cmdline.split('\0').next().unwrap_or("");
    shell_from_name(first)
}

#[cfg(not(target_os = "linux"))]
fn detect_shell_from_parent_process() -> Option<style::Shell> {
    None
}

fn resolve_shell(cli_shell: Option<style::Shell>) -> style::Shell {
    if let Some(shell) = cli_shell {
        return shell;
    }

    if let Some(shell) = detect_shell_from_env() {
        return shell;
    }

    detect_shell_from_parent_process().unwrap_or(style::Shell::None)
}

fn main() -> ExitCode {
    let cli = match parse_args() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("Try 'prmt --help' for more information.");
            return ExitCode::FAILURE;
        }
    };

    let format = cli
        .format
        .or_else(|| env::var("PRMT_FORMAT").ok())
        .unwrap_or_else(|| "{path:cyan} {node:green} {git:purple}".to_string());

    let shell = resolve_shell(cli.shell);

    let result = if cli.bench {
        handle_bench(&format, cli.no_version, cli.code, cli.no_color, shell)
    } else {
        handle_format(
            &format,
            cli.no_version,
            cli.debug,
            cli.code,
            cli.no_color,
            shell,
        )
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
) -> error::Result<String> {
    if debug {
        let start = Instant::now();
        let output = executor::execute_with_shell(format, no_version, exit_code, no_color, shell)?;
        let elapsed = start.elapsed();

        eprintln!("Format: {}", format);
        eprintln!("Execution time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);

        Ok(output)
    } else {
        executor::execute_with_shell(format, no_version, exit_code, no_color, shell)
    }
}

fn handle_bench(
    format: &str,
    no_version: bool,
    exit_code: Option<i32>,
    no_color: bool,
    shell: style::Shell,
) -> error::Result<String> {
    let mut times = Vec::new();

    for _ in 0..100 {
        let start = Instant::now();
        let _ = executor::execute_with_shell(format, no_version, exit_code, no_color, shell)?;
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

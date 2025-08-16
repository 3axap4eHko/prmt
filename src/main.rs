use clap::Parser;
use std::env;
use std::process;
use std::time::Instant;

mod module_trait;
mod style;
mod registry;
mod parser;
mod executor;
mod modules;

#[derive(Parser)]
#[command(name = "prmt")]
#[command(about = "Ultra-fast customizable shell prompt generator")]
#[command(version)]
struct Cli {
    format: Option<String>,
    
    #[arg(short = 'f', long)]
    format_flag: Option<String>,
    
    #[arg(long)]
    no_version: bool,
    
    #[arg(long)]
    debug: bool,
    
    #[arg(long)]
    bench: bool,
    
    #[arg(long)]
    code: Option<i32>,
}

fn main() {
    let cli = Cli::parse();
    
    let format = cli.format
        .or(cli.format_flag)
        .or_else(|| env::var("PRMT_FORMAT").ok())
        .unwrap_or_else(|| "{path:cyan} {node:green} {git:purple}".to_string());
    
    let result = if cli.bench {
        handle_bench(&format, cli.no_version, cli.code)
    } else {
        handle_format(&format, cli.no_version, cli.debug, cli.code)
    };
    
    match result {
        Ok(output) => print!("{}", output),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

fn handle_format(format: &str, no_version: bool, debug: bool, exit_code: Option<i32>) -> Result<String, String> {
    if debug {
        let start = Instant::now();
        let output = executor::execute(format, no_version, exit_code)?;
        let elapsed = start.elapsed();
        
        eprintln!("Format: {}", format);
        eprintln!("Execution time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        
        Ok(output)
    } else {
        executor::execute(format, no_version, exit_code)
    }
}

fn handle_bench(format: &str, no_version: bool, exit_code: Option<i32>) -> Result<String, String> {
    let mut times = Vec::new();
    
    for _ in 0..100 {
        let start = Instant::now();
        let _ = executor::execute(format, no_version, exit_code)?;
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
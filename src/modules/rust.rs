use crate::error::Result;
use crate::memo::{RUST_VERSION, memoized_version};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use dirs::home_dir;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use toml::Value;

pub struct RustModule;

impl Default for RustModule {
    fn default() -> Self {
        Self::new()
    }
}

impl RustModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for RustModule {
    fn fs_markers(&self) -> &'static [&'static str] {
        &["Cargo.toml"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if context.marker_path("Cargo.toml").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some(String::new()));
        }

        let normalized_format = utils::validate_version_format(format, "rust")?;

        let version = match memoized_version(&RUST_VERSION, get_rust_version) {
            Some(v) => v,
            None => return Ok(None),
        };
        let version_str = version.as_ref();

        match normalized_format {
            "full" => Ok(Some(version_str.to_string())),
            "short" => Ok(Some(utils::shorten_version(version_str))),
            "major" => Ok(version_str.split('.').next().map(|s| s.to_string())),
            _ => unreachable!("validate_version_format should have caught this"),
        }
    }
}

#[derive(Default)]
struct RustupSettings {
    default_toolchain: Option<String>,
    default_host_triple: Option<String>,
    overrides: HashMap<PathBuf, String>,
}

impl RustupSettings {
    fn default_toolchain(&self) -> Option<&str> {
        self.default_toolchain.as_deref()
    }

    fn default_host_triple(&self) -> Option<&str> {
        self.default_host_triple.as_deref()
    }

    fn lookup_override(&self, cwd: &Path) -> Option<String> {
        let mut best: Option<(usize, &String)> = None;

        for (path, toolchain) in &self.overrides {
            if cwd.starts_with(path) {
                let depth = path.components().count();
                match best {
                    Some((best_depth, _)) if best_depth >= depth => {}
                    _ => best = Some((depth, toolchain)),
                }
            }
        }

        best.map(|(_, toolchain)| toolchain.clone())
    }
}

static RUSTUP_SETTINGS: OnceLock<RustupSettings> = OnceLock::new();
static TOOLCHAIN_OVERRIDE: OnceLock<Option<String>> = OnceLock::new();

fn rustup_settings() -> &'static RustupSettings {
    RUSTUP_SETTINGS.get_or_init(load_rustup_settings)
}

fn toolchain_override() -> Option<String> {
    TOOLCHAIN_OVERRIDE
        .get_or_init(compute_toolchain_override)
        .clone()
}

fn get_rust_version() -> Option<String> {
    let settings = rustup_settings();

    if let Some(toolchain) = toolchain_override()
        && let Some(version) = run_rustc_for_toolchain(&toolchain, settings)
    {
        return Some(version);
    }

    run_plain_rustc()
}

fn run_rustc_for_toolchain(toolchain: &str, settings: &RustupSettings) -> Option<String> {
    if let Some(path) = resolve_rustc_path(toolchain, settings) {
        let mut cmd = Command::new(path);
        cmd.arg("--version");
        if let Some(output) = run_command(cmd) {
            return parse_rustc_version(&output);
        }
    }

    let mut cmd = Command::new("rustup");
    cmd.args(["run", toolchain, "rustc", "--version"]);
    run_command(cmd).and_then(|out| parse_rustc_version(&out))
}

fn run_plain_rustc() -> Option<String> {
    let mut cmd = Command::new("rustc");
    cmd.arg("--version");
    run_command(cmd).and_then(|out| parse_rustc_version(&out))
}

fn run_command(mut command: Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn parse_rustc_version(stdout: &str) -> Option<String> {
    stdout.split_whitespace().nth(1).map(|s| s.to_string())
}

fn compute_toolchain_override() -> Option<String> {
    if let Ok(toolchain) = env::var("RUSTUP_TOOLCHAIN") {
        let trimmed = toolchain.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let settings = rustup_settings();
    let cwd = env::current_dir().ok();

    if let Some(ref dir) = cwd {
        if let Some(toolchain) = settings.lookup_override(dir) {
            return Some(toolchain);
        }
        if let Some(toolchain) = find_toolchain_file(dir) {
            return Some(toolchain);
        }
    }

    settings.default_toolchain().map(|s| s.to_string())
}

fn load_rustup_settings() -> RustupSettings {
    let mut settings = RustupSettings::default();

    let Some(home) = rustup_home() else {
        return settings;
    };

    let path = home.join("settings.toml");
    let Ok(contents) = fs::read_to_string(path) else {
        return settings;
    };

    let Ok(value) = toml::from_str::<Value>(&contents) else {
        return settings;
    };

    if value
        .get("version")
        .and_then(Value::as_str)
        .map(|v| v != "12")
        .unwrap_or(false)
    {
        return settings;
    }

    if let Some(default_toolchain) = value.get("default_toolchain").and_then(Value::as_str) {
        let toolchain = default_toolchain.trim();
        if !toolchain.is_empty() {
            settings.default_toolchain = Some(toolchain.to_string());
        }
    }

    if let Some(host) = value.get("default_host_triple").and_then(Value::as_str) {
        let host = host.trim();
        if !host.is_empty() {
            settings.default_host_triple = Some(host.to_string());
        }
    }

    if let Some(overrides) = value.get("overrides").and_then(Value::as_table) {
        for (path, toolchain) in overrides {
            if let Some(toolchain) = toolchain.as_str() {
                let trimmed = toolchain.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let pathbuf = PathBuf::from(path);
                let canonical = fs::canonicalize(&pathbuf).unwrap_or(pathbuf);
                settings.overrides.insert(canonical, trimmed.to_string());
            }
        }
    }

    settings
}

fn resolve_rustc_path(toolchain: &str, settings: &RustupSettings) -> Option<PathBuf> {
    let home = rustup_home()?;
    let bin = rustc_binary_name();

    let base = home.join("toolchains");
    let direct = base.join(toolchain).join("bin").join(bin);
    if direct.exists() {
        return Some(direct);
    }

    if toolchain.contains('-') {
        return None;
    }

    let host_candidate = settings
        .default_host_triple()
        .map(|s| s.to_string())
        .or_else(|| env::var("HOST").ok());

    if let Some(host) = host_candidate {
        let alt = base
            .join(format!("{toolchain}-{host}"))
            .join("bin")
            .join(bin);
        if alt.exists() {
            return Some(alt);
        }
    }

    None
}

fn find_toolchain_file(start: &Path) -> Option<String> {
    let mut dir = Some(start);
    while let Some(current) = dir {
        if let Some(toolchain) = read_toolchain_file(&current.join("rust-toolchain"), false) {
            return Some(toolchain);
        }
        if let Some(toolchain) = read_toolchain_file(&current.join("rust-toolchain.toml"), true) {
            return Some(toolchain);
        }
        dir = current.parent();
    }
    None
}

fn read_toolchain_file(path: &Path, toml_only: bool) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return None;
    }

    if !toml_only && !trimmed.contains('\n') {
        return Some(trimmed.to_string());
    }

    let value: Value = toml::from_str(&contents).ok()?;
    match value.get("toolchain") {
        Some(Value::Table(table)) => table
            .get("channel")
            .and_then(Value::as_str)
            .map(|s| s.trim().to_string()),
        Some(Value::String(channel)) => {
            let channel = channel.trim();
            if channel.is_empty() {
                None
            } else {
                Some(channel.to_string())
            }
        }
        _ => None,
    }
}

fn rustup_home() -> Option<PathBuf> {
    if let Ok(path) = env::var("RUSTUP_HOME") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    home_dir().map(|dir| dir.join(".rustup"))
}

fn rustc_binary_name() -> &'static str {
    if cfg!(windows) { "rustc.exe" } else { "rustc" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn parses_rustc_version() {
        let input = "rustc 1.76.0 (a58dcd2a3 2024-01-17)";
        assert_eq!(parse_rustc_version(input), Some("1.76.0".to_string()));
    }

    #[test]
    fn read_plain_toolchain_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rust-toolchain");
        fs::write(&path, "stable\n").unwrap();
        assert_eq!(
            read_toolchain_file(&path, false),
            Some("stable".to_string())
        );
    }

    #[test]
    fn read_toml_toolchain_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rust-toolchain.toml");
        let mut file = fs::File::create(&path).unwrap();
        writeln!(
            file,
            "[toolchain]\nchannel = \"nightly-x86_64-unknown-linux-gnu\""
        )
        .unwrap();
        assert_eq!(
            read_toolchain_file(&path, true),
            Some("nightly-x86_64-unknown-linux-gnu".to_string())
        );
    }
}

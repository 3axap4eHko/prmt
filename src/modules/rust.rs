use crate::cache::VERSION_CACHE;
use crate::error::Result;
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use dirs::home_dir;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
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

#[cold]
fn get_rust_version() -> Option<String> {
    if let Some(toolchain) = detect_toolchain() {
        if let Some(version) = try_direct_rustc(&toolchain) {
            return Some(version);
        }

        if let DetectedToolchain::Rustup { name, .. } = &toolchain
            && let Some(version) = try_rustup_run(name)
        {
            return Some(version);
        }
    }

    execute_version_command({
        let mut command = Command::new("rustc");
        command.arg("--version");
        command
    })
}

impl Module for RustModule {
    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        if utils::find_upward("Cargo.toml").is_none() {
            return Ok(None);
        }

        if context.no_version {
            return Ok(Some("rust".to_string()));
        }

        // Validate and normalize format
        let normalized_format = utils::validate_version_format(format, "rust")?;

        // Check cache first
        let cache_key = "rust_version";
        let version = if let Some(cached) = VERSION_CACHE.get(cache_key) {
            match cached {
                Some(v) => v,
                None => return Ok(None),
            }
        } else {
            // Get version with timeout consideration
            let version = get_rust_version();
            VERSION_CACHE.insert(
                cache_key.to_string(),
                version.clone(),
                Duration::from_secs(300),
            );
            match version {
                Some(v) => v,
                None => return Ok(None),
            }
        };

        match normalized_format {
            "full" => Ok(Some(version)),
            "short" => {
                let parts: Vec<&str> = version.split('.').collect();
                if parts.len() >= 2 {
                    Ok(Some(format!("{}.{}", parts[0], parts[1])))
                } else {
                    Ok(Some(version))
                }
            }
            "major" => Ok(version.split('.').next().map(|s| s.to_string())),
            _ => unreachable!("validate_version_format should have caught this"),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct RustupMetadata {
    rustup_home: Option<PathBuf>,
    default_toolchain: Option<String>,
    default_host_triple: Option<String>,
    overrides: Vec<(PathBuf, String)>,
}

#[derive(Clone, Debug)]
enum DetectedToolchain {
    Rustup {
        name: String,
        rustup_home: Option<PathBuf>,
        host_triple: Option<String>,
    },
    Custom {
        root: PathBuf,
    },
}

#[derive(Clone, Debug)]
enum ToolchainDirective {
    Channel(String),
    Path(PathBuf),
}

fn detect_toolchain() -> Option<DetectedToolchain> {
    let metadata = RustupMetadata::load();

    if let Ok(toolchain) = env::var("RUSTUP_TOOLCHAIN") {
        let trimmed = toolchain.trim();
        if !trimmed.is_empty() {
            return Some(DetectedToolchain::Rustup {
                name: trimmed.to_string(),
                rustup_home: metadata.rustup_home.clone(),
                host_triple: metadata.default_host_triple.clone(),
            });
        }
    }

    if let Some(directive) = find_rust_toolchain_directive() {
        return match directive {
            ToolchainDirective::Channel(channel) => Some(DetectedToolchain::Rustup {
                name: channel,
                rustup_home: metadata.rustup_home.clone(),
                host_triple: metadata.default_host_triple.clone(),
            }),
            ToolchainDirective::Path(path) => Some(DetectedToolchain::Custom { root: path }),
        };
    }

    if let Ok(current_dir) = env::current_dir()
        && let Some(toolchain) = metadata.override_for(&current_dir)
    {
        return Some(DetectedToolchain::Rustup {
            name: toolchain,
            rustup_home: metadata.rustup_home.clone(),
            host_triple: metadata.default_host_triple.clone(),
        });
    }

    metadata
        .default_toolchain
        .as_ref()
        .map(|default| DetectedToolchain::Rustup {
            name: default.clone(),
            rustup_home: metadata.rustup_home.clone(),
            host_triple: metadata.default_host_triple.clone(),
        })
}

fn try_direct_rustc(toolchain: &DetectedToolchain) -> Option<String> {
    let rustc_path = match toolchain {
        DetectedToolchain::Custom { root } => {
            let bin = root.join("bin").join(rustc_binary_name());
            if bin.exists() { Some(bin) } else { None }
        }
        DetectedToolchain::Rustup {
            name,
            rustup_home: Some(home),
            host_triple,
        } => resolve_rustup_rustc(home, name, host_triple.as_deref()),
        DetectedToolchain::Rustup { .. } => None,
    }?;

    execute_version_command({
        let mut command = Command::new(rustc_path);
        command.arg("--version");
        command
    })
}

fn try_rustup_run(toolchain: &str) -> Option<String> {
    execute_version_command({
        let mut command = Command::new("rustup");
        command
            .arg("run")
            .arg(toolchain)
            .arg("rustc")
            .arg("--version");
        command
    })
}

fn execute_version_command(mut command: Command) -> Option<String> {
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }

    parse_rustc_version(&output.stdout)
}

fn parse_rustc_version(stdout: &[u8]) -> Option<String> {
    let version_str = String::from_utf8_lossy(stdout);
    version_str.split_whitespace().nth(1).map(|s| s.to_string())
}

fn resolve_rustup_rustc(
    rustup_home: &Path,
    toolchain: &str,
    host_triple: Option<&str>,
) -> Option<PathBuf> {
    let bin_name = rustc_binary_name();
    let toolchains_dir = rustup_home.join("toolchains");

    let direct = toolchains_dir.join(toolchain).join("bin").join(bin_name);
    if direct.exists() {
        return Some(direct);
    }

    if let Some(host) = host_triple {
        let host_candidate = toolchains_dir
            .join(format!("{toolchain}-{host}"))
            .join("bin")
            .join(bin_name);
        if host_candidate.exists() {
            return Some(host_candidate);
        }
    }

    if let Ok(entries) = fs::read_dir(&toolchains_dir) {
        let mut candidates: Vec<(String, PathBuf)> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name();
                let name_str = name.to_str()?;
                if !name_str.starts_with(toolchain) {
                    return None;
                }

                let remaining = &name_str[toolchain.len()..];
                if remaining.is_empty() || remaining.starts_with('-') {
                    let candidate = entry.path().join("bin").join(bin_name);
                    if candidate.exists() {
                        return Some((name_str.to_string(), candidate));
                    }
                }

                None
            })
            .collect();

        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        if let Some((_, path)) = candidates.into_iter().next() {
            return Some(path);
        }
    }

    None
}

fn rustc_binary_name() -> &'static str {
    if cfg!(windows) { "rustc.exe" } else { "rustc" }
}

impl RustupMetadata {
    fn load() -> Self {
        let rustup_home = rustup_home();
        let mut metadata = RustupMetadata {
            rustup_home: rustup_home.clone(),
            ..Default::default()
        };

        if let Some(home) = rustup_home {
            let settings_path = home.join("settings.toml");
            if let Ok(contents) = fs::read_to_string(settings_path)
                && let Ok(value) = contents.parse::<Value>()
            {
                if let Some(default_toolchain) =
                    value.get("default_toolchain").and_then(Value::as_str)
                {
                    metadata.default_toolchain = Some(default_toolchain.trim().to_string());
                }

                if let Some(host_triple) = value.get("default_host_triple").and_then(Value::as_str)
                {
                    metadata.default_host_triple = Some(host_triple.trim().to_string());
                }

                if let Some(overrides) = value.get("overrides").and_then(Value::as_table) {
                    metadata.overrides = overrides
                        .iter()
                        .filter_map(|(path, toolchain)| {
                            toolchain
                                .as_str()
                                .map(|tc| (PathBuf::from(path), tc.to_string()))
                        })
                        .collect();
                }
            }
        }

        metadata
    }

    fn override_for(&self, dir: &Path) -> Option<String> {
        let mut best_match: Option<(usize, &String)> = None;
        for (path, toolchain) in &self.overrides {
            if dir.starts_with(path) {
                let depth = path.components().count();
                match best_match {
                    Some((best_depth, _)) if depth <= best_depth => {}
                    _ => best_match = Some((depth, toolchain)),
                }
            }
        }

        best_match.map(|(_, toolchain)| toolchain.clone())
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

fn find_rust_toolchain_directive() -> Option<ToolchainDirective> {
    if let Some(path) = utils::find_upward("rust-toolchain.toml")
        && let Some(directive) = parse_rust_toolchain_toml(&path)
    {
        return Some(directive);
    }

    if let Some(path) = utils::find_upward("rust-toolchain") {
        return parse_rust_toolchain_toml(&path).or_else(|| parse_rust_toolchain_plain(&path));
    }

    None
}

fn parse_rust_toolchain_toml(path: &Path) -> Option<ToolchainDirective> {
    let contents = fs::read_to_string(path).ok()?;
    let value: Value = toml::from_str(&contents).ok()?;

    match value.get("toolchain") {
        Some(Value::Table(table)) => {
            if let Some(path_str) = table.get("path").and_then(Value::as_str) {
                let resolved = resolve_toolchain_path(path, path_str);
                return Some(ToolchainDirective::Path(resolved));
            }

            table
                .get("channel")
                .and_then(Value::as_str)
                .map(|channel| ToolchainDirective::Channel(channel.trim().to_string()))
        }
        Some(Value::String(channel)) => {
            Some(ToolchainDirective::Channel(channel.trim().to_string()))
        }
        _ => None,
    }
}

fn parse_rust_toolchain_plain(path: &Path) -> Option<ToolchainDirective> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        return Some(ToolchainDirective::Channel(trimmed.to_string()));
    }
    None
}

fn resolve_toolchain_path(file_path: &Path, entry: &str) -> PathBuf {
    let base = file_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let candidate = {
        let path = Path::new(entry);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            base.join(path)
        }
    };

    fs::canonicalize(&candidate).unwrap_or(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn parses_plain_rust_toolchain_channel() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("rust-toolchain");
        fs::write(&file_path, "stable\n").unwrap();

        let directive = parse_rust_toolchain_plain(&file_path);
        match directive {
            Some(ToolchainDirective::Channel(channel)) => assert_eq!(channel, "stable"),
            _ => panic!("expected channel directive"),
        }
    }

    #[test]
    fn parses_toml_rust_toolchain_channel() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("rust-toolchain.toml");
        fs::write(
            &file_path,
            r#"
[toolchain]
channel = "nightly"
"#,
        )
        .unwrap();

        let directive = parse_rust_toolchain_toml(&file_path);
        match directive {
            Some(ToolchainDirective::Channel(channel)) => assert_eq!(channel, "nightly"),
            _ => panic!("expected channel directive"),
        }
    }

    #[test]
    fn parses_toml_rust_toolchain_path() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("rust-toolchain.toml");
        fs::write(
            &file_path,
            r#"
[toolchain]
path = "./custom"
"#,
        )
        .unwrap();

        let directive = parse_rust_toolchain_toml(&file_path);
        match directive {
            Some(ToolchainDirective::Path(path)) => {
                assert!(
                    path.ends_with("custom"),
                    "path should resolve relative directory"
                );
            }
            _ => panic!("expected path directive"),
        }
    }

    #[test]
    fn resolves_rustup_rustc_with_prefix_match() {
        let dir = tempdir().unwrap();
        let toolchains_dir = dir.path().join("toolchains");
        fs::create_dir_all(toolchains_dir.join("stable-x86_64-unknown-linux-gnu/bin")).unwrap();
        let rustc_path = toolchains_dir
            .join("stable-x86_64-unknown-linux-gnu")
            .join("bin")
            .join(rustc_binary_name());
        let mut file = fs::File::create(&rustc_path).unwrap();
        file.write_all(b"").unwrap();

        let resolved =
            resolve_rustup_rustc(dir.path(), "stable", Some("x86_64-unknown-linux-gnu")).unwrap();
        assert_eq!(resolved, rustc_path);
    }
}

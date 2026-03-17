use crate::error::{PromptError, Result};
use crate::memo::{GIT_MEMO, GitInfo};
use crate::module_trait::{Module, ModuleContext};
use bitflags::bitflags;
#[cfg(feature = "git-gix")]
use gix::bstr::{BString, ByteSlice};
#[cfg(feature = "git-gix")]
use gix::dir::entry::Status as DirEntryStatus;
#[cfg(feature = "git-gix")]
use gix::dir::walk::EmissionMode as DirwalkEmissionMode;
#[cfg(feature = "git-gix")]
use gix::progress::Discard;
#[cfg(feature = "git-gix")]
use gix::status::Item as StatusItem;
#[cfg(feature = "git-gix")]
use gix::status::index_worktree::Item as IndexWorktreeItem;
#[cfg(feature = "git-gix")]
use gix::status::plumbing::index_as_worktree::EntryStatus as IndexEntryStatus;
use rayon::join;
use std::path::Path;
use std::process::Command;
#[cfg(feature = "git-gix")]
use std::sync::Arc;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct GitStatus: u8 {
        const MODIFIED = 0b001;
        const STAGED = 0b010;
        const UNTRACKED = 0b100;
    }
}

#[derive(Clone, Copy, Debug)]
enum GitMode {
    Full,
    Short,
}

#[derive(Debug)]
struct GitFormat {
    mode: GitMode,
    owned_only: bool,
}

pub struct GitModule;

impl Default for GitModule {
    fn default() -> Self {
        Self::new()
    }
}

impl GitModule {
    pub fn new() -> Self {
        Self
    }
}

#[cold]
fn get_git_status_slow(repo_root: &Path) -> GitStatus {
    let mut status = GitStatus::empty();

    // Only run git status if not memoized
    if let Ok(output) = std::process::Command::new("git")
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--untracked-files=normal")
        .current_dir(repo_root)
        .output()
        && output.status.success()
    {
        let status_text = String::from_utf8_lossy(&output.stdout);

        for line in status_text.lines() {
            if line.starts_with("??") {
                status |= GitStatus::UNTRACKED;
            } else if !line.is_empty() {
                let bytes = line.as_bytes();
                if bytes.len() >= 2 {
                    if bytes[0] != b' ' && bytes[0] != b'?' {
                        status |= GitStatus::STAGED;
                    }
                    if bytes[1] != b' ' && bytes[1] != b'?' {
                        status |= GitStatus::MODIFIED;
                    }
                }
            }
        }
    }
    status
}

#[cfg(feature = "git-gix")]
fn dir_has_files(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if !ft.is_dir() {
            return true;
        }
        if dir_has_files(&entry.path()) {
            return true;
        }
    }
    false
}

#[cfg(feature = "git-gix")]
fn collect_git_status_fast(repo: &gix::Repository) -> Option<GitStatus> {
    let mut status = GitStatus::empty();
    let workdir = repo.workdir()?;

    let platform = repo
        .status(Discard)
        .ok()?
        .dirwalk_options(|opts| opts.emit_ignored(Some(DirwalkEmissionMode::CollapseDirectory)));
    let iter = platform.into_iter(Vec::<BString>::new()).ok()?;

    for item in iter {
        let item = item.ok()?;
        match item {
            StatusItem::IndexWorktree(change) => match change {
                IndexWorktreeItem::DirectoryContents { entry, .. } => {
                    if matches!(entry.status, DirEntryStatus::Untracked) {
                        let full = workdir.join(entry.rela_path.to_str_lossy().as_ref());
                        if !full.is_dir() || dir_has_files(&full) {
                            status |= GitStatus::UNTRACKED;
                        }
                    }
                }
                IndexWorktreeItem::Modification {
                    status: entry_status,
                    ..
                } => match entry_status {
                    IndexEntryStatus::IntentToAdd => status |= GitStatus::STAGED,
                    IndexEntryStatus::NeedsUpdate(_) => {}
                    IndexEntryStatus::Conflict { .. } | IndexEntryStatus::Change(_) => {
                        status |= GitStatus::MODIFIED;
                    }
                },
                IndexWorktreeItem::Rewrite { .. } => {
                    status |= GitStatus::MODIFIED;
                }
            },
            StatusItem::TreeIndex(_) => {
                status |= GitStatus::STAGED;
            }
        }

        if status.contains(GitStatus::MODIFIED)
            && status.contains(GitStatus::STAGED)
            && status.contains(GitStatus::UNTRACKED)
        {
            break;
        }
    }

    Some(status)
}

#[cfg(feature = "git-gix")]
fn current_branch_from_repo(repo: &gix::Repository) -> String {
    if let Ok(Some(head_ref)) = repo.head_ref() {
        String::from_utf8(head_ref.name().shorten().to_vec()).unwrap_or_else(|_| "HEAD".to_string())
    } else if let Ok(Some(head_name)) = repo.head_name() {
        String::from_utf8(head_name.shorten().to_vec()).unwrap_or_else(|_| "HEAD".to_string())
    } else if let Ok(head) = repo.head() {
        head.id()
            .map(|id| id.shorten_or_id().to_string())
            .unwrap_or_else(|| "HEAD".to_string())
    } else {
        "HEAD".to_string()
    }
}

fn current_branch_from_cli(repo_root: &Path) -> Option<String> {
    run_git(&["symbolic-ref", "--quiet", "--short", "HEAD"], repo_root)
        .or_else(|| run_git(&["rev-parse", "--short", "HEAD"], repo_root))
}

fn branch_and_status_cli(repo_root: &Path, need_status: bool) -> (String, GitStatus) {
    if need_status {
        join(
            || current_branch_from_cli(repo_root).unwrap_or_else(|| "HEAD".to_string()),
            || get_git_status_slow(repo_root),
        )
    } else {
        (
            current_branch_from_cli(repo_root).unwrap_or_else(|| "HEAD".to_string()),
            GitStatus::empty(),
        )
    }
}

#[cfg(feature = "git-gix")]
fn branch_and_status(repo_root: &Path, need_status: bool) -> (String, GitStatus) {
    match gix::ThreadSafeRepository::open(repo_root) {
        Ok(repo) => {
            let repo = Arc::new(repo);
            if need_status {
                let repo_for_branch = Arc::clone(&repo);
                let repo_for_status = Arc::clone(&repo);
                let repo_root_for_status = repo_root;
                join(
                    || {
                        let local = repo_for_branch.to_thread_local();
                        current_branch_from_repo(&local)
                    },
                    || {
                        let local = repo_for_status.to_thread_local();
                        collect_git_status_fast(&local)
                            .unwrap_or_else(|| get_git_status_slow(repo_root_for_status))
                    },
                )
            } else {
                let local = repo.to_thread_local();
                (current_branch_from_repo(&local), GitStatus::empty())
            }
        }
        Err(_) => branch_and_status_cli(repo_root, need_status),
    }
}

#[cfg(not(feature = "git-gix"))]
fn branch_and_status(repo_root: &Path, need_status: bool) -> (String, GitStatus) {
    branch_and_status_cli(repo_root, need_status)
}

fn run_git(args: &[&str], repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn parse_git_format(format: &str) -> Result<GitFormat> {
    let mut mode = None;
    let mut owned_only = false;

    for part in format.split('+') {
        if part.is_empty() {
            continue;
        }

        match part {
            "full" | "f" => mode = Some(GitMode::Full),
            "short" | "s" => mode = Some(GitMode::Short),
            "owned" | "o" | "owned-only" | "owned_only" => owned_only = true,
            _ => {
                return Err(PromptError::InvalidFormat {
                    module: "git".to_string(),
                    format: format.to_string(),
                    valid_formats: "full, f, short, s, +o, +owned".to_string(),
                });
            }
        }
    }

    Ok(GitFormat {
        mode: mode.unwrap_or(GitMode::Full),
        owned_only,
    })
}

fn is_repo_owned_by_user(repo_root: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let Ok(metadata) = std::fs::metadata(repo_root) else {
            return false;
        };
        let current_uid = unsafe { libc::geteuid() };
        metadata.uid() == current_uid
    }

    #[cfg(not(unix))]
    {
        let _ = repo_root;
        true
    }
}

impl Module for GitModule {
    fn fs_markers(&self) -> &'static [&'static str] {
        &[".git"]
    }

    fn render(&self, format: &str, context: &ModuleContext) -> Result<Option<String>> {
        let format = parse_git_format(format)?;

        // Fast path: find git directory
        let git_dir = match context.marker_path(".git") {
            Some(path) => path,
            None => return Ok(None),
        };
        let repo_root = match git_dir.parent() {
            Some(p) => p,
            None => return Ok(None),
        };

        if format.owned_only && !is_repo_owned_by_user(repo_root) {
            return Ok(None);
        }

        // Check memoized info first
        if let Some(memoized) = GIT_MEMO.get(repo_root) {
            return Ok(match format.mode {
                GitMode::Full => {
                    let mut result = memoized.branch.clone();
                    if memoized.has_changes {
                        result.push('*');
                    }
                    if memoized.has_staged {
                        result.push('+');
                    }
                    if memoized.has_untracked {
                        result.push('?');
                    }
                    Some(result)
                }
                GitMode::Short => Some(memoized.branch),
            });
        }

        let need_status = matches!(format.mode, GitMode::Full);
        let (branch_name, status) = branch_and_status(repo_root, need_status);

        // Memoize the result for other placeholders during this render
        let info = GitInfo {
            branch: branch_name.clone(),
            has_changes: status.contains(GitStatus::MODIFIED),
            has_staged: status.contains(GitStatus::STAGED),
            has_untracked: status.contains(GitStatus::UNTRACKED),
        };
        GIT_MEMO.insert(repo_root.to_path_buf(), info);

        // Build result
        Ok(match format.mode {
            GitMode::Full => {
                let mut result = branch_name;
                if status.contains(GitStatus::MODIFIED) {
                    result.push('*');
                }
                if status.contains(GitStatus::STAGED) {
                    result.push('+');
                }
                if status.contains(GitStatus::UNTRACKED) {
                    result.push('?');
                }
                Some(result)
            }
            GitMode::Short => Some(branch_name),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use tempfile::tempdir;

    struct EnvVarGuard {
        key: String,
        original: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::set_var(key, value);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }

        fn unset(key: &str) -> Self {
            let original = env::var_os(key);
            unsafe {
                env::remove_var(key);
            }
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                unsafe {
                    env::set_var(&self.key, value);
                }
            } else {
                unsafe {
                    env::remove_var(&self.key);
                }
            }
        }
    }

    fn git_init(repo_root: &Path) {
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo_root)
            .status()
            .expect("git init");
        assert!(status.success(), "git init should succeed");
    }

    #[test]
    fn parse_git_format_defaults_to_full() {
        let format = parse_git_format("").expect("format");
        assert!(matches!(format.mode, GitMode::Full));
        assert!(!format.owned_only);
    }

    #[test]
    fn parse_git_format_full_owned() {
        let format = parse_git_format("full+owned").expect("format");
        assert!(matches!(format.mode, GitMode::Full));
        assert!(format.owned_only);
    }

    #[test]
    fn parse_git_format_short_o() {
        let format = parse_git_format("s+o").expect("format");
        assert!(matches!(format.mode, GitMode::Short));
        assert!(format.owned_only);
    }

    #[test]
    fn parse_git_format_rejects_unknown() {
        let err = parse_git_format("full+wat").unwrap_err();
        match err {
            PromptError::InvalidFormat { module, .. } => assert_eq!(module, "git"),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[cfg(feature = "git-gix")]
    #[test]
    fn dir_has_files_returns_false_for_empty_tree() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b/c")).unwrap();
        fs::create_dir_all(tmp.path().join("a/d")).unwrap();
        assert!(!dir_has_files(tmp.path()));
    }

    #[cfg(feature = "git-gix")]
    #[test]
    fn dir_has_files_returns_true_when_file_nested() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b")).unwrap();
        fs::write(tmp.path().join("a/b/file.txt"), "content").unwrap();
        assert!(dir_has_files(tmp.path()));
    }

    #[test]
    fn empty_repo_has_no_untracked_status_in_slow_path() {
        let dir = tempdir().expect("tempdir");
        git_init(dir.path());

        assert!(get_git_status_slow(dir.path()).is_empty());
    }

    #[test]
    #[cfg(feature = "git-gix")]
    fn empty_repo_has_no_untracked_status_in_gix_path() {
        let dir = tempdir().expect("tempdir");
        git_init(dir.path());

        let repo = gix::ThreadSafeRepository::open(dir.path()).expect("open repo");
        let local = repo.to_thread_local();

        assert!(matches!(collect_git_status_fast(&local), Some(status) if status.is_empty()));
    }

    #[test]
    #[cfg(feature = "git-gix")]
    fn untracked_file_sets_untracked_status_in_gix_path() {
        let dir = tempdir().expect("tempdir");
        git_init(dir.path());
        fs::write(dir.path().join("note.txt"), b"scratch").expect("write note");

        let repo = gix::ThreadSafeRepository::open(dir.path()).expect("open repo");
        let local = repo.to_thread_local();

        assert!(matches!(
            collect_git_status_fast(&local),
            Some(status) if status.contains(GitStatus::UNTRACKED)
        ));
    }

    #[test]
    #[cfg(feature = "git-gix")]
    fn empty_dir_tree_not_reported_as_untracked() {
        let dir = tempdir().expect("tempdir");
        git_init(dir.path());
        fs::write(dir.path().join("file.txt"), "hello").unwrap();
        Command::new("git")
            .args(["add", "file.txt"])
            .current_dir(dir.path())
            .status()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(dir.path())
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@test.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@test.com")
            .status()
            .unwrap();

        fs::create_dir_all(dir.path().join("empty/nested/deep")).unwrap();

        let (_, status) = branch_and_status(dir.path(), true);
        assert!(
            !status.contains(GitStatus::UNTRACKED),
            "empty directory tree should not be reported as untracked"
        );
    }

    #[test]
    #[cfg(feature = "git-gix")]
    #[serial]
    fn xdg_ignored_progress_dir_stays_clean_in_gix_path() {
        let home = tempdir().expect("home");
        let ignore_dir = home.path().join(".config/git");
        fs::create_dir_all(&ignore_dir).expect("create ignore dir");
        fs::write(ignore_dir.join("ignore"), b"**/.progress/\n").expect("write ignore file");

        let _home = EnvVarGuard::set("HOME", home.path().to_str().expect("utf8 path"));
        let _xdg = EnvVarGuard::unset("XDG_CONFIG_HOME");
        let _git_config_global = EnvVarGuard::unset("GIT_CONFIG_GLOBAL");

        let dir = tempdir().expect("repo");
        git_init(dir.path());
        fs::create_dir_all(dir.path().join(".progress")).expect("create progress dir");
        fs::write(dir.path().join(".progress/master.md"), b"scratch").expect("write progress file");

        let repo = gix::ThreadSafeRepository::open(dir.path()).expect("open repo");
        let local = repo.to_thread_local();

        assert!(matches!(
            collect_git_status_fast(&local),
            Some(status) if !status.contains(GitStatus::UNTRACKED)
        ));
    }

    #[test]
    #[cfg(feature = "git-gix")]
    #[serial]
    fn xdg_ignored_progress_dir_does_not_set_untracked_status() {
        let home = tempdir().expect("home");
        let ignore_dir = home.path().join(".config/git");
        fs::create_dir_all(&ignore_dir).expect("create ignore dir");
        fs::write(ignore_dir.join("ignore"), b"**/.progress/\n").expect("write ignore file");

        let _home = EnvVarGuard::set("HOME", home.path().to_str().expect("utf8 path"));
        let _xdg = EnvVarGuard::unset("XDG_CONFIG_HOME");
        let _git_config_global = EnvVarGuard::unset("GIT_CONFIG_GLOBAL");

        let dir = tempdir().expect("repo");
        git_init(dir.path());
        fs::create_dir_all(dir.path().join(".progress")).expect("create progress dir");
        fs::write(dir.path().join(".progress/master.md"), b"scratch").expect("write progress file");

        assert!(get_git_status_slow(dir.path()).is_empty());
        assert!(matches!(
            branch_and_status(dir.path(), true),
            (_, status) if !status.contains(GitStatus::UNTRACKED)
        ));
    }
}

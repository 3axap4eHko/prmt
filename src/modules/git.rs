use crate::error::{PromptError, Result};
use crate::memo::{GIT_MEMO, GitInfo};
use crate::module_trait::{Module, ModuleContext};
use bitflags::bitflags;
#[cfg(feature = "git-gix")]
use gix::bstr::BString;
#[cfg(feature = "git-gix")]
use gix::progress::Discard;
#[cfg(feature = "git-gix")]
use gix::status::Item as StatusItem;
#[cfg(feature = "git-gix")]
use gix::status::index_worktree::iter::Summary as WorktreeSummary;
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
fn collect_git_status_fast(repo: &gix::Repository) -> Option<GitStatus> {
    let mut status = GitStatus::empty();

    let platform = repo.status(Discard).ok()?;
    let iter = platform.into_iter(Vec::<BString>::new()).ok()?;

    for item in iter {
        let item = item.ok()?;
        match item {
            StatusItem::IndexWorktree(change) => {
                if let Some(summary) = change.summary() {
                    match summary {
                        WorktreeSummary::Added => status |= GitStatus::UNTRACKED,
                        WorktreeSummary::IntentToAdd => status |= GitStatus::STAGED,
                        WorktreeSummary::Conflict
                        | WorktreeSummary::Copied
                        | WorktreeSummary::Modified
                        | WorktreeSummary::Removed
                        | WorktreeSummary::Renamed
                        | WorktreeSummary::TypeChange => status |= GitStatus::MODIFIED,
                    }
                }
            }
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
}

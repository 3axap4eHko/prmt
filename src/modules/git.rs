use crate::cache::{GIT_CACHE, GitInfo};
use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use bitflags::bitflags;
use gix::bstr::BString;
use gix::progress::Discard;
use gix::status::Item as StatusItem;
use gix::status::index_worktree::iter::Summary as WorktreeSummary;
use rayon::join;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct GitStatus: u8 {
        const MODIFIED = 0b001;
        const STAGED = 0b010;
        const UNTRACKED = 0b100;
    }
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
fn get_git_status_slow(repo_root: &PathBuf) -> GitStatus {
    let mut status = GitStatus::empty();

    // Only run git status if not cached
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
                let chars: Vec<char> = line.chars().take(2).collect();
                if chars.len() >= 2 {
                    if chars[0] != ' ' && chars[0] != '?' {
                        status |= GitStatus::STAGED;
                    }
                    if chars[1] != ' ' && chars[1] != '?' {
                        status |= GitStatus::MODIFIED;
                    }
                }
            }
        }
    }
    status
}

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
        let root_for_branch = repo_root.to_path_buf();
        let root_for_status = repo_root.to_path_buf();
        join(
            || current_branch_from_cli(&root_for_branch).unwrap_or_else(|| "HEAD".to_string()),
            || get_git_status_slow(&root_for_status),
        )
    } else {
        (
            current_branch_from_cli(repo_root).unwrap_or_else(|| "HEAD".to_string()),
            GitStatus::empty(),
        )
    }
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

fn validate_git_format(format: &str) -> Result<&str> {
    match format {
        "" | "full" | "f" => Ok("full"),
        "short" | "s" => Ok("short"),
        _ => Err(PromptError::InvalidFormat {
            module: "git".to_string(),
            format: format.to_string(),
            valid_formats: "full, f, short, s".to_string(),
        }),
    }
}

impl Module for GitModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Result<Option<String>> {
        // Validate format first
        let normalized_format = validate_git_format(format)?;

        // Fast path: find git directory
        let git_dir = match utils::find_upward(".git") {
            Some(d) => d,
            None => return Ok(None),
        };
        let repo_root = match git_dir.parent() {
            Some(p) => p.to_path_buf(),
            None => return Ok(None),
        };

        // Check cache first
        if let Some(cached) = GIT_CACHE.get(&repo_root) {
            return Ok(match normalized_format {
                "full" => {
                    let mut result = cached.branch.clone();
                    if cached.has_changes {
                        result.push('*');
                    }
                    if cached.has_staged {
                        result.push('+');
                    }
                    if cached.has_untracked {
                        result.push('?');
                    }
                    Some(result)
                }
                "short" => Some(cached.branch),
                _ => unreachable!("validate_git_format should have caught this"),
            });
        }

        let need_status = normalized_format == "full";
        let (branch_name, status) = match gix::ThreadSafeRepository::open(&repo_root) {
            Ok(repo) => {
                let repo = Arc::new(repo);
                if need_status {
                    let repo_for_branch = Arc::clone(&repo);
                    let repo_for_status = Arc::clone(&repo);
                    let repo_root_for_status = repo_root.clone();
                    join(
                        || {
                            let local = repo_for_branch.to_thread_local();
                            current_branch_from_repo(&local)
                        },
                        || {
                            let local = repo_for_status.to_thread_local();
                            collect_git_status_fast(&local)
                                .unwrap_or_else(|| get_git_status_slow(&repo_root_for_status))
                        },
                    )
                } else {
                    let local = repo.to_thread_local();
                    (current_branch_from_repo(&local), GitStatus::empty())
                }
            }
            Err(_) => branch_and_status_cli(&repo_root, need_status),
        };

        // Cache the result
        let info = GitInfo {
            branch: branch_name.clone(),
            has_changes: status.contains(GitStatus::MODIFIED),
            has_staged: status.contains(GitStatus::STAGED),
            has_untracked: status.contains(GitStatus::UNTRACKED),
        };
        GIT_CACHE.insert(repo_root, info);

        // Build result
        Ok(match normalized_format {
            "full" => {
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
            "short" => Some(branch_name),
            _ => unreachable!("validate_git_format should have caught this"),
        })
    }
}

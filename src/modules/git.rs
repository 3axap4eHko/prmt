use crate::cache::{GIT_CACHE, GitInfo};
use crate::error::{PromptError, Result};
use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;
use bitflags::bitflags;
use std::path::PathBuf;

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

        // Open repo with minimal operations
        let repo = match gix::open(&repo_root) {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };

        // Get branch name efficiently
        let branch_name = if let Ok(Some(head_ref)) = repo.head_ref() {
            String::from_utf8(head_ref.name().shorten().to_vec())
                .unwrap_or_else(|_| "HEAD".to_string())
        } else if let Ok(Some(head_name)) = repo.head_name() {
            String::from_utf8(head_name.shorten().to_vec()).unwrap_or_else(|_| "HEAD".to_string())
        } else {
            "main".to_string()
        };

        // Get status info based on format
        let status = match normalized_format {
            "full" => get_git_status_slow(&repo_root),
            "short" => GitStatus::empty(),
            _ => unreachable!("validate_git_format should have caught this"),
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

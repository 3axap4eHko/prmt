use crate::module_trait::{Module, ModuleContext};
use crate::modules::utils;

pub struct GitModule;

impl GitModule {
    pub fn new() -> Self {
        Self
    }
}

impl Module for GitModule {
    fn render(&self, format: &str, _context: &ModuleContext) -> Option<String> {
        let git_dir = utils::find_upward(".git")?;
        let repo_root = git_dir.parent()?;
        
        let repo = gix::open(repo_root).ok()?;
        
        let branch_name = if let Ok(Some(head_ref)) = repo.head_ref() {
            String::from_utf8(head_ref.name().shorten().to_vec())
                .unwrap_or_else(|_| "HEAD".to_string())
        } else if let Ok(Some(head_name)) = repo.head_name() {
            String::from_utf8(head_name.shorten().to_vec())
                .unwrap_or_else(|_| "HEAD".to_string())
        } else {
            "main".to_string()
        };
        
        match format {
            "" | "full" => {
                let mut result = branch_name.clone();
                
                if let Ok(output) = std::process::Command::new("git")
                    .arg("status")
                    .arg("--porcelain=v1")
                    .arg("--untracked-files=normal")
                    .current_dir(repo_root)
                    .output()
                {
                    if output.status.success() {
                        let status_text = String::from_utf8_lossy(&output.stdout);
                        
                        let mut has_changes = false;
                        let mut has_staged = false;
                        let mut has_untracked = false;
                        
                        for line in status_text.lines() {
                            if line.starts_with("??") {
                                has_untracked = true;
                            } else if !line.is_empty() {
                                let chars: Vec<char> = line.chars().take(2).collect();
                                if chars.len() >= 2 {
                                    if chars[0] != ' ' && chars[0] != '?' {
                                        has_staged = true;
                                    }
                                    if chars[1] != ' ' && chars[1] != '?' {
                                        has_changes = true;
                                    }
                                }
                            }
                        }
                        
                        if has_changes {
                            result.push('*');
                        }
                        if has_staged {
                            result.push('+');
                        }
                        if has_untracked {
                            result.push('?');
                        }
                    }
                }
                
                Some(result)
            }
            "short" => {
                Some(branch_name)
            }
            _ => None,
        }
    }
}
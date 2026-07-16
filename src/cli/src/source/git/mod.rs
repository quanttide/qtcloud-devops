//! Git 操作模块，按 git 概念拆分为子模块。
//!
//! - [`exec`] — 原始执行器（`git`、`git_check`）
//! - [`repo`] — 仓库查询（`is_git_repo`、`ref_exists`）
//! - [`status`] — 工作区状态（`is_working_tree_dirty`）
//! - [`log`] — 提交历史（`rev_list_count`、`parse_commit_messages`）
//! - [`diff`] — 变更查询（`get_changed_paths_since_last_tag`）

pub mod diff;
pub mod log;
pub mod repo;
pub mod status;

use std::path::Path;

/// 执行 git 命令，成功返回 stdout（去尾空白），失败返回错误描述。
///
/// 这是最底层的 git 执行基元，上层操作（status/log/diff）都建立在它之上。
pub fn git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("git 无法执行: {}", e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if stderr.is_empty() {
            "git 命令失败".into()
        } else {
            stderr
        })
    }
}

/// 执行 git 命令，仅检查成功与否。
pub fn git_check(args: &[&str], cwd: &Path) -> bool {
    std::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ═══════════════════════════════════════════════════════════════════════
// 兼容性 re-export：保持 `crate::source::git::xxx` 路径可用
// ═══════════════════════════════════════════════════════════════════════

pub use diff::get_changed_paths_since_last_tag;
pub use log::{parse_commit_messages, rev_list_count};
pub use repo::{is_git_repo, ref_exists};
pub use status::is_working_tree_dirty;

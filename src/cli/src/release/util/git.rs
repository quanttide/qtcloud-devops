//! git 命令封装。
//!
//! 提供统一的 `git` 命令执行入口，收敛分散在各模块的 `std::process::Command::new("git")` 调用。

use std::path::Path;
use std::process::Command;

/// 在 `cwd` 下执行 git 命令，成功时返回 stdout（去尾空白），失败返回错误描述。
pub fn git(args: &[&str], cwd: &Path) -> Result<String, String> {
    let out = Command::new("git")
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

/// 执行 git 命令，返回 `Option<bool>` 表示成功与否（不关心 stdout 内容）。
pub fn git_check(args: &[&str], cwd: &Path) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 检查路径是否为 git 仓库（存在 `.git` 目录或文件）。
///
/// `.git` 文件表示该目录是一个 git 子模块的工作树。
pub fn is_git_repo(path: &Path) -> bool {
    let git_dir = path.join(".git");
    git_dir.is_dir() || git_dir.is_file()
}

/// 统计 `tag..HEAD` 之间的提交数。
///
/// `path_filter` 可选，限制路径范围。返回 `Some(n)` 表示成功。
pub fn rev_list_count(repo_path: &Path, tag: &str, path_filter: Option<&str>) -> Option<usize> {
    let range = format!("{}..HEAD", tag);
    let mut args = vec!["rev-list", "--count", &range];
    if let Some(filter) = path_filter {
        if !filter.is_empty() && filter != "." {
            args.push("--");
            args.push(filter);
        }
    }
    let out = Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .ok()?;
    if out.status.success() {
        let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
        s.parse::<usize>().ok()
    } else {
        None
    }
}

/// 检查工作区是否有未提交的变更。
pub fn is_working_tree_dirty(repo_path: &Path) -> bool {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// 检查指定 ref 是否存在（如 tag、branch）。
pub fn ref_exists(repo_path: &Path, refname: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", refname])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_repo_dir() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_file() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".git"), "gitdir: ../.git/modules/foo").unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_false() {
        let d = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(d.path()));
    }

    #[test]
    fn test_is_working_tree_dirty_in_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        // 非 git 目录视为 dirty（无法判断）
        let dirty = is_working_tree_dirty(d.path());
        assert!(
            !dirty,
            "新目录的 git status --porcelain 应返回空（非 git 目录 git 命令失败）"
        );
    }
}

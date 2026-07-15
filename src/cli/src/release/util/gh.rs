//! gh 操作：创建/删除/查看 GitHub Release。
//!
//! 所有操作通过系统 `gh` CLI 执行。

use std::path::Path;
use std::process::Command;

/// 检查 gh CLI 是否已安装且可用。
pub fn check_gh_installed() -> bool {
    Command::new("gh")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// 创建 GitHub Release。
pub fn create_release(version: &str, notes: &str, repo: &str) -> bool {
    let out = Command::new("gh")
        .args([
            "release", "create", version, "--title", version, "--notes", notes, "--repo", repo,
        ])
        .output();
    match out {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if msg.contains("already exists") || msg.contains("已存在") {
                return true;
            }
            eprintln!("创建 Release 失败: {}", msg);
            false
        }
        Err(e) => {
            eprintln!("创建 Release 失败: {}", e);
            false
        }
    }
}

/// 查看 GitHub Release body 内容。
///
/// 返回 `None` 表示 Release 不存在或访问失败。
pub fn view_release_body(version: &str, repo: &str) -> Option<String> {
    let out = Command::new("gh")
        .args(["release", "view", version, "--repo", repo, "--json", "body", "--jq", ".body"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
        }
        _ => None,
    }
}

/// 删除 GitHub Release（等价于 `gh release delete <version> --yes`）。
pub fn delete_release(version: &str, repo: &str) -> bool {
    let out = Command::new("gh")
        .args(["release", "delete", version, "--yes", "--repo", repo])
        .output();
    match out {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if msg.contains("not found") || msg.contains("404") {
                return true;
            }
            eprintln!("删除 Release 失败: {}", msg);
            false
        }
        Err(e) => {
            eprintln!("删除 Release 失败: {}", e);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_gh_installed_does_not_panic() {
        // 只是确保函数不会 panic，不关心返回值
        let _ = check_gh_installed();
    }
}

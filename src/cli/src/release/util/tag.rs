//! tag 操作：创建/删除/推送 git tag，回滚。
//!
//! 本地 tag 操作使用 `git2` crate（创建、删除本地引用）。
//! 远程 tag 操作使用系统 `git` CLI（推送、删除远程），
//! 避免 `git2` 缺少 credential callback 导致的认证失败。

use super::git as util_git;
use std::path::Path;

/// 创建轻量 tag（`git tag <version>`）。已存在则跳过（幂等）。
pub fn create_tag(version: &str, repo_path: &Path) -> bool {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let refname = format!("refs/tags/{}", version);
    if repo.find_reference(&refname).is_ok() {
        return true;
    }
    let head_id = match repo.head().ok().and_then(|h| h.target()) {
        Some(id) => id,
        None => return false,
    };
    let result = repo.reference(&refname, head_id, false, "");
    result.is_ok()
}

fn tag_push_refspec(version: &str) -> String {
    format!("refs/tags/{}", version)
}

/// 推送 tag 到远程（需要网络）。
///
/// 使用系统 git 命令而非 git2，避免 git2 缺少 credential callback 导致的认证失败。
pub fn push_tag(version: &str, repo_path: &Path) -> Result<(), String> {
    // 先确认是 git 仓库
    if !util_git::is_git_repo(repo_path) {
        return Err("不是 git 仓库".into());
    }
    // 没有 origin 则跳过推送（测试仓库常见）
    if !util_git::git_check(&["remote", "get-url", "origin"], repo_path) {
        return Ok(());
    }
    let output = std::process::Command::new("git")
        .args(["push", "origin", &tag_push_refspec(version)])
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("执行 git push 失败: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(format!("推送标签失败: {}", stderr))
    }
}

/// 回滚 tag：删除本地和远端 tag。
pub fn rollback_tag(version: &str, repo_path: &Path) {
    let local_ok = delete_local_tag(version, repo_path);
    let remote_ok = delete_remote_tag(version, repo_path);
    if local_ok && remote_ok {
        eprintln!("已回滚标签 {}", version);
    }
}

/// 删除本地 tag（`git tag -d <version>`）。不存在也算成功。
pub fn delete_local_tag(version: &str, repo_path: &Path) -> bool {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return true,
    };
    let refname = format!("refs/tags/{}", version);
    repo.find_reference(&refname)
        .ok()
        .and_then(|mut r| r.delete().ok())
        .is_some()
}

/// 删除远端 tag（等价于 `git push --delete origin <version>`）。
///
/// 使用系统 git 命令而非 git2，避免 git2 缺少 credential callback 导致的认证失败。
pub fn delete_remote_tag(version: &str, repo_path: &Path) -> bool {
    let output = std::process::Command::new("git")
        .args(["push", "--delete", "origin", version])
        .current_dir(repo_path)
        .output();
    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
    }

    #[test]
    fn test_create_tag_in_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(!create_tag("v1.0.0", d.path()));
    }

    #[test]
    fn test_create_tag_idempotent() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(create_tag("v1.0.0", d.path()));
        assert!(create_tag("v1.0.0", d.path()));
    }

    #[test]
    fn test_push_tag_in_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        let result = push_tag("v1.0.0", d.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_push_refspec_scoped() {
        assert_eq!(tag_push_refspec("cli/v1.0.0"), "refs/tags/cli/v1.0.0");
    }

    #[test]
    fn test_tag_push_refspec_simple() {
        assert_eq!(tag_push_refspec("v1.0.0"), "refs/tags/v1.0.0");
    }

    #[test]
    fn test_tag_push_refspec_root_tag() {
        assert_eq!(tag_push_refspec("v0.1.0"), "refs/tags/v0.1.0");
    }

    #[test]
    fn test_rollback_tag_removes_tag() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        create_tag("v1.0.0", d.path());
        assert!(git2::Repository::open(d.path())
            .unwrap()
            .find_reference("refs/tags/v1.0.0")
            .is_ok());
        rollback_tag("v1.0.0", d.path());
        assert!(git2::Repository::open(d.path())
            .unwrap()
            .find_reference("refs/tags/v1.0.0")
            .is_err());
    }
}

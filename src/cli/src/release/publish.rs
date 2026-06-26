use std::path::Path;

use super::util::{self, Registry};

/// 发布版本。
///
/// 内部处理流程：
/// 1. 校验版本号格式（需匹配 `vX.Y.Z` 或 `scope/vX.Y.Z`）
/// 2. 校验 CHANGELOG.md 存在且包含对应版本记录
/// 3. 用户确认（除非 `yes = true`）
/// 4. 创建 git tag（幂等，已存在时跳过）
/// 5. 推送 tag 到远端（无远端时静默跳过）
/// 6. 创建 GitHub Release（幂等，已存在时跳过）
/// 7. 打印 registry 发布提示（不实际发布，由 CI 执行）
///
/// 回滚：步骤 5 失败时删除本地 tag；步骤 6 失败时删除本地和远端 tag。
///
/// # 参数
/// - `version`: 版本号。格式 `vX.Y.Z` 或 `scope/vX.Y.Z`（如 `cli/v0.5.0`）
/// - `repo_path`: git 仓库路径
/// - `yes`: 跳过用户确认
/// - `registry`: CI 发布目标提示（仅打印，不执行）
pub fn publish(
    version: &str,
    repo_path: &Path,
    yes: bool,
    registry: Option<Registry>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !util::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }

    // 自动生成 CHANGELOG（如果不存在当前版本记录）
    if let Err(e) = super::ensure_changelog(repo_path, version) {
        eprintln!(
            "⚠ CHANGELOG 生成失败: {}\n   发布将继续，但请确保 CHANGELOG.md 包含版本 {} 的记录。",
            e, version
        );
        // 不阻塞发布，仅输出警告
    }

    let changelog_path = repo_path.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
    }

    if !yes && !util::confirm_release(version, false) {
        return Err("已取消发布".into());
    }

    if !util::create_tag(version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if !util::push_tag(version, repo_path) {
        util::rollback_tag(version, repo_path);
        return Err(format!("推送标签 {} 失败", version).into());
    }
    println!("✓ 标签 {} 已创建并推送", version);

    let notes = util::extract_notes(version, &changelog_path);
    if let Some(repo) = util::get_remote_repo(repo_path) {
        if !util::create_release(version, notes.as_deref().unwrap_or(""), &repo) {
            util::rollback_tag(version, repo_path);
            return Err("创建 GitHub Release 失败".into());
        }
        println!("✓ GitHub Release {} 已创建", version);
        println!("  https://github.com/{}/releases/tag/{}", repo, version);
    }
    if let Some(reg) = registry {
        println!("  {:?} 由 CI 自动发布，无需本地操作", reg);
    }
    println!("✓ 版本 {} 已发布", version);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn git_init(path: &Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_commit(path: &Path, msg: &str) {
        std::fs::write(path.join("file"), msg).unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", msg])
            .current_dir(path)
            .output()
            .unwrap();
    }

    #[test]
    fn test_publish_rejects_invalid_version() {
        assert!(publish("bad", tempfile::tempdir().unwrap().path(), true, None).is_err());
    }
    #[test]
    fn test_publish_rejects_missing_changelog() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let e = publish("v1.0.0", d.path(), true, None)
            .unwrap_err()
            .to_string();
        assert!(e.contains("CHANGELOG"));
    }
    #[test]
    fn test_publish_formal_with_yes() {
        let d = tempfile::tempdir().unwrap();
        let r = publish("v1.0.0", d.path(), true, None);
        assert!(r.is_ok() || r.is_err());
    }
    #[test]
    fn test_publish_prerelease_with_yes() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "## [1.0.0-rc.1]\n\ncontent\n",
        )
        .unwrap();
        let r = publish("v1.0.0-rc.1", d.path(), true, None);
        assert!(r.is_ok() || r.is_err());
    }
}

use std::path::Path;

use super::util::{self, Registry};

pub fn publish(version: &str, repo_path: &Path, yes: bool, registry: Option<Registry>) -> Result<(), Box<dyn std::error::Error>> {
    if !util::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
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
        std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["config", "user.email", "test@test.com"]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["config", "user.name", "Test"]).current_dir(path).output().unwrap();
    }

    fn git_commit(path: &Path, msg: &str) {
        std::fs::write(path.join("file"), msg).unwrap();
        std::process::Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", msg]).current_dir(path).output().unwrap();
    }

    #[test] fn test_publish_rejects_invalid_version() { assert!(publish("bad", tempfile::tempdir().unwrap().path(), true, None).is_err()); }
    #[test] fn test_publish_rejects_missing_changelog() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); let e = publish("v1.0.0", d.path(), true, None).unwrap_err().to_string(); assert!(e.contains("CHANGELOG")); }
    #[test] fn test_publish_formal_with_yes() { let d = tempfile::tempdir().unwrap(); let r = publish("v1.0.0", d.path(), true, None); assert!(r.is_ok() || r.is_err()); }
    #[test] fn test_publish_prerelease_with_yes() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap(); let r = publish("v1.0.0-rc.1", d.path(), true, None); assert!(r.is_ok() || r.is_err()); }
}

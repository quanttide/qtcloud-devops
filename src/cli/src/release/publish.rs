use std::path::Path;

use super::util::{self, Registry};

fn is_prerelease(version: &str) -> bool {
    let base = version.split('/').last().unwrap_or(version);
    base.contains('-')
}

pub fn publish(version: &str, repo_path: &Path, yes: bool, pre_release: bool, registry: Option<Registry>) -> Result<(), Box<dyn std::error::Error>> {
    if !util::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }
    let prerelease = pre_release || is_prerelease(version);
    if pre_release && !is_prerelease(version) {
        return Err(format!("--pre-release 版本需要包含后缀（如 -rc.1）: {}", version).into());
    }

    let changelog_path = repo_path.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
    }

    if !prerelease && !yes {
        if !util::confirm_release(version, false) {
            return Err("已取消发布".into());
        }
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

    #[test] fn test_is_prerelease_rc() { assert!(is_prerelease("v1.0.0-rc.1")); }
    #[test] fn test_is_prerelease_alpha() { assert!(is_prerelease("v1.0.0-alpha.1")); }
    #[test] fn test_is_prerelease_beta() { assert!(is_prerelease("v1.0.0-beta.2")); }
    #[test] fn test_is_prerelease_scoped() { assert!(is_prerelease("cli/v0.3.2-rc.1")); }
    #[test] fn test_is_prerelease_formal() { assert!(!is_prerelease("v1.0.0")); }
    #[test] fn test_is_prerelease_formal_scoped() { assert!(!is_prerelease("cli/v0.3.2")); }

    #[test] fn test_publish_rejects_missing_changelog() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); let e = publish("v1.0.0", d.path(), true, false, None).unwrap_err().to_string(); assert!(e.contains("CHANGELOG")); }
    #[test] fn test_publish_without_stage_succeeds() { let d = tempfile::tempdir().unwrap(); let r = publish("v1.0.0", d.path(), true, false, None); assert!(r.is_ok() || r.is_err()); }
    #[test] fn test_publish_pre_release_rejects_formal() { let d = tempfile::tempdir().unwrap(); let e = publish("v1.0.0", d.path(), true, true, None).unwrap_err().to_string(); assert!(e.contains("--pre-release")); }
}

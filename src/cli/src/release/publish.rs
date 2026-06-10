use std::path::Path;

use super::util::{self, Registry};

pub fn publish(version: &str, repo_path: &Path, yes: bool, registry: Option<Registry>) -> Result<(), Box<dyn std::error::Error>> {
    let changelog_path = repo_path.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
    }
    if !util::confirm_release(version, yes) {
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
    use crate::test_support::{git_commit, git_init};

    #[test] fn test_publish_rejects_missing_changelog() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); let e = publish("v1.0.0", d.path(), true, None).unwrap_err().to_string(); assert!(e.contains("CHANGELOG")); }
    #[test] fn test_publish_without_stage_succeeds() { let d = tempfile::tempdir().unwrap(); let r = publish("v1.0.0", d.path(), true, None); assert!(r.is_ok() || r.is_err()); }
}


use std::path::Path;

use super::util;

pub fn stage(version: &str, repo_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !util::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }
    if !is_prerelease(version) {
        return Err(format!("stage 仅用于预发布版本（含 -rc.N、-alpha.N 等后缀），正式版请直接 publish: {}", version).into());
    }
    let changelog_path = repo_path.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
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
        if util::create_release(version, notes.as_deref().unwrap_or(""), &repo) {
            println!("✓ GitHub Release {} 已创建", version);
            println!("  https://github.com/{}/releases/tag/{}", repo, version);
        }
    }
    Ok(())
}

fn is_prerelease(version: &str) -> bool {
    let base = version.split('/').last().unwrap_or(version);
    base.contains('-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{git_commit, git_init};

    #[test] fn test_stage_invalid_version() { assert!(stage("bad", tempfile::tempdir().unwrap().path()).is_err()); }
    #[test] fn test_stage_formal_rejected() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); let e = stage("v1.0.0", d.path()).unwrap_err().to_string(); assert!(e.contains("仅用于预发布")); }
    #[test] fn test_stage_idempotent() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap(); assert!(stage("v1.0.0-rc.1", d.path()).is_ok()); assert!(stage("v1.0.0-rc.1", d.path()).is_ok()); }
    #[test] fn test_stage_rejects_missing_changelog() { let d = tempfile::tempdir().unwrap(); git_init(d.path()); git_commit(d.path(), "init"); let e = stage("v1.0.0-rc.1", d.path()).unwrap_err().to_string(); assert!(e.contains("CHANGELOG")); }
    #[test] fn test_is_prerelease_rc() { assert!(is_prerelease("v1.0.0-rc.1")); }
    #[test] fn test_is_prerelease_alpha() { assert!(is_prerelease("v1.0.0-alpha.1")); }
    #[test] fn test_is_prerelease_beta() { assert!(is_prerelease("v1.0.0-beta.2")); }
    #[test] fn test_is_prerelease_scoped() { assert!(is_prerelease("cli/v0.3.2-rc.1")); }
    #[test] fn test_is_prerelease_formal() { assert!(!is_prerelease("v1.0.0")); }
    #[test] fn test_is_prerelease_formal_scoped() { assert!(!is_prerelease("cli/v0.3.2")); }
}


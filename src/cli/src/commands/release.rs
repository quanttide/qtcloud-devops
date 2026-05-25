use std::path::Path;
use std::process::Command;

use crate::model::release::{FileStorage, ReleaseRecord, ReleaseStatus, Storage, TransitionError};

// ===== utility functions =====

pub fn validate_version(version: &str) -> bool {
    let re = regex::Regex::new(
        r"^(v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?|[a-zA-Z0-9_.-]+/v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?)$",
    ).unwrap();
    re.is_match(version)
}

fn normalize_version(version: &str) -> String {
    let s = version.strip_prefix('v').unwrap_or(version);
    s.split("/v").last().unwrap_or(s).to_string()
}

pub fn precheck_version_changelog(version: &str, changelog_path: &Path) -> Vec<String> {
    let mut errors = Vec::new();
    if !validate_version(version) {
        errors.push(format!("版本号格式错误: {}", version));
    }
    if changelog_path.exists() {
        let content = std::fs::read_to_string(changelog_path).unwrap_or_default();
        let ver = normalize_version(version);
        let marker = format!("## [{}]", ver);
        if !content.contains(&marker) {
            errors.push(format!("CHANGELOG.md 未找到 {} 版本记录", ver));
        }
    } else {
        errors.push(format!("CHANGELOG.md 不存在: {}", changelog_path.display()));
    }
    errors
}

pub fn extract_notes(version: &str, changelog_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(changelog_path).ok()?;
    let ver = normalize_version(version);
    let start_marker = format!("## [{}]", ver);
    let mut capture = false;
    let mut notes: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with(&start_marker) {
            capture = true;
            continue;
        }
        if capture {
            if line.starts_with("## [") {
                break;
            }
            notes.push(line);
        }
    }
    let text = notes.join("\n").trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

pub fn confirm_release(version: &str, yes: bool) -> bool {
    if yes { return true; }
    use std::io::Write;
    println!("\n发布版本: {}", version);
    print!("确认发布? (y/N): ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

fn git_args(args: &[&str], repo_path: &Path) -> Command {
    let mut cmd = Command::new("git");
    cmd.arg("-C");
    cmd.arg(repo_path);
    cmd.args(args);
    cmd
}

pub fn create_tag(version: &str, repo_path: &Path) -> bool {
    match git_args(&["tag", version], repo_path).output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => { eprintln!("创建标签失败: {}", String::from_utf8_lossy(&out.stderr).trim()); false }
        Err(e) => { eprintln!("创建标签失败: {}", e); false }
    }
}

pub fn push_tag(version: &str, repo_path: &Path) -> bool {
    match git_args(&["push", "origin", version], repo_path).output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if msg.contains("does not appear") || msg.contains("repository '' does not exist") {
                return true; // 没有 remote 时静默跳过（本地 tag 已创建）
            }
            eprintln!("推送标签失败: {}", msg);
            false
        }
        Err(e) => { eprintln!("推送标签失败: {}", e); false }
    }
}

pub fn get_remote_repo(repo_path: &Path) -> Option<String> {
    let result = git_args(&["remote", "get-url", "origin"], repo_path).output().ok()?;
    if !result.status.success() { return None; }
    let url = String::from_utf8_lossy(&result.stdout).trim().to_string();
    parse_github_repo(&url)
}

pub fn parse_github_repo(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"github\.com[/:]([^/]+/[^/]+?)(?:\.git)?$").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

pub fn create_release(version: &str, notes: &str, repo: &str) -> bool {
    match Command::new("gh")
        .args(["release", "create", version, "--title", version, "--notes", notes, "--repo", repo])
        .output()
    {
        Ok(out) if out.status.success() => true,
        Ok(out) => { eprintln!("创建 Release 失败: {}", String::from_utf8_lossy(&out.stderr).trim()); false }
        Err(e) => { eprintln!("创建 Release 失败: {}", e); false }
    }
}

pub fn rollback_tag(version: &str, repo_path: &Path) {
    git_args(&["tag", "-d", version], repo_path).output().ok();
    git_args(&["push", "origin", "--delete", version], repo_path).output().ok();
    println!("↻ 标签 {} 已回滚", version);
}

// ===== stage =====

fn is_prerelease(version: &str) -> bool {
    let base = version.split('/').last().unwrap_or(version);
    base.contains('-')
}

pub fn stage(version: &str, repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    if !validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }
    if !is_prerelease(version) {
        return Err(format!("stage 仅用于预发布版本（含 -rc.N、-alpha.N 等后缀），正式版请直接 publish: {}", version).into());
    }
    let mut storage = FileStorage::new(repo_path);
    if let Some(existing) = storage.load(version) {
        match existing.status {
            ReleaseStatus::Published => return Err(format!("版本 {} 已发布，不可重复 stage", version).into()),
            ReleaseStatus::Staged => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
                let mut updated = existing.clone();
                updated.updated_at = now;
                storage.save(&updated)?;
                // re-push tag (idempotent)
                if create_tag(version, repo_path) && push_tag(version, repo_path) {
                    println!("✓ 标签 {} 已更新并推送", version);
                }
                return Ok(updated.id);
            }
            ReleaseStatus::Cancelled => {}
            ReleaseStatus::Retired => return Err(format!("版本 {} 已退役，不可重复 stage", version).into()),
        }
    }
    let record = ReleaseRecord::new_staged(version);
    storage.save(&record)?;
    if !create_tag(version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if !push_tag(version, repo_path) {
        rollback_tag(version, repo_path);
        return Err(format!("推送标签 {} 失败", version).into());
    }
    println!("✓ 版本 {} 已进入 Staged 状态 (发布尝试 ID: {})", version, record.id);
    println!("✓ 标签 {} 已创建并推送", version);
    Ok(record.id)
}

// ===== publish =====

pub fn publish(version: &str, repo_path: &Path, yes: bool) -> Result<String, Box<dyn std::error::Error>> {
    let mut storage = FileStorage::new(repo_path);
    let mut record = storage
        .load(version)
        .ok_or_else(|| format!("版本 {} 不存在，请先执行 stage", version))?;
    if record.status != ReleaseStatus::Staged {
        return Err(format!("版本 {} 不处于 Staged 状态 (当前: {:?})", version, record.status).into());
    }
    if !confirm_release(version, yes) {
        return Err("已取消发布".into());
    }
    if !create_tag(version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if !push_tag(version, repo_path) {
        rollback_tag(version, repo_path);
        return Err(format!("推送标签 {} 失败", version).into());
    }
    println!("✓ 标签 {} 已创建并推送", version);

    let changelog_path = repo_path.join("CHANGELOG.md");
    let notes = extract_notes(version, &changelog_path);
    if let Some(repo) = get_remote_repo(repo_path) {
        if !create_release(version, notes.as_deref().unwrap_or(""), &repo) {
            rollback_tag(version, repo_path);
            return Err("创建 GitHub Release 失败".into());
        }
        println!("✓ GitHub Release {} 已创建", version);
        println!("  https://github.com/{}/releases/tag/{}", repo, version);
    }

    record.status = ReleaseStatus::Published;
    record.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
    storage.save(&record)?;
    let id = record.id.clone();
    println!("✓ 版本 {} 已发布 (发布尝试 ID: {})", version, id);
    Ok(id)
}

// ===== cancel =====

pub fn cancel(version: &str, repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    eprintln!("warning: cancel 已废弃，将在 v0.4.0 移除。rc 失败时直接递增 rc 序号即可，不需要 cancel。");
    let mut storage = FileStorage::new(repo_path);
    let mut record = storage
        .load(version)
        .ok_or_else(|| format!("版本 {} 不存在", version))?;
    if record.status != ReleaseStatus::Staged {
        return Err(Box::new(TransitionError::NotStaged(version.to_string())));
    }
    rollback_tag(version, repo_path);
    if let Some(repo) = get_remote_repo(repo_path) {
        std::process::Command::new("gh")
            .args(["release", "delete", version, "--repo", &repo, "--yes"])
            .output().ok();
        println!("✓ GitHub Release {} 已删除", version);
    }
    record.status = ReleaseStatus::Cancelled;
    record.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
    storage.save(&record)?;
    let id = record.id.clone();
    println!("✓ 版本 {} 已取消 (发布尝试 ID: {})", version, id);
    Ok(id)
}

// ===== retire =====

pub fn retire(version: &str, repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut storage = FileStorage::new(repo_path);
    let mut record = storage
        .load(version)
        .ok_or_else(|| format!("版本 {} 不存在", version))?;
    if record.status != ReleaseStatus::Published {
        return Err(Box::new(TransitionError::NotPublished(version.to_string())));
    }
    record.status = ReleaseStatus::Retired;
    record.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
    storage.save(&record)?;
    let id = record.id.clone();
    println!("✓ 版本 {} 已退役 (发布尝试 ID: {})", version, id);
    Ok(id)
}

// ===== release_status =====

pub fn release_status(repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let storage = FileStorage::new(repo_path);
    let mut records = storage.list();
    if records.is_empty() {
        println!("当前无发布记录");
        return Ok(String::new());
    }

    records.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let staged: Vec<&ReleaseRecord> = records.iter().filter(|r| r.status == ReleaseStatus::Staged).collect();
    let published: Vec<&ReleaseRecord> = records.iter().filter(|r| r.status == ReleaseStatus::Published).collect();

    println!("发布状态报告");
    println!("{}", "-".repeat(40));
    println!("待发布: {}", staged.len());
    for r in &staged {
        println!("  {} (尝试: {})", r.version, &r.id[..8]);
    }
    println!("已发布: {}", published.len());
    for r in &published {
        println!("  {} (尝试: {})", r.version, &r.id[..8]);
    }
    println!();

    println!("最新发布:");
    for r in records.iter().take(5) {
        let status_str = match r.status {
            ReleaseStatus::Staged => "Staged",
            ReleaseStatus::Published => "Published",
            ReleaseStatus::Cancelled => "Cancelled",
            ReleaseStatus::Retired => "Retired",
        };
        println!("  {:<25} {:<12} {}", r.version, status_str, r.updated_at);
    }

    Ok(records.len().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(version: &str, status: ReleaseStatus) -> ReleaseRecord {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs().to_string();
        ReleaseRecord {
            id: uuid::Uuid::new_v4().to_string(),
            version: version.to_string(),
            status,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    // validate_version

    #[test]
    fn test_validate_version_v_prefix() { assert!(validate_version("v1.2.3")); }
    #[test]
    fn test_validate_version_with_suffix() { assert!(validate_version("v1.2.3-alpha.1")); assert!(validate_version("v1.2.3-rc1")); }
    #[test]
    fn test_validate_version_pkg() { assert!(validate_version("pkg/v1.2.3")); assert!(validate_version("cli/v0.1.0")); }
    #[test]
    fn test_validate_version_invalid() { assert!(!validate_version("1.2.3")); assert!(!validate_version("v1.2")); assert!(!validate_version("abc")); }

    // parse_github_repo

    #[test]
    fn test_parse_github_repo_https() { assert_eq!(parse_github_repo("https://github.com/owner/repo.git"), Some("owner/repo".into())); }
    #[test]
    fn test_parse_github_repo_ssh() { assert_eq!(parse_github_repo("git@github.com:owner/repo.git"), Some("owner/repo".into())); }
    #[test]
    fn test_parse_github_repo_not_github() { assert_eq!(parse_github_repo("https://gitlab.com/owner/repo.git"), None); }

    // extract_notes

    #[test]
    fn test_extract_notes_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(extract_notes("v1.0.0", &dir.path().join("CHANGELOG.md")).is_some());
    }

    #[test]
    fn test_extract_notes_not_found() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(extract_notes("v2.0.0", &dir.path().join("CHANGELOG.md")).is_none());
    }

    // confirm_release

    #[test]
    fn test_confirm_release_yes_flag() { assert!(confirm_release("v1.0.0", true)); }

    // precheck_changelog

    #[test]
    fn test_precheck_changelog_no_errors() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(precheck_version_changelog("v1.0.0", &dir.path().join("CHANGELOG.md")).is_empty());
    }

    #[test]
    fn test_precheck_changelog_missing_entry() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(precheck_version_changelog("v2.0.0", &dir.path().join("CHANGELOG.md")).iter().any(|e| e.contains("未找到")));
    }

    // stage helpers

    fn git_init(path: &std::path::Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output().unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(path).output().unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(path).output().unwrap();
        std::fs::write(path.join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."]).current_dir(path).output().unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"]).current_dir(path).output().unwrap();
    }

    #[test]
    fn test_stage_new_version() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        let id = stage("v1.0.0-rc.1", dir.path()).unwrap();
        assert!(!id.is_empty());
        let s = FileStorage::new(dir.path());
        assert_eq!(s.load("v1.0.0-rc.1").unwrap().status, ReleaseStatus::Staged);
    }

    #[test]
    fn test_stage_invalid_version() { assert!(stage("bad", tempfile::tempdir().unwrap().path()).is_err()); }

    #[test]
    fn test_stage_formal_rejected() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        let err = stage("v1.0.0", dir.path()).unwrap_err().to_string();
        assert!(err.contains("仅用于预发布"));
    }

    #[test]
    fn test_stage_published_rejected() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0-rc.1", ReleaseStatus::Published)).unwrap();
        assert!(stage("v1.0.0-rc.1", dir.path()).unwrap_err().to_string().contains("已发布"));
    }

    #[test]
    fn test_stage_cancelled_restage() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        let old_id;
        {
            let mut s = FileStorage::new(dir.path());
            let r = make_record("v1.0.0-rc.1", ReleaseStatus::Cancelled);
            old_id = r.id.clone();
            s.save(&r).unwrap();
        }
        let id = stage("v1.0.0-rc.1", dir.path()).unwrap();
        assert_ne!(id, old_id);
    }

    #[test]
    fn test_stage_retired_rejected() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0-rc.1", ReleaseStatus::Retired)).unwrap();
        assert!(stage("v1.0.0-rc.1", dir.path()).unwrap_err().to_string().contains("退役"));
    }

    #[test]
    fn test_stage_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        git_init(dir.path());
        assert_eq!(stage("v1.0.0-rc.1", dir.path()).unwrap(), stage("v1.0.0-rc.1", dir.path()).unwrap());
    }

    // publish

    #[test]
    fn test_publish_not_found() {
        let dir = tempfile::tempdir().unwrap();
        assert!(publish("v1.0.0", dir.path(), true).unwrap_err().to_string().contains("请先执行 stage"));
    }

    #[test]
    fn test_publish_not_staged() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        let mut r = ReleaseRecord::new_staged("v1.0.0");
        r.status = ReleaseStatus::Cancelled;
        s.save(&r).unwrap();
        assert!(publish("v1.0.0", dir.path(), true).is_err());
    }

    // cancel

    #[test]
    fn test_cancel_nonexistent() { assert!(cancel("v9.9.9", tempfile::tempdir().unwrap().path()).is_err()); }

    #[test]
    fn test_cancel_not_staged() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Published)).unwrap();
        assert!(cancel("v1.0.0", dir.path()).is_err());
    }

    #[test]
    fn test_cancel_happy_path() {
        let dir = tempfile::tempdir().unwrap();
        let record_id;
        {
            let mut s = FileStorage::new(dir.path());
            let r = ReleaseRecord::new_staged("v1.0.0");
            record_id = r.id.clone();
            s.save(&r).unwrap();
        }
        cancel("v1.0.0", dir.path()).unwrap();
        assert_eq!(FileStorage::new(dir.path()).load("v1.0.0").unwrap().status, ReleaseStatus::Cancelled);
        assert_eq!(FileStorage::new(dir.path()).load("v1.0.0").unwrap().id, record_id);
    }

    // retire

    #[test]
    fn test_retire_nonexistent() { assert!(retire("v9.9.9", tempfile::tempdir().unwrap().path()).is_err()); }

    #[test]
    fn test_retire_not_published() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Staged)).unwrap();
        assert!(retire("v1.0.0", dir.path()).is_err());
    }

    #[test]
    fn test_retire_from_published() {
        let dir = tempfile::tempdir().unwrap();
        {
            let mut s = FileStorage::new(dir.path());
            s.save(&make_record("v1.0.0", ReleaseStatus::Published)).unwrap();
        }
        retire("v1.0.0", dir.path()).unwrap();
        assert_eq!(FileStorage::new(dir.path()).load("v1.0.0").unwrap().status, ReleaseStatus::Retired);
    }

    // release_status

    #[test]
    fn test_release_status_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(release_status(dir.path()).unwrap(), "");
    }

    #[test]
    fn test_release_status_with_records() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Staged)).unwrap();
        s.save(&make_record("v2.0.0", ReleaseStatus::Published)).unwrap();
        assert_eq!(release_status(dir.path()).unwrap(), "2");
    }

    #[test]
    fn test_release_status_multiple_staged() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = FileStorage::new(dir.path());
        s.save(&make_record("v1.0.0", ReleaseStatus::Staged)).unwrap();
        s.save(&make_record("v2.0.0", ReleaseStatus::Staged)).unwrap();
        s.save(&make_record("v3.0.0", ReleaseStatus::Published)).unwrap();
        assert_eq!(release_status(dir.path()).unwrap(), "3");
    }
}

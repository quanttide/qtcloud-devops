use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Registry {
    PyPI,
    PubDev,
    Crates,
}

pub fn validate_version(version: &str) -> bool {
    let re = regex::Regex::new(
        r"^(v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?|[a-zA-Z0-9_.-]+/v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?)$",
    ).unwrap();
    re.is_match(version)
}

pub fn normalize_version(version: &str) -> String {
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
    let text = notes
        .iter()
        .filter(|l| !l.trim().starts_with("## ["))
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn confirm_release(version: &str, yes: bool) -> bool {
    if yes {
        return true;
    }
    use std::io::Write;
    println!("\n发布版本: {}", version);
    print!("确认发布? (y/N): ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes"
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
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if msg.contains("already exists") || msg.contains("已存在") {
                return true;
            }
            eprintln!("创建标签失败: {}", msg);
            false
        }
        Err(e) => {
            eprintln!("创建标签失败: {}", e);
            false
        }
    }
}

pub fn push_tag(version: &str, repo_path: &Path) -> bool {
    match git_args(&["push", "origin", version], repo_path).output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if msg.contains("does not appear") || msg.contains("repository '' does not exist") {
                return true;
            }
            eprintln!("推送标签失败: {}", msg);
            false
        }
        Err(e) => {
            eprintln!("推送标签失败: {}", e);
            false
        }
    }
}

pub fn get_remote_repo(repo_path: &Path) -> Option<String> {
    let result = git_args(&["remote", "get-url", "origin"], repo_path)
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
    parse_github_repo(&String::from_utf8_lossy(&result.stdout).trim())
}

pub fn parse_github_repo(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"github\.com[/:]([^/]+/[^/]+?)(?:\.git)?$").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

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

pub fn rollback_tag(version: &str, repo_path: &Path) {
    git_args(&["tag", "-d", version], repo_path).output().ok();
    git_args(&["push", "origin", "--delete", version], repo_path)
        .output()
        .ok();
    println!("↻ 标签 {} 已回滚", version);
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
    fn test_validate_version_v_prefix() {
        assert!(validate_version("v1.2.3"));
    }
    #[test]
    fn test_validate_version_with_suffix() {
        assert!(validate_version("v1.2.3-alpha.1"));
        assert!(validate_version("v1.2.3-rc1"));
    }
    #[test]
    fn test_validate_version_pkg() {
        assert!(validate_version("pkg/v1.2.3"));
        assert!(validate_version("cli/v0.1.0"));
    }
    #[test]
    fn test_validate_version_invalid() {
        assert!(!validate_version("1.2.3"));
        assert!(!validate_version("v1.2"));
        assert!(!validate_version("abc"));
    }
    #[test]
    fn test_parse_github_repo_https() {
        assert_eq!(
            parse_github_repo("https://github.com/owner/repo.git"),
            Some("owner/repo".into())
        );
    }
    #[test]
    fn test_parse_github_repo_ssh() {
        assert_eq!(
            parse_github_repo("git@github.com:owner/repo.git"),
            Some("owner/repo".into())
        );
    }
    #[test]
    fn test_parse_github_repo_not_github() {
        assert_eq!(parse_github_repo("https://gitlab.com/owner/repo.git"), None);
    }
    #[test]
    fn test_extract_notes_found() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(extract_notes("v1.0.0", &d.path().join("C.md")).is_some());
    }
    #[test]
    fn test_extract_notes_not_found() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(extract_notes("v2.0.0", &d.path().join("C.md")).is_none());
    }
    #[test]
    fn test_confirm_release_yes_flag() {
        assert!(confirm_release("v1.0.0", true));
    }
    #[test]
    fn test_precheck_changelog_no_errors() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(precheck_version_changelog("v1.0.0", &d.path().join("C.md")).is_empty());
    }
    #[test]
    fn test_precheck_changelog_missing_entry() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [1.0.0]\n\ncontent").unwrap();
        assert!(precheck_version_changelog("v2.0.0", &d.path().join("C.md"))
            .iter()
            .any(|e| e.contains("未找到")));
    }
    #[test]
    fn test_precheck_changelog_file_not_found() {
        let d = tempfile::tempdir().unwrap();
        assert!(precheck_version_changelog("v1.0.0", &d.path().join("N.md"))
            .iter()
            .any(|e| e.contains("不存在")));
    }
    #[test]
    fn test_precheck_changelog_version_invalid() {
        let d = tempfile::tempdir().unwrap();
        assert!(precheck_version_changelog("bad", &d.path().join("C.md"))
            .iter()
            .any(|e| e.contains("格式错误")));
    }
    #[test]
    fn test_registry_debug() {
        assert_eq!(format!("{:?}", Registry::PyPI), "PyPI");
    }
    #[test]
    fn test_registry_clone_eq() {
        assert_eq!(Registry::PyPI, Registry::PyPI);
    }
    #[test]
    fn test_normalize_version_v_prefix() {
        assert_eq!(normalize_version("v1.2.3"), "1.2.3");
    }
    #[test]
    fn test_normalize_version_pkg() {
        assert_eq!(normalize_version("pkg/v1.2.3"), "1.2.3");
    }
    #[test]
    fn test_normalize_version_no_prefix() {
        assert_eq!(normalize_version("1.2.3"), "1.2.3");
    }
    #[test]
    fn test_normalize_version_scoped() {
        assert_eq!(normalize_version("cli/v0.3.2"), "0.3.2");
    }
    #[test]
    fn test_validate_version_formal() {
        assert!(validate_version("v1.0.0"));
    }
    #[test]
    fn test_validate_version_prerelease() {
        assert!(validate_version("v1.0.0-rc.1"));
    }
    #[test]
    fn test_validate_version_no_v() {
        assert!(!validate_version("1.0.0"));
    }
    #[test]
    fn test_validate_version_empty() {
        assert!(!validate_version(""));
    }
    #[test]
    fn test_validate_version_scope_only() {
        assert!(!validate_version("cli/"));
    }
    #[test]
    fn test_get_remote_repo_no_git_repo() {
        assert_eq!(get_remote_repo(tempfile::tempdir().unwrap().path()), None);
    }
    #[test]
    fn test_create_release_no_gh() {
        assert!(!create_release("v0.0.0-test", "", "no/repo"));
    }
    #[test]
    fn test_create_tag_in_non_git_dir() {
        assert!(!create_tag(
            "v0.0.0-test",
            tempfile::tempdir().unwrap().path()
        ));
    }
    #[test]
    fn test_create_tag_idempotent() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(create_tag("v0.0.0-test", d.path()));
        assert!(create_tag("v0.0.0-test", d.path()));
    }
    #[test]
    fn test_push_tag_in_non_git_dir() {
        assert!(!push_tag(
            "v0.0.0-test",
            tempfile::tempdir().unwrap().path()
        ));
    }
    #[test]
    fn test_push_tag_fails_with_non_existent_remote() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        assert!(create_tag("v0.0.0-test-remote", d.path()));
        std::process::Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://nonexistent.invalid/repo.git",
            ])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(!push_tag("v0.0.0-test-remote", d.path()));
    }
    #[test]
    fn test_get_remote_repo_in_git_without_remote() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert_eq!(get_remote_repo(d.path()), None);
    }
    #[test]
    fn test_rollback_tag_removes_tag() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "commit",
                "-m",
                "x",
            ])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(create_tag("v0.0.0-test-rollback", d.path()));
        rollback_tag("v0.0.0-test-rollback", d.path());
        let o = std::process::Command::new("git")
            .args(["tag", "-l"])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(!String::from_utf8_lossy(&o.stdout).contains("v0.0.0-test-rollback"));
    }
}

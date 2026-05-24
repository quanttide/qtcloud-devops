use std::path::Path;
use std::process::Command;

pub fn validate_version(version: &str) -> bool {
    let re = regex::Regex::new(
        r"^(v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?|[a-zA-Z0-9_.-]+/v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?)$",
    )
    .unwrap();
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
    if yes {
        return true;
    }
    use std::io::Write;
    println!("\n发布版本: {}", version);
    print!("确认发布? (y/N): ");
    std::io::stdout().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    let input = input.trim().to_lowercase();
    input == "y" || input == "yes"
}

pub fn create_tag(version: &str) -> bool {
    match Command::new("git").args(["tag", version]).output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            eprintln!("创建标签失败: {}", String::from_utf8_lossy(&out.stderr).trim());
            false
        }
        Err(e) => {
            eprintln!("创建标签失败: {}", e);
            false
        }
    }
}

pub fn push_tag(version: &str) -> bool {
    match Command::new("git").args(["push", "origin", version]).output() {
        Ok(out) if out.status.success() => true,
        Ok(out) => {
            eprintln!("推送标签失败: {}", String::from_utf8_lossy(&out.stderr).trim());
            false
        }
        Err(e) => {
            eprintln!("推送标签失败: {}", e);
            false
        }
    }
}

pub fn get_remote_repo() -> Option<String> {
    let result = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
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
        Ok(out) => {
            eprintln!("创建 Release 失败: {}", String::from_utf8_lossy(&out.stderr).trim());
            false
        }
        Err(e) => {
            eprintln!("创建 Release 失败: {}", e);
            false
        }
    }
}

pub fn rollback_tag(version: &str) {
    Command::new("git").args(["tag", "-d", version]).output().ok();
    Command::new("git").args(["push", "origin", "--delete", version]).output().ok();
    println!("↻ 标签 {} 已回滚", version);
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "## [1.0.0]\n\ncontent").unwrap();
        let notes = extract_notes("v1.0.0", &path);
        assert!(notes.is_some());
        assert!(notes.unwrap().contains("content"));
    }

    #[test]
    fn test_extract_notes_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "## [1.0.0]\n\ncontent").unwrap();
        assert!(extract_notes("v2.0.0", &path).is_none());
    }

    #[test]
    fn test_confirm_release_yes_flag() {
        assert!(confirm_release("v1.0.0", true));
    }

    #[test]
    fn test_precheck_changelog_no_errors() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "## [1.0.0]\n\ncontent").unwrap();
        assert!(precheck_version_changelog("v1.0.0", &path).is_empty());
    }

    #[test]
    fn test_precheck_changelog_missing_entry() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "## [1.0.0]\n\ncontent").unwrap();
        let errors = precheck_version_changelog("v2.0.0", &path);
        assert!(errors.iter().any(|e| e.contains("未找到")));
    }
}

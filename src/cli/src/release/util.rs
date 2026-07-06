use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PublishTarget {
    PyPI,
    PubDev,
    Crates,
}

pub fn validate_version(version: &str) -> bool {
    crate::contract::validate_version(version)
}

pub fn normalize_version(version: &str) -> String {
    crate::contract::normalize_version(version)
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
        let v_marker = format!("## [v{}]", ver);
        if !content.contains(&marker) && !content.contains(&v_marker) {
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
    let start_marker_v = format!("## [v{}]", ver);
    let mut capture = false;
    let mut notes: Vec<&str> = Vec::new();
    for line in content.lines() {
        if line.trim().starts_with(&start_marker) || line.trim().starts_with(&start_marker_v) {
            capture = true;
            continue;
        }
        if capture {
            if line.starts_with("## [") {
                if line.contains(&ver) || line.contains(&format!("v{}", ver)) {
                    continue;
                }
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

/// 构造 tag push 的 refspec。
fn tag_push_refspec(version: &str) -> String {
    format!("refs/tags/{}", version)
}

/// 推送 tag 到远程（需要网络）。
pub fn push_tag(version: &str, repo_path: &Path) -> Result<(), String> {
    let repo = git2::Repository::open(repo_path)
        .map_err(|e| format!("打开仓库失败: {}", e))?;
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => return Ok(()), // 无 remote 视为成功
    };
    let refspec = tag_push_refspec(version);
    remote.push(&[&refspec], None)
        .map_err(|e| format!("推送标签失败: {}", e))
}

/// 从 version 字符串提取 scope，查契约得到子目录。
pub fn resolve_scope_dir(version: &str, repo_path: &Path) -> std::path::PathBuf {
    let scope_name = if version.contains('/') { version.split('/').next().unwrap_or("") } else { "(root)" };
    if scope_name == "(root)" || scope_name.is_empty() { return repo_path.to_path_buf(); }
    let scopes = crate::contract::load_scopes(repo_path);
    if let Some(s) = scopes.iter().find(|s| s.name == scope_name) {
        let d = repo_path.join(&s.dir);
        if d.exists() { return d; }
    }
    let d = repo_path.join(scope_name);
    if d.is_dir() { d } else { repo_path.to_path_buf() }
}

/// 将 git tag 解析为 semver::Version（用于语义化版本比较）。
/// 支持 "vX.Y.Z"、"scope/vX.Y.Z"、"X.Y.Z-pre.N" 等格式。
pub fn parse_tag_semver(tag: &str) -> semver::Version {
    let ver_str = tag.split('/').last().unwrap_or(tag);
    let ver_str = ver_str.strip_prefix('v').unwrap_or(ver_str);
    semver::Version::parse(ver_str).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

/// 查询 remote origin 的 GitHub 仓库标识。
pub fn get_remote_repo(repo_path: &Path) -> Option<String> {
    let repo = gix::open(repo_path).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url(gix::remote::Direction::Fetch)?;
    parse_github_repo(url.to_string().as_str())
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
pub fn delete_remote_tag(version: &str, repo_path: &Path) -> bool {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => return true, // 无 remote 视为成功
    };
    let refspec = format!(":refs/tags/{}", version);
    remote.push(&[&refspec], None).is_ok()
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
                return true; // 不存在也算成功
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

    use super::*;

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
    fn test_extract_notes_filters_header_lines() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("C.md"),
            "## [1.0.0] - 2026-06-26\n\n\
             ## [v1.0.0] - 2023-08-31\n\n\
             ### Added\n- feature\n",
        )
        .unwrap();
        let notes = extract_notes("v1.0.0", &d.path().join("C.md")).unwrap_or_default();
        assert!(!notes.contains("## ["), "提取内容应过滤 ## [ 行: {}", notes);
        assert!(notes.contains("### Added"));
        assert!(notes.contains("- feature"));
    }

    #[test]
    fn test_extract_notes_with_v_prefix_marker() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [v1.0.0]\n\ncontent line").unwrap();
        let notes = extract_notes("v1.0.0", &d.path().join("C.md")).unwrap_or_default();
        assert!(notes.contains("content line"));
    }

    #[test]
    fn test_extract_notes_next_version_stops() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("C.md"),
            "## [1.0.0]\n\ncontent\n## [2.0.0]\n\nnext\n",
        )
        .unwrap();
        let notes = extract_notes("v1.0.0", &d.path().join("C.md")).unwrap();
        assert!(notes.contains("content"));
        assert!(!notes.contains("next"));
    }

    #[test]
    fn test_precheck_changelog_with_v_prefix_marker() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("C.md"), "## [v1.0.0]\n\ncontent\n").unwrap();
        assert!(precheck_version_changelog("v1.0.0", &d.path().join("C.md")).is_empty());
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
    fn test_publish_target_debug() {
        assert_eq!(format!("{:?}", PublishTarget::PyPI), "PyPI");
    }

    #[test]
    fn test_publish_target_clone_eq() {
        assert_eq!(PublishTarget::PyPI, PublishTarget::PyPI);
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
        assert!(push_tag(
            "v0.0.0-test",
            tempfile::tempdir().unwrap().path()
        ).is_err());
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
        assert!(push_tag("v0.0.0-test-remote", d.path()).is_err());
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
    fn test_tag_push_refspec_scoped() {
        assert_eq!(tag_push_refspec("cli/v0.9.3"), "refs/tags/cli/v0.9.3");
    }

    #[test]
    fn test_tag_push_refspec_simple() {
        assert_eq!(tag_push_refspec("v0.9.3"), "refs/tags/v0.9.3");
    }

    #[test]
    fn test_tag_push_refspec_root_tag() {
        assert_eq!(tag_push_refspec("v0.1.0"), "refs/tags/v0.1.0");
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

    #[test]
    fn test_resolve_scope_dir_with_contract() {
        let d = tempfile::tempdir().unwrap();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(contract_dir.join("contract.yaml"), "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n").unwrap();
        std::fs::create_dir_all(d.path().join("packages/cli")).unwrap();
        let resolved = resolve_scope_dir("cli/v0.1.0", d.path());
        assert!(resolved.ends_with("packages/cli"), "预期以 packages/cli 结尾，但得到: {:?}", resolved);
    }
}

mod audit;
mod detect;
mod precheck;
mod publish;
mod status;
pub(crate) mod util;

pub use crate::source::changelog::{ensure_changelog, ChangelogError};
pub use audit::{audit, audit_all, AuditItem};
pub use detect::DetectError;
pub use precheck::{run_precheck, PrecheckResult};
pub use publish::publish;
pub use status::{collect_all, status, ReleaseState, ReleaseStatus};
pub use util::gh::{check_gh_installed, create_release, delete_release};
pub use util::git::{
    git, git_check, is_git_repo, is_working_tree_dirty, ref_exists, rev_list_count,
};
pub use util::tag::{create_tag, delete_local_tag, delete_remote_tag, push_tag, rollback_tag};

// ═══════════════════════════════════════════════════════════════════════
// 业务逻辑（保留在 mod.rs）
// ═══════════════════════════════════════════════════════════════════════

use std::path::Path;

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
        let ver = normalize_version(version);
        if let Ok(cl) = quanttide_devops::source::changelog::Changelog::from_path(changelog_path) {
            if !cl.contains_version(&ver) {
                errors.push(format!("CHANGELOG.md 未找到 {} 版本记录", ver));
            }
        }
    } else {
        errors.push(format!("CHANGELOG.md 不存在: {}", changelog_path.display()));
    }
    errors
}

pub fn extract_notes(version: &str, changelog_path: &Path) -> Option<String> {
    let ver = normalize_version(version);
    let cl = quanttide_devops::source::changelog::Changelog::from_path(changelog_path).ok()?;
    cl.release_notes(&ver).map(|s| s.to_string())
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

/// 从配置文件加载 scope 名称到路径的映射。
///
/// 确保 `(root)` scope 始终存在，即使配置文件中未定义。
pub fn load_scopes_map(repo_path: &Path) -> std::collections::HashMap<String, String> {
    let mut map: std::collections::HashMap<String, String> =
        crate::contract::load_scopes(repo_path)
            .into_iter()
            .map(|s| (s.name, s.dir))
            .collect();
    if !map.contains_key("(root)") {
        map.insert("(root)".to_string(), "".to_string());
    }
    map
}

/// 获取每个 scope 的最新 semver 标签。
///
/// 遍历仓库所有标签，按 `scope/version` 格式（或根 scope 的纯版本号）分组，
/// 每组取语义版本最大的标签。返回 `(scope, 最新标签)` 列表。
pub fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)> {
    use quanttide_devops::source::git_tag::{parse_semver_tag, GixTagSource, TagSource};
    let source = GixTagSource::new(repo_path);
    let all = match source.all_tags() {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let mut result: Vec<(String, String)> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for tag in &all {
        let scope = if let Some(slash) = tag.find('/') {
            tag[..slash].to_string()
        } else {
            "(root)".to_string()
        };
        if seen.contains(&scope) {
            continue;
        }
        seen.push(scope.clone());
        let latest = all
            .iter()
            .filter(|t| {
                if scope == "(root)" {
                    !t.contains('/')
                } else {
                    t.starts_with(&format!("{}/", scope))
                }
            })
            .max_by(|a, b| parse_semver_tag(a).cmp(&parse_semver_tag(b)));
        if let Some(t) = latest {
            result.push((scope, t.clone()));
        }
    }
    result
}

/// 从 version 字符串提取 scope，查契约得到子目录。
pub fn resolve_scope_dir(version: &str, repo_path: &Path) -> std::path::PathBuf {
    let scope_name = if version.contains('/') {
        version.split('/').next().unwrap_or("")
    } else {
        "(root)"
    };
    if scope_name == "(root)" || scope_name.is_empty() {
        return repo_path.to_path_buf();
    }
    let scopes = crate::contract::load_scopes(repo_path);
    if let Some(s) = scopes.iter().find(|s| s.name == scope_name) {
        let d = repo_path.join(&s.dir);
        if d.exists() {
            return d;
        }
    }
    let d = repo_path.join(scope_name);
    if d.is_dir() {
        d
    } else {
        repo_path.to_path_buf()
    }
}

/// 查询 remote origin 的 GitHub 仓库标识。
pub fn get_remote_repo(repo_path: &Path) -> Option<String> {
    let repo = gix::open(repo_path).ok()?;
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url(gix::remote::Direction::Fetch)?;
    parse_github_repo(url.to_string().as_str())
}

pub fn parse_github_repo(url: &str) -> Option<String> {
    let after = url.split("github.com").nth(1)?;
    let path = after
        .strip_prefix('/')
        .or_else(|| after.strip_prefix(':'))?;
    let repo = path.strip_suffix(".git").unwrap_or(path);
    if repo.is_empty() || repo.contains('/') && repo.split('/').last()?.is_empty() {
        return None;
    }
    Some(repo.to_string())
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
    fn test_extract_notes() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("C.md"),
            "# Changelog\n\n## [1.0.0] - 2026-06-26\n\n### Added\n- feature\n",
        )
        .unwrap();
        let notes = extract_notes("v1.0.0", &d.path().join("C.md")).unwrap_or_default();
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
    fn test_resolve_scope_dir_with_contract() {
        let d = tempfile::tempdir().unwrap();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n",
        )
        .unwrap();
        std::fs::create_dir_all(d.path().join("packages/cli")).unwrap();
        let resolved = resolve_scope_dir("cli/v0.1.0", d.path());
        assert!(
            resolved.ends_with("packages/cli"),
            "预期以 packages/cli 结尾，但得到: {:?}",
            resolved
        );
    }
}

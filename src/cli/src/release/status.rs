//! 发布状态查询
//!
//! 提供查看仓库当前发布状态的能力：按 scope（组件/子模块）列出最新标签和
//! 自上次发布以来的未发布提交数，帮助开发者快速了解各模块的发布进度。

use std::collections::HashMap;
use std::path::Path;

use crate::contract;

/// 单个 scope 的发布状态快照。
pub struct ReleaseStatus {
    /// scope 名称（如 `qtcloud-core`，根 scope 为 `(root)`）。
    pub scope: String,
    /// 相对仓库根目录的路径。
    pub path: String,
    /// 该 scope 的最新 semver 标签。
    pub latest_tag: String,
    /// 自最新标签以来的未发布提交数。
    pub unreleased: usize,
}

impl ReleaseStatus {
    /// 收集仓库中所有 scope 的发布状态。
    pub fn collect_all(repo_path: &Path) -> Vec<ReleaseStatus> {
        let scopes_map = load_scopes_map(repo_path);
        let latest_tags = get_latest_tags_by_scope(repo_path);

        latest_tags
            .iter()
            .map(|(scope, tag)| {
                let (path, scope_dir) = resolve_scope_path(repo_path, &scopes_map, scope);
                let unreleased = count_unreleased_in_dir(repo_path, tag, &scope_dir);
                ReleaseStatus {
                    scope: scope.clone(),
                    path,
                    latest_tag: tag.clone(),
                    unreleased,
                }
            })
            .collect()
    }
}

/// 打印发布状态到标准输出。
///
/// 委托给 [`status_to`]，忽略 I/O 错误。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

/// 向指定 writer 写入发布状态报告。
///
/// 调用 [`ReleaseStatus::collect_all`] 获取数据后格式化输出。
///
/// # 输出格式
///
/// ```text
/// 发布状态
/// ────────────────────────────────────────
///   [qtcloud-core]
///     路径:         apps/qtcloud-core
///     最新标签:     v2.1.0
///     未发布提交:   3
///   [(root)]
///     路径:         .
///     最新标签:     v5.0.0
///     未发布提交:   0
/// ```
pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let statuses = ReleaseStatus::collect_all(repo_path);

    writeln!(writer, "发布状态")?;
    writeln!(writer, "{}", "─".repeat(40))?;

    if statuses.is_empty() {
        writeln!(writer, "  最新标签:     (无)")?;
        return Ok(());
    }

    for s in &statuses {
        writeln!(writer, "  [{}]", s.scope)?;
        writeln!(writer, "    路径:         {}", s.path)?;
        writeln!(writer, "    最新标签:     {}", s.latest_tag)?;
        writeln!(writer, "    未发布提交:   {}", s.unreleased)?;
    }

    Ok(())
}

/// 解析 scope 的相对路径和磁盘目录。
fn resolve_scope_path(
    repo_path: &Path,
    scopes_map: &HashMap<String, String>,
    scope: &str,
) -> (String, std::path::PathBuf) {
    if scope == "(root)" {
        return (".".into(), repo_path.to_path_buf());
    }
    match scopes_map.get(scope) {
        Some(rel) => (rel.clone(), repo_path.join(rel)),
        None => {
            let d = repo_path.join(scope);
            if d.is_dir() {
                (scope.into(), d)
            } else {
                (scope.into(), repo_path.to_path_buf())
            }
        }
    }
}

/// 从配置文件加载 scope 名称到路径的映射。
///
/// 确保 `(root)` scope 始终存在，即使配置文件中未定义。
fn load_scopes_map(repo_path: &Path) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = contract::load_scopes(repo_path)
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
fn get_latest_tags_by_scope(repo_path: &Path) -> Vec<(String, String)> {
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

/// 统计 scope 目录中自指定标签以来的未发布提交数。
///
/// 如果 `scope_dir` 是独立的 git 子仓库（子模块），委托给
/// [`count_unreleased_in_submodule`]；否则在 `repo_path` 主仓库中，
/// 用 `git rev-list --count tag..HEAD -- scope_dir` 统计。
fn count_unreleased_in_dir(repo_path: &Path, tag: &str, scope_dir: &Path) -> usize {
    if is_git_repo(scope_dir) {
        return count_unreleased_in_submodule(scope_dir, tag);
    }
    let rel = scope_dir.strip_prefix(repo_path).unwrap_or(scope_dir);
    let rel_str = rel.to_string_lossy().trim_start_matches('/').to_string();
    let range = format!("{}..HEAD", tag);
    let mut args = vec!["rev-list", "--count", &range];
    if !rel_str.is_empty() && rel_str != "." {
        args.push("--");
        args.push(rel_str.as_str());
    }
    std::process::Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

/// 判断路径是否为 git 仓库（存在 `.git` 目录或文件）。
///
/// `.git` 文件表示该目录是一个 git 子模块的工作树。
fn is_git_repo(path: &Path) -> bool {
    let git_dir = path.join(".git");
    git_dir.is_dir() || git_dir.is_file()
}

/// 统计子模块中自指定标签以来的未发布提交数。
///
/// 直接在子模块目录中执行 `git rev-list --count tag..HEAD`。
fn count_unreleased_in_submodule(submodule_path: &Path, tag: &str) -> usize {
    std::process::Command::new("git")
        .args(["rev-list", "--count", &format!("{}..HEAD", tag)])
        .current_dir(submodule_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                s.parse::<usize>().ok()
            } else {
                None
            }
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_latest_tags_semver_v10_greater_than_v9() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
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
            .args(["commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v9.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v10.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        let tags = get_latest_tags_by_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, "v10.0.0", "v10.0.0 应在 v9.0.0 之前");
    }

    // ── 测试辅助 ────────────────────────────────────────────────

    fn git_init_test(path: &Path) {
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(path)
            .output()
            .unwrap();
        std::fs::write(path.join("f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(path)
            .output()
            .unwrap();
    }

    fn git_tag_test(path: &Path, tag: &str) {
        std::process::Command::new("git")
            .args(["-C", path.to_str().unwrap(), "tag", tag])
            .output()
            .unwrap();
    }

    // ── is_git_repo ───────────────────────────────────────────

    #[test]
    fn test_is_git_repo_dir() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir(d.path().join(".git")).unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_file() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join(".git"), "gitdir: ../.git/modules/foo").unwrap();
        assert!(is_git_repo(d.path()));
    }

    #[test]
    fn test_is_git_repo_false() {
        let d = tempfile::tempdir().unwrap();
        assert!(!is_git_repo(d.path()));
    }

    #[test]
    fn test_status_to_output() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
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
            .args(["commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();

        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok(), "status_to 应成功: {:?}", result);
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("发布状态"), "应包含标题");
        assert!(out.contains("v1.0.0"), "应包含 tag 信息");
    }

    // ── status_to edge cases ─────────────────────────────────────

    #[test]
    fn test_status_to_no_tags() {
        let d = tempfile::tempdir().unwrap();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
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
            .args(["commit", "-m", "init"])
            .current_dir(d.path())
            .output()
            .unwrap();
        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok());
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("(无)"), "无 tag 应显示 (无)");
    }

    #[test]
    fn test_status_to_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok());
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("(无)"), "非 git 目录应显示 (无)");
    }
}

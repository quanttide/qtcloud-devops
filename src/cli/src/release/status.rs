//! 发布状态查询
//!
//! 提供查看仓库当前发布状态的能力：按 scope（组件/子模块）列出最新标签和
//! 自上次发布以来的未发布提交数，帮助开发者快速了解各模块的发布进度。

use std::collections::HashSet;
use std::path::Path;

/// 发布阶段的状态枚举。
///
/// 描述一个 scope 当前所处的发布生命周期阶段。
#[derive(Debug, Clone, PartialEq)]
pub enum ReleaseStatus {
    /// 从未发布过（无匹配的 git tag）。
    Unreleased,
    /// 已发布且为最新状态，无新的未发布变更。
    Latest,
    /// 有自上次发布以来的未发布提交。
    Pending,
    /// tag 与配置文件版本不一致。
    Inconsistent,
    /// 无法确定状态（如 git 命令失败）。
    Unknown,
}

/// 发布阶段状态快照。
///
/// 记录一个 scope 在某个时刻的发布状态。
#[derive(Debug)]
pub struct ReleaseState {
    /// 发布生命周期状态。
    pub status: ReleaseStatus,
    /// scope 名称。
    pub scope: String,
    /// scope 相对路径。
    pub scope_path: String,
    /// 当前最新 tag 版本号（若有）。
    pub current_version: Option<String>,
    /// 自最新 tag 以来的未发布提交数。
    pub pending_commits: usize,
    /// 变更日志路径。
    pub changelog: String,
    /// 版本一致性检查结果（空表示未检查或不适用）。
    pub version_consistent: Option<bool>,
}

impl ReleaseState {
    /// 收集仓库中所有 scope 的发布状态。
    pub fn collect_all(repo_path: &Path) -> Vec<ReleaseState> {
        let scopes_map = super::load_scopes_map(repo_path);
        let latest_tags = super::get_latest_tags_by_scope(repo_path);
        let tagged_scopes: HashSet<&str> = latest_tags.iter().map(|(s, _)| s.as_str()).collect();

        let mut states: Vec<ReleaseState> = Vec::new();

        // 1) 有 tag 的 scope：检查 changelog、计算未发布提交数 → 确定状态
        for (scope, tag) in &latest_tags {
            let scope_dir = super::resolve_scope_dir(tag, repo_path);
            let scope_path = scope_dir
                .strip_prefix(repo_path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| ".".into());
            let changelog_path = scope_dir.join("CHANGELOG.md");
            let version_consistent = check_changelog(&changelog_path, tag);

            let (status, pending_commits) =
                match count_unreleased_in_dir(repo_path, tag, &scope_dir) {
                    None => (ReleaseStatus::Unknown, 0),
                    Some(n) if version_consistent == Some(false) => {
                        (ReleaseStatus::Inconsistent, n)
                    }
                    Some(n) if n > 0 => (ReleaseStatus::Pending, n),
                    Some(n) => (ReleaseStatus::Latest, n),
                };

            states.push(ReleaseState {
                status,
                scope: scope.clone(),
                scope_path,
                current_version: Some(tag.clone()),
                pending_commits,
                changelog: "CHANGELOG.md".into(),
                version_consistent,
            });
        }

        // 2) 配置中定义但无 tag 的 scope → Unreleased
        for (scope, dir) in &scopes_map {
            if !tagged_scopes.contains(scope.as_str()) {
                let scope_path = if scope == "(root)" {
                    "".into()
                } else {
                    dir.clone()
                };
                states.push(ReleaseState {
                    status: ReleaseStatus::Unreleased,
                    scope: scope.clone(),
                    scope_path,
                    current_version: None,
                    pending_commits: 0,
                    changelog: "CHANGELOG.md".into(),
                    version_consistent: None,
                });
            }
        }

        states
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
/// 调用 [`ReleaseState::collect_all`] 获取数据后格式化输出。
///
/// # 输出格式
///
/// ```text
/// 发布状态
/// ────────────────────────────────────────
///   [qtcloud-core]          Pending
///     路径:         apps/qtcloud-core
///     最新标签:     v2.1.0
///     未发布提交:   3
///     状态:         待发布
///   [(root)]                Latest
///     路径:         .
///     最新标签:     v5.0.0
///     未发布提交:   0
///     状态:         已是最新
/// ```
pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let states = ReleaseState::collect_all(repo_path);

    writeln!(writer, "发布状态")?;
    writeln!(writer, "{}", "─".repeat(40))?;

    if states.is_empty() {
        writeln!(writer, "  最新标签:     (无)")?;
        return Ok(());
    }

    for s in &states {
        writeln!(writer, "  [{}]  {}", s.scope, s.status)?;
        writeln!(writer, "    路径:         {}", s.scope_path)?;
        match &s.current_version {
            Some(v) => writeln!(writer, "    最新标签:     {}", v)?,
            None => writeln!(writer, "    最新标签:     (无)")?,
        }
        writeln!(writer, "    未发布提交:   {}", s.pending_commits)?;
        writeln!(writer, "    变更日志:     {}", s.changelog)?;
        match s.version_consistent {
            Some(true) => writeln!(writer, "    版本一致:     是")?,
            Some(false) => writeln!(writer, "    版本一致:     否")?,
            None => {}
        }
    }

    Ok(())
}

impl std::fmt::Display for ReleaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreleased => write!(f, "未发布"),
            Self::Latest => write!(f, "已是最新"),
            Self::Pending => write!(f, "待发布"),
            Self::Inconsistent => write!(f, "版本冲突"),
            Self::Unknown => write!(f, "状态未知"),
        }
    }
}

/// 统计 scope 目录中自指定标签以来的未发布提交数。
///
/// 返回 `Some(n)` 表示成功，`None` 表示 git 命令失败。
/// 如果 `scope_dir` 是独立的 git 子仓库（子模块），委托给
/// [`count_unreleased_in_submodule`]；否则在 `repo_path` 主仓库中，
/// 用 `git rev-list --count tag..HEAD -- scope_dir` 统计。
fn count_unreleased_in_dir(repo_path: &Path, tag: &str, scope_dir: &Path) -> Option<usize> {
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
/// 返回 `Some(n)` 表示成功，`None` 表示 git 命令失败。
/// 直接在子模块目录中执行 `git rev-list --count tag..HEAD`。
fn count_unreleased_in_submodule(submodule_path: &Path, tag: &str) -> Option<usize> {
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
}

/// 检查 scope 的 CHANGELOG 文件是否包含指定版本号。
///
/// 返回：
/// - `Some(true)` — 文件存在且内容包含版本号
/// - `Some(false)` — 文件不存在或不包含版本号
/// - `None` — 文件存在但无法读取
fn check_changelog(path: &Path, version: &str) -> Option<bool> {
    if !path.exists() {
        return Some(false);
    }
    let content = std::fs::read_to_string(path).ok()?;
    Some(content.contains(version))
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
        let tags = super::super::get_latest_tags_by_scope(d.path());
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].1, "v10.0.0", "v10.0.0 应在 v9.0.0 之前");
    }

    // ── 测试辅助 ────────────────────────────────────────────────

    // ── ReleaseStatus ──────────────────────────────────────────

    #[test]
    fn test_release_status_variants() {
        assert_ne!(ReleaseStatus::Unreleased, ReleaseStatus::Latest);
        assert_ne!(ReleaseStatus::Latest, ReleaseStatus::Pending);
        assert_ne!(ReleaseStatus::Pending, ReleaseStatus::Inconsistent);
        assert_ne!(ReleaseStatus::Inconsistent, ReleaseStatus::Unknown);
    }

    #[test]
    fn test_release_status_debug() {
        assert_eq!(format!("{:?}", ReleaseStatus::Unreleased), "Unreleased");
        assert_eq!(format!("{:?}", ReleaseStatus::Latest), "Latest");
        assert_eq!(format!("{:?}", ReleaseStatus::Pending), "Pending");
        assert_eq!(format!("{:?}", ReleaseStatus::Inconsistent), "Inconsistent");
        assert_eq!(format!("{:?}", ReleaseStatus::Unknown), "Unknown");
    }

    // ── ReleaseState ───────────────────────────────────────────────

    #[test]
    fn test_release_state_unreleased() {
        let state = ReleaseState {
            status: ReleaseStatus::Unreleased,
            scope: "cli".into(),
            scope_path: "src/cli".into(),
            current_version: None,
            pending_commits: 0,
            changelog: "CHANGELOG.md".into(),
            version_consistent: None,
        };
        assert_eq!(state.status, ReleaseStatus::Unreleased);
        assert!(state.current_version.is_none());
        assert_eq!(state.pending_commits, 0);
    }

    #[test]
    fn test_release_state_pending() {
        let state = ReleaseState {
            status: ReleaseStatus::Pending,
            scope: "core".into(),
            scope_path: "packages/core".into(),
            current_version: Some("v1.2.3".into()),
            pending_commits: 5,
            changelog: "CHANGELOG.md".into(),
            version_consistent: Some(true),
        };
        assert_eq!(state.status, ReleaseStatus::Pending);
        assert_eq!(state.current_version.as_deref(), Some("v1.2.3"));
        assert_eq!(state.pending_commits, 5);
        assert_eq!(state.version_consistent, Some(true));
    }

    #[test]
    fn test_release_state_latest() {
        let state = ReleaseState {
            status: ReleaseStatus::Latest,
            scope: "(root)".into(),
            scope_path: ".".into(),
            current_version: Some("v5.0.0".into()),
            pending_commits: 0,
            changelog: "CHANGELOG.md".into(),
            version_consistent: Some(true),
        };
        assert_eq!(state.status, ReleaseStatus::Latest);
        assert_eq!(state.pending_commits, 0);
    }

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
        std::fs::write(d.path().join("CHANGELOG.md"), "## v1.0.0\n\ncontent\n").unwrap();

        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok(), "status_to 应成功: {:?}", result);
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("发布状态"), "应包含标题");
        assert!(out.contains("v1.0.0"), "应包含 tag 信息");
        assert!(out.contains("已是最新"), "CHANGELOG 一致时应为 Latest");
        assert!(out.contains("版本一致:     是"), "应显示版本一致性");
    }

    // ── check_changelog ────────────────────────────────────────────

    #[test]
    fn test_check_changelog_exists_and_contains() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## v1.0.0\n\nfeatures\n").unwrap();
        assert_eq!(
            check_changelog(&d.path().join("CHANGELOG.md"), "v1.0.0"),
            Some(true)
        );
    }

    #[test]
    fn test_check_changelog_exists_but_not_contains() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## v1.0.0\n").unwrap();
        assert_eq!(
            check_changelog(&d.path().join("CHANGELOG.md"), "v2.0.0"),
            Some(false)
        );
    }

    #[test]
    fn test_check_changelog_missing() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            check_changelog(&d.path().join("CHANGELOG.md"), "v1.0.0"),
            Some(false)
        );
    }

    #[test]
    fn test_check_changelog_unreadable() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("CHANGELOG.md");
        std::fs::write(&p, "").unwrap();
        // 在 Unix 上移除读权限使读取失败
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o200)).unwrap();
            assert_eq!(check_changelog(&p, "v1.0.0"), None);
        }
        #[cfg(not(unix))]
        {
            // Windows 上权限模型不同，跳过该分支
            assert_eq!(check_changelog(&p, "v1.0.0"), Some(false));
        }
    }

    // ── Inconsistent ───────────────────────────────────────────────

    #[test]
    fn test_collect_all_inconsistent_when_changelog_mismatch() {
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
        // CHANGELOG 写入错误的版本号 → Inconsistent
        std::fs::write(d.path().join("CHANGELOG.md"), "## v0.9.0\n").unwrap();

        let states = ReleaseState::collect_all(d.path());
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].status, ReleaseStatus::Inconsistent);
        assert_eq!(states[0].version_consistent, Some(false));
        assert_eq!(states[0].scope, "(root)");
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
        assert!(out.contains("未发布"), "无 tag 的 scope 应显示 未发布");
    }

    #[test]
    fn test_status_to_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok());
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("未发布"), "非 git 目录的 scope 应显示 未发布");
    }
}

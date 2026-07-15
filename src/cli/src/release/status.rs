//! 发布状态查询
//!
//! 提供查看仓库当前发布状态的能力：按 scope（组件/子模块）列出最新标签、
//! 未发布提交数、CHANGELOG 版本一致性和发布生命周期阶段。
//!
//! 状态枚举 [`ReleaseStatus`] 和状态快照 [`ReleaseState`] 由
//! `quanttide-devops` 工具箱 0.3.1 提供。本模块的 [`collect_all`]
//! 组合 scope 配置、git 标签、CHANGELOG 检查等逻辑，产出各 scope 的
//! [`ReleaseState`] 快照。
//!
//! 使用示例见 `quanttide-devops` 工具箱 `examples/release.rs`。

use std::collections::HashSet;
use std::path::Path;

pub use quanttide_devops::stage::release::{ReleaseState, ReleaseStatus};

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
        let version_consistent = if !changelog_path.exists() {
            Some(false)
        } else {
            match quanttide_devops::source::changelog::Changelog::from_path(&changelog_path) {
                Ok(cl) => {
                    let ver = super::normalize_version(tag);
                    Some(cl.contains_version(&ver))
                }
                Err(_) => None,
            }
        };

        let (status, pending_commits) = match count_unreleased_in_dir(repo_path, tag, &scope_dir) {
            None => (ReleaseStatus::Unknown, 0),
            Some(n) if version_consistent == Some(false) => (ReleaseStatus::Inconsistent, n),
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

/// 打印发布状态到标准输出。
///
/// 委托给 [`status_to`]，忽略 I/O 错误。
pub fn status(repo_path: &Path) {
    let mut stdout = std::io::stdout();
    status_to(&mut stdout, repo_path).ok();
}

/// 向指定 writer 写入发布状态报告。
///
/// 调用 [`collect_all`] 获取数据后格式化输出。
///
/// # 输出格式
///
/// ```text
/// 发布状态
/// ────────────────────────────────────────
///   [qtcloud-core]
///     状态:         待发布
///     路径:         apps/qtcloud-core
///     最新标签:     v2.1.0
///     未发布提交:   3
///     变更日志:     CHANGELOG.md
///     版本一致:     是
///
///   [(root)]
///     状态:         已是最新
///     路径:         .
///     最新标签:     v5.0.0
///     未发布提交:   0
///     变更日志:     CHANGELOG.md
///     版本一致:     是
/// ```
pub fn status_to(writer: &mut impl std::io::Write, repo_path: &Path) -> std::io::Result<()> {
    let states = collect_all(repo_path);

    writeln!(writer, "发布状态")?;
    writeln!(writer, "{}", "─".repeat(40))?;

    for s in &states {
        writeln!(writer, "  [{}]", s.scope)?;
        writeln!(writer, "    状态:         {}", status_label(&s.status))?;
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
        writeln!(writer)?;
    }

    Ok(())
}

/// 返回状态枚举的中文标签，用于命令行输出。
fn status_label(status: &ReleaseStatus) -> &'static str {
    match status {
        ReleaseStatus::Unreleased => "未发布",
        ReleaseStatus::Latest => "已是最新",
        ReleaseStatus::Pending => "待发布",
        ReleaseStatus::Inconsistent => "版本冲突",
        ReleaseStatus::Unknown => "状态未知",
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
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0]\n\ncontent\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        let result = status_to(&mut buf, d.path());
        assert!(result.is_ok(), "status_to 应成功: {:?}", result);
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("发布状态"), "应包含标题");
        assert!(out.contains("v1.0.0"), "应包含 tag 信息");
        assert!(out.contains("已是最新"), "CHANGELOG 一致时应为 Latest");
        assert!(out.contains("版本一致:     是"), "应显示版本一致性");
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
        std::fs::write(d.path().join("CHANGELOG.md"), "# Changelog\n\n## [0.9.0]\n").unwrap();

        let states = collect_all(d.path());
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

    // ── status_label ──────────────────────────────────────────────

    #[test]
    fn test_status_label_all_variants() {
        assert_eq!(status_label(&ReleaseStatus::Unreleased), "未发布");
        assert_eq!(status_label(&ReleaseStatus::Latest), "已是最新");
        assert_eq!(status_label(&ReleaseStatus::Pending), "待发布");
        assert_eq!(status_label(&ReleaseStatus::Inconsistent), "版本冲突");
        assert_eq!(status_label(&ReleaseStatus::Unknown), "状态未知");
    }

    // ── collect_all: Pending ───────────────────────────────────────

    #[test]
    fn test_collect_all_pending() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 新提交
        std::fs::write(d.path().join("g"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "post-release"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0]\n\ncontent\n",
        )
        .unwrap();

        let states = collect_all(d.path());
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].status, ReleaseStatus::Pending);
        assert_eq!(states[0].pending_commits, 1);
    }

    // ── collect_all: tag 无 CHANGELOG → Inconsistent ──────────────

    #[test]
    fn test_collect_all_no_changelog() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 不创建 CHANGELOG.md

        let states = collect_all(d.path());
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].status, ReleaseStatus::Inconsistent);
        assert_eq!(states[0].version_consistent, Some(false));
    }

    // ── status() 便捷函数 ─────────────────────────────────────────

    #[test]
    fn test_status_function_does_not_panic() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        // 无 tag，不 panic 即可
        status(d.path());
    }

    // ── collect_all: 多 scope + Unreleased ───────────────────────

    #[test]
    fn test_collect_all_multiple_scopes() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        // 创建第二个 scope 目录
        std::fs::create_dir_all(d.path().join("packages/lib")).unwrap();
        // 写 contract 配置两个 scope
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: .\n    language: rust\n    build_tool: cargo\n    registry: crate\n    release: {}\n  lib:\n    dir: packages/lib\n    language: rust\n    build_tool: cargo\n    registry: crate\n    release: {}\n",
        )
        .unwrap();

        // 给 cli scope 打 tag
        std::process::Command::new("git")
            .args(["tag", "cli/v0.1.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [0.1.0]\n\ncontent\n",
        )
        .unwrap();

        let states = collect_all(d.path());
        // (root) + cli + lib = 3 个 scope
        assert!(
            states.iter().any(|s| s.scope == "(root)"),
            "应包含 (root) scope"
        );
        // cli 有 tag → Latest, lib、root 无 tag → Unreleased
        assert_eq!(states.len(), 3);
        let cli = states.iter().find(|s| s.scope == "cli").unwrap();
        assert_eq!(cli.status, ReleaseStatus::Latest);
        let root = states.iter().find(|s| s.scope == "(root)").unwrap();
        assert_eq!(root.status, ReleaseStatus::Unreleased);
        let lib = states.iter().find(|s| s.scope == "lib").unwrap();
        assert_eq!(lib.status, ReleaseStatus::Unreleased);
        assert!(lib.current_version.is_none());
    }

    // ── status_to: Inconsistent 渲染 ──────────────────────────────

    #[test]
    fn test_status_to_with_inconsistent() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 不匹配的 CHANGELOG → Inconsistent
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [0.9.0]\n\nold\n",
        )
        .unwrap();

        let mut buf = Vec::new();
        status_to(&mut buf, d.path()).unwrap();
        let out = String::from_utf8_lossy(&buf);
        assert!(out.contains("版本冲突"), "Inconsistent 应显示 版本冲突");
        assert!(out.contains("版本一致:     否"), "应显示 版本一致: 否");
    }

    // ── count_unreleased_in_dir: 子目录 scope ────────────────────

    #[test]
    fn test_collect_all_subdir_scope() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        // scope 目录 = repo 子目录（非 git 仓库）
        std::fs::create_dir_all(d.path().join("apps/core")).unwrap();
        // 在该子目录中放一个文件触发 path filter
        std::fs::write(d.path().join("apps/core/main.rs"), "fn main() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "add core"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 注意：这个提交在 tag 之前，所以 pending_commits = 0
        // scope 级别的 tag
        std::process::Command::new("git")
            .args(["tag", "core/v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 子目录 scope 的 CHANGELOG
        std::fs::create_dir_all(d.path().join("apps/core")).unwrap();
        std::fs::write(
            d.path().join("apps/core/CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0]\n\ncontent\n",
        )
        .unwrap();

        // 写 contract 定义 scope
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  core:\n    dir: apps/core\n    language: rust\n    build_tool: cargo\n    registry: crate\n    release: {}\n",
        )
        .unwrap();

        let states = collect_all(d.path());
        let core = states.iter().find(|s| s.scope == "core").unwrap();
        assert_eq!(core.status, ReleaseStatus::Latest);
        assert_eq!(core.scope_path, "apps/core");
    }

    // ── CHANGELOG 解析失败 → version_consistent = None ───────────

    #[test]
    fn test_collect_all_unparseable_changelog() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 空文件 → Changelog::from_path 返回 Err
        std::fs::write(d.path().join("CHANGELOG.md"), "").unwrap();

        let states = collect_all(d.path());
        assert_eq!(states[0].version_consistent, None);
        // version_consistent 不确定时不降级为 Inconsistent，保持 Latest
        assert_eq!(states[0].status, ReleaseStatus::Latest);
    }

    // ── count_unreleased_in_dir 失败 → Unknown ───────────────────

    #[test]
    fn test_collect_all_unknown() {
        let d = tempfile::tempdir().unwrap();
        git_init_test(d.path());
        std::process::Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // 在 scope 子目录创建空 git 仓库（有提交但无 tag）
        std::fs::create_dir_all(d.path().join("broken-mod")).unwrap();
        let sub_commit = std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path().join("broken-mod"))
            .output()
            .unwrap();
        // 在子仓库中创建一个提交
        std::process::Command::new("git")
            .args(["config", "user.email", "t@t"])
            .current_dir(d.path().join("broken-mod"))
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "t"])
            .current_dir(d.path().join("broken-mod"))
            .output()
            .unwrap();
        std::fs::write(d.path().join("broken-mod/f"), "").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path().join("broken-mod"))
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(d.path().join("broken-mod"))
            .output()
            .unwrap();
        // 在主仓库打 scoped tag
        std::process::Command::new("git")
            .args(["tag", "broken/v1.0.0"])
            .current_dir(d.path())
            .output()
            .unwrap();
        // broken-mod 中有提交但没有 broken/v1.0.0 这个 tag
        // count_unreleased_in_submodule 执行 git rev-list broken/v1.0.0..HEAD 会失败 → None → Unknown

        // 写 contract 指向 broken-mod
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  broken:\n    dir: broken-mod\n    language: rust\n    build_tool: cargo\n    registry: crate\n    release: {}\n",
        )
        .unwrap();

        let states = collect_all(d.path());
        let broken = states.iter().find(|s| s.scope == "broken").unwrap();
        assert_eq!(broken.status, ReleaseStatus::Unknown);
    }
}

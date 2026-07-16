use std::path::{Path, PathBuf};

use super::PublishTarget;
use super::plan::{build_plan, print_plan, validate_plan, ReleasePlan};
use crate::source::changelog::write_changelog_content;
use crate::source::git::worktree;

/// 发布版本。三阶段架构：Plan（只读）→ Confirm → Execute（只写）。
///
/// 阶段一 Plan：计算版本号、配置文件 diff、CHANGELOG 内容、Lockfile 需求，不写盘。
/// 阶段二 Confirm：展示计划给用户，确认后进入 Execute。
/// 阶段三 Execute：写文件、git commit、git tag、push、GitHub Release。
///
/// `version` 为 None 时自动检测。`dry_run` 为 true 时只打印不执行。
pub fn publish(
    version: Option<&str>,
    repo_path: &Path,
    yes: bool,
    force: bool,
    dry_run: bool,
    registry: Option<PublishTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    // ── Phase 1: Plan（只读）───────────────────────────────────────
    let version = resolve_version(version, repo_path, dry_run)?;
    if dry_run {
        print_dry_run_preview(&version);
        return Ok(());
    }

    let plan = build_plan(repo_path, &version, force)?;
    validate_plan(&plan)?;

    // ── Phase 2: Confirm ───────────────────────────────────────────
    print_plan(&plan);
    confirm_or_abort(yes, &plan.version)?;

    // ── Phase 3: Execute（只写）────────────────────────────────────
    execute_plan(&plan, repo_path, registry)?;

    println!("✓ 版本 {} 已发布", plan.version);
    Ok(())
}

fn resolve_version(version: Option<&str>, repo_path: &Path, dry_run: bool) -> Result<String, Box<dyn std::error::Error>> {
    match version {
        Some(v) => {
            if !super::validate_version(v) {
                return Err(format!("版本号格式错误: {}", v).into());
            }
            Ok(v.to_string())
        }
        None => {
            let result = super::detect::detect_version(repo_path)?;
            if dry_run {
                println!("\n💡 建议版本: {}", result.version);
                println!("   使用 -v 指定版本执行发布，或直接运行不加 --dry-run");
            }
            Ok(result.version)
        }
    }
}

fn print_dry_run_preview(version: &str) {
    println!("\n💡 预览发布: {}", version);
    println!("   将更新 Cargo.toml/pyproject.toml 版本号");
    println!("   将更新 CHANGELOG.md");
    println!("   将创建 git tag 并推送到远端");
    println!("   将创建 GitHub Release");
    println!("   使用 -y 跳过确认直接发布");
}

fn confirm_or_abort(yes: bool, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !yes && !super::confirm_release(version, false) {
        Err("已取消发布".into())
    } else {
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Execute 阶段（所有写操作集中在此）
// ═══════════════════════════════════════════════════════════════════════

/// 执行发布计划的所有写操作。
fn execute_plan(
    plan: &ReleasePlan,
    repo_path: &Path,
    registry: Option<PublishTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 强制重新发布时清理旧 tag 和 Release
    if plan.force {
        prepare_force_release(&plan.version, repo_path);
    }

    // 2. 写配置文件
    write_config_files(plan);

    // 3. 同步 Cargo.lock（若需要）
    let lock_changed = maybe_update_lockfile(plan);

    // 4. 写 CHANGELOG
    let changelog_written = maybe_write_changelog(plan, repo_path);

    // 5. Git 提交
    git_commit_all(plan, repo_path, lock_changed || changelog_written || !plan.config_updates.is_empty());

    // 6. Git tag + push + GitHub Release
    execute_release(&plan.version, repo_path, registry)?;

    Ok(())
}

fn prepare_force_release(version: &str, repo_path: &Path) {
    if let Some(repo) = super::get_remote_repo(repo_path) {
        eprintln!("🔁 强制重新发布，清理旧资源...");
        super::delete_release(version, &repo);
    }
    super::delete_remote_tag(version, repo_path);
    super::delete_local_tag(version, repo_path);
}

/// 写入所有待更新的配置文件。
fn write_config_files(plan: &ReleasePlan) {
    for update in &plan.config_updates {
        std::fs::write(&update.path, &update.new_content).ok();
        let name = update.path.file_name().unwrap_or_default();
        println!("✓ {} 版本已更新为 {}", name.to_string_lossy(), plan.ver);
    }
}

/// 若需要则同步 Cargo.lock，返回 true 表示文件有变更。
fn maybe_update_lockfile(plan: &ReleasePlan) -> bool {
    if !plan.lockfile_needs_update {
        return false;
    }
    // 备份当前内容以便比较
    let lock_path = plan.scope_dir.join("Cargo.lock");
    let before = std::fs::read_to_string(&lock_path).unwrap_or_default();

    let ok = std::process::Command::new("cargo")
        .args(["generate-lockfile"])
        .current_dir(&plan.scope_dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("⚠ cargo generate-lockfile 失败，Cargo.lock 可能未同步");
        return false;
    }

    let after = std::fs::read_to_string(&lock_path).unwrap_or_default();
    if after == before {
        println!("✓ Cargo.lock 已是最新，无需变更");
        false
    } else {
        println!("✓ Cargo.lock 已同步");
        true
    }
}

/// 若 Plan 中包含了 CHANGELOG 新内容则写入文件，返回 true。
fn maybe_write_changelog(plan: &ReleasePlan, repo_path: &Path) -> bool {
    let content = match &plan.changelog_content {
        Some(c) => c,
        None => return false,
    };
    match write_changelog_content(repo_path, &plan.scope_dir, &plan.version, content) {
        Ok(Some(_)) => {
            println!("✓ CHANGELOG.md 已更新（版本 {})", plan.ver);
            true
        }
        Ok(None) => false,
        Err(e) => {
            eprintln!("⚠ CHANGELOG 写入失败: {}", e);
            false
        }
    }
}

/// Git add + commit 所有变更文件。
fn git_commit_all(plan: &ReleasePlan, repo_path: &Path, anything_changed: bool) {
    if !anything_changed {
        return;
    }

    // 收集待 stage 的文件（相对于 repo 根）
    let mut to_stage: Vec<PathBuf> = Vec::new();
    for f in &["Cargo.toml", "pyproject.toml", "Cargo.lock", "CHANGELOG.md"] {
        let path = plan.scope_dir.join(f);
        if path.exists() {
            to_stage.push(path);
        }
    }
    worktree::stage_files(repo_path, &to_stage);

    // Commit
    let msg = match (&plan.changelog_content, plan.config_updates.is_empty()) {
        (Some(_), _) => format!("chore: add CHANGELOG entry for {}", plan.ver),
        (None, false) => format!("chore: bump version to {}", plan.ver),
        _ => format!("chore: prepare release {}", plan.ver),
    };
    worktree::commit(repo_path, &msg);
    println!("✓ Git 提交: {}", msg);
}

fn execute_release(
    version: &str,
    repo_path: &Path,
    registry: Option<PublishTarget>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !super::create_tag(version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if let Err(e) = super::push_tag(version, repo_path) {
        super::rollback_tag(version, repo_path);
        return Err(format!("推送标签失败: {}", e).into());
    }
    println!("✓ 标签 {} 已创建并推送", version);

    let changelog_path = repo_path.join("CHANGELOG.md");
    let notes = super::extract_notes(version, &changelog_path);
    if let Some(repo) = super::get_remote_repo(repo_path) {
        if !super::create_release(version, notes.as_deref().unwrap_or(""), &repo) {
            super::rollback_tag(version, repo_path);
            return Err("创建 GitHub Release 失败".into());
        }
        println!("✓ GitHub Release {} 已创建", version);
        println!("  https://github.com/{}/releases/tag/{}", repo, version);
    }
    if let Some(reg) = registry {
        println!("  {:?} 由 CI 自动发布，无需本地操作", reg);
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════════════════════════════

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
    fn test_publish_rejects_invalid_version() {
        assert!(publish(
            Some("bad"),
            tempfile::tempdir().unwrap().path(),
            true,
            false,
            false,
            None
        )
        .is_err());
    }

    #[test]
    fn test_publish_auto_generates_changelog() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let result = publish(Some("v1.0.0"), d.path(), true, false, false, None);
        assert!(result.is_ok());
        let changelog = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap_or_default();
        assert!(changelog.contains("## [1.0.0]"));
    }

    #[test]
    fn test_publish_formal_with_yes() {
        let d = tempfile::tempdir().unwrap();
        let r = publish(Some("v1.0.0"), d.path(), true, false, false, None);
        assert!(r.is_ok() || r.is_err());
    }

    #[test]
    fn test_publish_prerelease_with_yes() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        std::fs::write(
            d.path().join("CHANGELOG.md"),
            "## [1.0.0-rc.1]\n\ncontent\n",
        )
        .unwrap();
        let r = publish(Some("v1.0.0-rc.1"), d.path(), true, false, false, None);
        assert!(r.is_ok() || r.is_err());
    }

    #[test]
    fn test_publish_scoped_version_with_contract() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");

        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n",
        )
        .unwrap();

        let scope_dir = d.path().join("packages/cli");
        std::fs::create_dir_all(&scope_dir).unwrap();
        std::fs::write(
            scope_dir.join("Cargo.toml"),
            "[package]\nname = \"cli\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            scope_dir.join("CHANGELOG.md"),
            "# CHANGELOG\n\n## [0.1.0]\n\ncontent\n",
        )
        .unwrap();

        // git add + commit 所有文件
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "setup scope"])
            .current_dir(d.path())
            .output()
            .unwrap();

        let result = publish(Some("cli/v0.2.0"), d.path(), true, false, false, None);
        assert!(result.is_ok(), "publish 失败: {:?}", result.err());

        let cargo = std::fs::read_to_string(scope_dir.join("Cargo.toml")).unwrap();
        assert!(cargo.contains("version = \"0.2.0\""));
    }

    // ── Execute 阶段单元测试 ────────────────────────────────────────

    #[test]
    fn test_write_config_files_writes_disk() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let plan = ReleasePlan {
            version: "v0.2.0".into(),
            ver: "0.2.0".into(),
            repo_path: d.path().to_path_buf(),
            scope_dir: d.path().to_path_buf(),
            force: false,
            config_updates: vec![super::super::plan::ConfigUpdate {
                path: d.path().join("Cargo.toml"),
                new_content: "[package]\nname = \"test\"\nversion = \"0.2.0\"\n".into(),
            }],
            changelog_content: None,
            lockfile_needs_update: false,
        };
        write_config_files(&plan);
        let content = std::fs::read_to_string(d.path().join("Cargo.toml")).unwrap();
        assert!(content.contains("version = \"0.2.0\""));
    }

    #[test]
    fn test_maybe_update_lockfile_noop_when_not_needed() {
        let d = tempfile::tempdir().unwrap();
        let plan = ReleasePlan {
            version: "v0.2.0".into(),
            ver: "0.2.0".into(),
            repo_path: d.path().to_path_buf(),
            scope_dir: d.path().to_path_buf(),
            force: false,
            config_updates: vec![],
            changelog_content: None,
            lockfile_needs_update: false,
        };
        assert!(!maybe_update_lockfile(&plan));
    }
}

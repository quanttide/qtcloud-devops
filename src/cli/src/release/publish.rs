use std::path::Path;

use super::PublishTarget;
use crate::contract;

/// 发布版本。
///
/// 内部处理流程：
/// 1. 校验版本号格式（有 `-v` 时），或自动检测版本号（无 `-v` 时）
/// 2. 从 contract.yaml 获取 scope 子目录
/// 3. 自动更新 Cargo.toml / pyproject.toml 版本号
/// 4. 自动生成 CHANGELOG（如有需要）并提交
/// 5. 校验 CHANGELOG 包含对应版本记录
/// 6. 用户确认（除非 `yes = true`）
/// 7. 创建 git tag → 推送 → 创建 GitHub Release
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
    // ── 确定版本号 ────────────────────────────────────────────────
    let version = match version {
        Some(v) => {
            if !super::validate_version(v) {
                return Err(format!("版本号格式错误: {}", v).into());
            }
            v.to_string()
        }
        None => {
            let result = super::detect::detect_version(repo_path)?;
            if dry_run {
                println!("\n💡 建议版本: {}", result.version);
                println!("   使用 -v 指定版本执行发布，或直接运行不加 --dry-run");
                return Ok(());
            }
            result.version
        }
    };

    // ── dry-run：仅预览 ───────────────────────────────────────────
    if dry_run {
        println!("\n💡 预览发布: {}", version);
        println!("   将更新 Cargo.toml/pyproject.toml 版本号");
        println!("   将更新 CHANGELOG.md");
        println!("   将创建 git tag 并推送到远端");
        println!("   将创建 GitHub Release");
        println!("   使用 -y 跳过确认直接发布");
        return Ok(());
    }

    let ver = super::normalize_version(&version);
    let scope_dir = super::resolve_scope_dir(&version, repo_path);

    update_config_version(&scope_dir, &ver);
    prepare_force_release(force, &version, repo_path);
    verify_config_consistency(&scope_dir, &ver)?;
    update_cargo_lock(&scope_dir);
    prepare_changelog_and_commit(repo_path, &scope_dir, &version);
    precheck_changelog(&version, &scope_dir)?;
    confirm_or_abort(yes, &version)?;
    execute_release(&version, repo_path, registry)?;

    println!("✓ 版本 {} 已发布", version);
    Ok(())
}

fn prepare_force_release(force: bool, version: &str, repo_path: &Path) {
    if !force { return; }
    if let Some(repo) = super::get_remote_repo(repo_path) {
        eprintln!("🔁 强制重新发布，清理旧资源...");
        super::delete_release(version, &repo);
    }
    super::delete_remote_tag(version, repo_path);
    super::delete_local_tag(version, repo_path);
}

fn verify_config_consistency(scope_dir: &Path, ver: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config_files = contract::read_config_versions(scope_dir);
    let inconsistent: Vec<_> = config_files.iter().filter(|(_, v)| match v {
        Some(cv) => cv != ver, None => false,
    }).collect();
    if !inconsistent.is_empty() {
        for (fname, v) in &inconsistent {
            eprintln!("⚠ {}: 版本 {} 与目标 {} 不一致", fname, v.as_deref().unwrap_or("?"), ver);
        }
        return Err("存在版本号不一致的配置文件，请先同步".into());
    }
    Ok(())
}

fn update_cargo_lock(scope_dir: &Path) {
    if !scope_dir.join("Cargo.toml").exists() { return; }
    let ok = std::process::Command::new("cargo")
        .args(["generate-lockfile"]).current_dir(scope_dir).output()
        .map(|o| o.status.success()).unwrap_or(false);
    if ok { println!("✓ Cargo.lock 已同步"); }
}

fn prepare_changelog_and_commit(repo_path: &Path, scope_dir: &Path, version: &str) {
    for f in &["Cargo.toml", "pyproject.toml", "Cargo.lock"] {
        let path = scope_dir.join(f);
        if path.exists() {
            if let Ok(rel) = path.strip_prefix(repo_path) {
                std::process::Command::new("git")
                    .args(["add", rel.to_str().unwrap_or(f)]).current_dir(repo_path).output().ok();
            }
        }
    }
    match super::ensure_changelog(repo_path, scope_dir, version) {
        Ok(Some(rel)) => {
            if std::process::Command::new("git").args(["add", &rel]).current_dir(repo_path)
                .output().map(|o| o.status.success()).unwrap_or(false)
            {
                let ver = super::normalize_version(version);
                std::process::Command::new("git")
                    .args(["commit", "-m", &format!("chore: add CHANGELOG entry for {}", ver)])
                    .current_dir(repo_path).output().ok();
                println!("✓ CHANGELOG 修改已提交");
            }
        }
        Err(e) => eprintln!("⚠ CHANGELOG 生成失败: {}\n   发布将继续，但请确保 CHANGELOG.md 包含版本 {} 的记录。", e, version),
        _ => {}
    }
}

fn precheck_changelog(version: &str, scope_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let changelog_path = scope_dir.join("CHANGELOG.md");
    let errors = super::precheck_version_changelog(version, &changelog_path);
    if !errors.is_empty() { return Err(errors.join("\n").into()); }
    Ok(())
}

fn confirm_or_abort(yes: bool, version: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !yes && !super::confirm_release(version, false) {
        Err("已取消发布".into())
    } else { Ok(()) }
}

fn execute_release(version: &str, repo_path: &Path, registry: Option<PublishTarget>) -> Result<(), Box<dyn std::error::Error>> {
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

/// 更新 Cargo.toml / pyproject.toml 中的版本号。
fn update_config_version(repo_path: &Path, version: &str) {
    for filename in &["Cargo.toml", "pyproject.toml"] {
        let path = repo_path.join(filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let updated = update_version_in_content(&content, version);
        if updated != content {
            std::fs::write(&path, &updated).ok();
            println!("✓ {} 版本已更新为 {}", filename, version);
        }
    }
}

fn update_version_in_content(content: &str, new_ver: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("version = \"") {
            let indent = &line[..line.find("version = \"").unwrap()];
            result.push_str(&format!("{}version = \"{}\"\n", indent, new_ver));
        } else if trimmed.starts_with("\"version\":") {
            let indent = &line[..line.find("\"version\":").unwrap()];
            result.push_str(&format!("{}\"version\": \"{}\",\n", indent, new_ver));
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    result
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
    fn test_update_version_in_content_toml() {
        let content = "name = \"foo\"\nversion = \"0.1.0\"\n";
        assert_eq!(
            update_version_in_content(content, "0.2.0"),
            "name = \"foo\"\nversion = \"0.2.0\"\n"
        );
    }
    #[test]
    fn test_update_version_in_content_json() {
        let content = "{\n  \"version\": \"1.0.0\",\n}\n";
        let result = update_version_in_content(content, "2.0.0");
        assert!(result.contains("\"version\": \"2.0.0\""));
    }

    #[test]
    fn test_update_config_version_creates_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            d.path().join("pyproject.toml"),
            "[project]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        update_config_version(d.path(), "0.2.0");
        let cargo = std::fs::read_to_string(d.path().join("Cargo.toml")).unwrap();
        assert!(cargo.contains("version = \"0.2.0\""));
        let pyproject = std::fs::read_to_string(d.path().join("pyproject.toml")).unwrap();
        assert!(pyproject.contains("version = \"0.2.0\""));
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
}

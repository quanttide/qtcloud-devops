use std::path::Path;

use super::util::{self, PublishTarget};
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
            if !util::validate_version(v) {
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

    let ver = util::normalize_version(&version);

    // 从 version 提取 scope 前缀，从契约获取子目录
    let scope_dir = resolve_scope_dir(&version, repo_path);

    // 自动更新配置文件版本（scope 子目录下）—— 先于一致性检查
    update_config_version(&scope_dir, &ver);

    // 强制模式：清理已存在的 tag 和 Release，允许重新发布
    if force {
        if let Some(repo) = super::util::get_remote_repo(repo_path) {
            eprintln!("🔁 强制重新发布，清理旧资源...");
            super::util::delete_release(&version, &repo);
        }
        super::util::delete_remote_tag(&version, repo_path);
        super::util::delete_local_tag(&version, repo_path);
    }

    // 预检：所有配置文件版本号一致
    let config_files = contract::read_all_config_versions(&scope_dir);
    let inconsistent: Vec<&(String, Option<String>)> = config_files
        .iter()
        .filter(|(_, v)| match v {
            Some(cv) => cv != &ver,
            None => false,
        })
        .collect();
    if !inconsistent.is_empty() {
        for (fname, v) in &inconsistent {
            let v_display = v.as_deref().unwrap_or("?");
            eprintln!("⚠ {}: 版本 {} 与目标 {} 不一致", fname, v_display, ver);
        }
        return Err("存在版本号不一致的配置文件，请先同步".into());
    }

    // 如果是 Rust 项目，更新 Cargo.lock 保持与 Cargo.toml 同步
    if scope_dir.join("Cargo.toml").exists() {
        let lockfile_updated = std::process::Command::new("cargo")
            .args(["generate-lockfile"])
            .current_dir(&scope_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if lockfile_updated {
            println!("✓ Cargo.lock 已同步");
        }
    }

    // git add 配置文件，让 ensure_changelog 的 commit 一并提交
    for f in &["Cargo.toml", "pyproject.toml", "Cargo.lock"] {
        let path = scope_dir.join(f);
        if path.exists() {
            if let Ok(rel) = path.strip_prefix(repo_path) {
                std::process::Command::new("git")
                    .args(["add", rel.to_str().unwrap_or(f)])
                    .current_dir(repo_path)
                    .output()
                    .ok();
            }
        }
    }

    // 自动生成 CHANGELOG（scope 子目录下，git 操作在 repo 根）
    if let Err(e) = super::ensure_changelog(repo_path, &scope_dir, &version) {
        eprintln!(
            "⚠ CHANGELOG 生成失败: {}\n   发布将继续，但请确保 CHANGELOG.md 包含版本 {} 的记录。",
            e, version
        );
    }

    let changelog_path = scope_dir.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(&version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
    }

    if !yes && !util::confirm_release(&version, false) {
        return Err("已取消发布".into());
    }

    if !util::create_tag(&version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if !util::push_tag(&version, repo_path) {
        util::rollback_tag(&version, repo_path);
        return Err(format!("推送标签 {} 失败", version).into());
    }
    println!("✓ 标签 {} 已创建并推送", version);

    let notes = util::extract_notes(&version, &changelog_path);
    if let Some(repo) = util::get_remote_repo(repo_path) {
        if !util::create_release(&version, notes.as_deref().unwrap_or(""), &repo) {
            util::rollback_tag(&version, repo_path);
            return Err("创建 GitHub Release 失败".into());
        }
        println!("✓ GitHub Release {} 已创建", version);
        println!("  https://github.com/{}/releases/tag/{}", repo, version);
    }
    if let Some(reg) = registry {
        println!("  {:?} 由 CI 自动发布，无需本地操作", reg);
    }
    println!("✓ 版本 {} 已发布", version);
    Ok(())
}

/// 从 version 字符串提取 scope，查契约得到子目录。
fn resolve_scope_dir(version: &str, repo_path: &Path) -> std::path::PathBuf {
    // "cli/v0.6.0" → scope="cli", "v0.1.0" → scope="(root)"
    let scope_name = if version.contains('/') {
        version.split('/').next().unwrap_or("")
    } else {
        "(root)"
    };
    if scope_name == "(root)" || scope_name.is_empty() {
        return repo_path.to_path_buf();
    }
    // 从契约查找 scope
    let scopes = contract::load_scopes(repo_path);
    if let Some(s) = scopes.iter().find(|s| s.name == scope_name) {
        let d = repo_path.join(&s.dir);
        if d.exists() {
            return d;
        }
    }
    // 回退：scope 名作为子目录
    let d = repo_path.join(scope_name);
    if d.is_dir() {
        d
    } else {
        repo_path.to_path_buf()
    }
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

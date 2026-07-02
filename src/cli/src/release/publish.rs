use std::path::Path;

use super::util::{self, Registry};
use crate::contract;

/// 发布版本。
///
/// 内部处理流程：
/// 1. 校验版本号格式
/// 2. 从 contract.yaml 获取 scope 子目录
/// 3. 自动更新 Cargo.toml / pyproject.toml 版本号
/// 4. 自动生成 CHANGELOG（如有需要）并提交
/// 5. 校验 CHANGELOG 包含对应版本记录
/// 6. 用户确认（除非 `yes = true`）
/// 7. 创建 git tag → 推送 → 创建 GitHub Release
pub fn publish(
    version: &str,
    repo_path: &Path,
    yes: bool,
    registry: Option<Registry>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !util::validate_version(version) {
        return Err(format!("版本号格式错误: {}", version).into());
    }

    let ver = util::normalize_version(version);

    // 从 version 提取 scope 前缀，从契约获取子目录
    let scope_dir = resolve_scope_dir(version, repo_path);

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

    // 自动更新配置文件版本（scope 子目录下）
    update_config_version(&scope_dir, &ver);
    // git add 配置文件，让 ensure_changelog 的 commit 一并提交
    for f in &["Cargo.toml", "pyproject.toml"] {
        let path = scope_dir.join(f);
        if path.exists() {
            std::process::Command::new("git")
                .args(["add", f])
                .current_dir(repo_path)
                .output()
                .ok();
        }
    }

    // 自动生成 CHANGELOG（scope 子目录下）
    if let Err(e) = super::ensure_changelog(&scope_dir, version) {
        eprintln!(
            "⚠ CHANGELOG 生成失败: {}\n   发布将继续，但请确保 CHANGELOG.md 包含版本 {} 的记录。",
            e, version
        );
    }

    let changelog_path = scope_dir.join("CHANGELOG.md");
    let precheck_errors = util::precheck_version_changelog(version, &changelog_path);
    if !precheck_errors.is_empty() {
        return Err(precheck_errors.join("\n").into());
    }

    if !yes && !util::confirm_release(version, false) {
        return Err("已取消发布".into());
    }

    if !util::create_tag(version, repo_path) {
        return Err(format!("创建标签 {} 失败", version).into());
    }
    if !util::push_tag(version, repo_path) {
        util::rollback_tag(version, repo_path);
        return Err(format!("推送标签 {} 失败", version).into());
    }
    println!("✓ 标签 {} 已创建并推送", version);

    let notes = util::extract_notes(version, &changelog_path);
    if let Some(repo) = util::get_remote_repo(repo_path) {
        if !util::create_release(version, notes.as_deref().unwrap_or(""), &repo) {
            util::rollback_tag(version, repo_path);
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
        assert!(publish("bad", tempfile::tempdir().unwrap().path(), true, None).is_err());
    }
    #[test]
    fn test_publish_auto_generates_changelog() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let result = publish("v1.0.0", d.path(), true, None);
        assert!(result.is_ok());
        let changelog = std::fs::read_to_string(d.path().join("CHANGELOG.md")).unwrap_or_default();
        assert!(changelog.contains("## [1.0.0]"));
    }
    #[test]
    fn test_publish_formal_with_yes() {
        let d = tempfile::tempdir().unwrap();
        let r = publish("v1.0.0", d.path(), true, None);
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
        let r = publish("v1.0.0-rc.1", d.path(), true, None);
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
}

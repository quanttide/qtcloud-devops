//! 发布计划：两阶段架构的 Plan 层。
//!
//! Plan 阶段只读不写，输出 `ReleasePlan` 供用户确认后执行。

use std::path::{Path, PathBuf};

use crate::contract;
use crate::source::changelog::generate_changelog_content;

// ── 类型定义 ──────────────────────────────────────────────────────────

/// 一条待写入的配置文件变更。
#[derive(Debug)]
pub struct ConfigUpdate {
    pub path: PathBuf,
    pub new_content: String,
}

/// 完整的发布计划。Plan 阶段只读计算的结果，Execute 阶段据此执行。
#[derive(Debug)]
pub struct ReleasePlan {
    /// 原始版本字符串（如 `"rust/v0.4.0"`）。
    pub version: String,
    /// 标准化版本号（如 `"0.4.0"`）。
    pub ver: String,
    /// 仓库根路径。
    pub repo_path: PathBuf,
    /// scope 工作目录。
    pub scope_dir: PathBuf,
    /// 是否强制重新发布。
    pub force: bool,
    /// 需要更新的配置文件列表（Cargo.toml / pyproject.toml）。
    pub config_updates: Vec<ConfigUpdate>,
    /// CHANGELOG 新条目内容。`None` 表示版本已存在无需追加。
    pub changelog_content: Option<String>,
    /// Cargo.lock 是否需要同步（Cargo.toml 版本有变更时）。
    pub lockfile_needs_update: bool,
}

// ── build_plan ────────────────────────────────────────────────────────

/// 只读地构建发布计划。
///
/// # 错误
///
/// - 版本已存在 tag 冲突（`!force` 时）
/// - 目标版本已存在于 CHANGELOG 且无新 changelog 内容可生成
pub fn build_plan(
    repo_path: &Path,
    version: &str,
    force: bool,
) -> Result<ReleasePlan, Box<dyn std::error::Error>> {
    let ver = super::normalize_version(version);
    let scope_dir = super::resolve_scope_dir(version, repo_path);

    // 计算配置文件变更（只读）
    let config_updates = compute_config_updates(&scope_dir, &ver);
    let lockfile_needs_update = !config_updates.is_empty() && scope_dir.join("Cargo.toml").exists();

    // 计算 CHANGELOG 新内容（只读）
    let changelog_content = match generate_changelog_content(repo_path, &scope_dir, version) {
        Ok(c) => c,
        Err(e) => {
            // LLM 不可用时不阻断流程，用户可手动维护 CHANGELOG
            eprintln!("⚠ 生成 CHANGELOG 失败: {} （发布继续，请手动维护）", e);
            None
        }
    };

    Ok(ReleasePlan {
        version: version.to_string(),
        ver,
        repo_path: repo_path.to_path_buf(),
        scope_dir,
        force,
        config_updates,
        changelog_content,
        lockfile_needs_update,
    })
}

/// 只读地计算配置文件版本更新内容。有变更才返回。
fn compute_config_updates(scope_dir: &Path, ver: &str) -> Vec<ConfigUpdate> {
    let mut updates = Vec::new();
    for filename in &["Cargo.toml", "pyproject.toml"] {
        let path = scope_dir.join(filename);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let updated = update_version_in_content(&content, ver);
        if updated != content {
            updates.push(ConfigUpdate {
                path,
                new_content: updated,
            });
        }
    }
    updates
}

/// 纯函数：更新文件内容中的版本号。
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

// ── validate_plan ─────────────────────────────────────────────────────

/// 验证发布计划的完整性。全部通过返回 `Ok(())`。
pub fn validate_plan(plan: &ReleasePlan) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 检查配置文件一致性（允许 Plan 中有挂起更新的文件）
    let pending_paths: std::collections::HashSet<&Path> = plan
        .config_updates
        .iter()
        .map(|u| u.path.as_path())
        .collect();
    let config_files = contract::read_config_versions(&plan.scope_dir);
    let inconsistent: Vec<_> = config_files
        .iter()
        .filter(|(fname, v)| {
            // 跳过已在 config_updates 中的文件（将由 Execute 阶段写入）
            if pending_paths.contains(plan.scope_dir.join(fname).as_path()) {
                return false;
            }
            match v {
                Some(cv) => cv != &plan.ver,
                None => false,
            }
        })
        .collect();
    if !inconsistent.is_empty() {
        for (fname, v) in &inconsistent {
            eprintln!(
                "⚠ {}: 版本 {} 与目标 {} 不一致",
                fname,
                v.as_deref().unwrap_or("?"),
                plan.ver
            );
        }
        return Err("存在版本号不一致的配置文件，请先同步".into());
    }

    // 2. 检查 CHANGELOG（Plan 已生成内容则跳过检查）
    if plan.changelog_content.is_none() {
        let changelog_path = plan.scope_dir.join("CHANGELOG.md");
        let errors = super::precheck_version_changelog(&plan.version, &changelog_path);
        if !errors.is_empty() {
            return Err(errors.join("\n").into());
        }
    }

    // 3. Tag 冲突警告（不阻断，create_tag 已处理幂等性）
    if crate::source::git::repo::ref_exists(&plan.repo_path, &plan.version) {
        if plan.force {
            eprintln!("🔁 标签 {} 已存在，使用 --force 重新发布", plan.version);
        } else {
            eprintln!("ℹ 标签 {} 已存在，将复用（如需强制覆盖，请用 --force）", plan.version);
        }
    }

    Ok(())
}

// ── print_plan ────────────────────────────────────────────────────────

/// 打印发布计划供用户确认。
pub fn print_plan(plan: &ReleasePlan) {
    println!("\n📋 发布计划");
    println!("{}", "─".repeat(40));
    println!("  版本:         {}", plan.version);
    println!("  目录:         {}", plan.scope_dir.display());

    if plan.force {
        println!("  模式:         强制重新发布（删除旧 tag + Release）");
    }

    if !plan.config_updates.is_empty() {
        println!("\n  配置文件更新:");
        for u in &plan.config_updates {
            let name = u.path.file_name().unwrap_or_default();
            println!("    • {} → 版本 {}", name.to_string_lossy(), plan.ver);
        }
    }

    match &plan.changelog_content {
        Some(c) => {
            let lines: Vec<&str> = c.lines().collect();
            let preview = if lines.len() <= 6 {
                c.clone()
            } else {
                format!("{}\n    ... ({} 行)", lines[..3].join("\n    "), lines.len())
            };
            println!("\n  CHANGELOG 更新:\n    {}", preview.replace('\n', "\n    "));
        }
        None => println!("\n  CHANGELOG:     版本已存在，无需更新"),
    }

    if plan.lockfile_needs_update {
        println!("\n  Cargo.lock:   需要同步");
    }

    println!("\n  将执行:");
    println!("    • {} 写配置文件", if plan.config_updates.is_empty() { "☐" } else { "✓" });
    println!("    • {} 生成 CHANGELOG", if plan.changelog_content.is_some() { "✓" } else { "☐" });
    println!("    • {} 同步 Cargo.lock", if plan.lockfile_needs_update { "✓" } else { "☐" });
    println!("    • ✓ Git 提交");
    println!("    • ✓ 创建并推送 tag");
    println!("    • ✓ 创建 GitHub Release");
    println!("{}", "─".repeat(40));
}

// ── 测试 ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_compute_config_updates_no_change() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let updates = compute_config_updates(d.path(), "0.1.0");
        assert!(updates.is_empty());
    }

    #[test]
    fn test_compute_config_updates_needed() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let updates = compute_config_updates(d.path(), "0.2.0");
        assert_eq!(updates.len(), 1);
        assert!(updates[0].new_content.contains("version = \"0.2.0\""));
    }

    #[test]
    fn test_validate_plan_accepts_pending_updates() {
        // 有 config_updates 覆盖时，validate 应接受（盘上版本 != 目标，但 Plan 会处理）
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let plan = ReleasePlan {
            version: "v0.2.0".into(),
            ver: "0.2.0".into(),
            repo_path: d.path().to_path_buf(),
            scope_dir: d.path().to_path_buf(),
            force: false,
            config_updates: vec![ConfigUpdate {
                path: d.path().join("Cargo.toml"),
                new_content: "[package]\nversion = \"0.2.0\"\n".into(),
            }],
            changelog_content: Some("placeholder content".into()),
            lockfile_needs_update: true,
        };
        let result = validate_plan(&plan);
        assert!(result.is_ok(), "有 config_updates 覆盖时应通过: {:?}", result.err());
    }

    #[test]
    fn test_validate_plan_rejects_inconsistent_config() {
        // 无 config_updates 覆盖时，盘上版本不一致应拒绝
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("Cargo.toml"),
            "[package]\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let plan = ReleasePlan {
            version: "v0.2.0".into(),
            ver: "0.2.0".into(),
            repo_path: d.path().to_path_buf(),
            scope_dir: d.path().to_path_buf(),
            force: false,
            config_updates: vec![],  // 无更新覆盖
            changelog_content: None,
            lockfile_needs_update: false,
        };
        let result = validate_plan(&plan);
        assert!(result.is_err());
    }
}

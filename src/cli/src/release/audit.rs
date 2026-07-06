use std::path::Path;

use super::util;
use crate::contract;

/// 发布审计结果项。
#[derive(Debug)]
pub struct AuditItem {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// 发布预检审计：不执行任何实际发布操作，仅检查是否具备发布条件。
pub fn audit(version: Option<&str>, repo_path: &Path) -> Result<Vec<AuditItem>, String> {
    let mut items: Vec<AuditItem> = Vec::new();

    // ── 1. 版本号格式 ──────────────────────────────────────────────
    let version = match version {
        Some(v) => {
            let ok = util::validate_version(v);
            items.push(AuditItem { name: "版本号格式", passed: ok, detail: if ok { v.to_string() } else { format!("无效: {}", v) } });
            if !ok { return Ok(items); }
            v.to_string()
        }
        None => match super::detect::detect_version(repo_path) {
            Ok(result) => {
                items.push(AuditItem { name: "版本号检测", passed: true, detail: result.version.clone() });
                result.version
            }
            Err(e) => {
                items.push(AuditItem { name: "版本号检测", passed: false, detail: format!("自动检测失败: {}", e) });
                return Ok(items);
            }
        },
    };

    let ver = util::normalize_version(&version);
    let scope_dir = super::util::resolve_scope_dir(&version, repo_path);

    // ── 2. 配置文件版本一致性 ─────────────────────────────────────
    let config_files = contract::read_config_versions(&scope_dir);
    let inconsistent: Vec<&(String, Option<String>)> = config_files.iter().filter(|(_, v)| match v { Some(cv) => cv != &ver, None => false }).collect();
    items.push(AuditItem {
        name: "配置文件一致性",
        passed: inconsistent.is_empty(),
        detail: if inconsistent.is_empty() { format!("{} 个文件版本均为 {}", config_files.len(), ver) } else {
            format!("不一致: {}", inconsistent.iter().map(|(f, v)| format!("{} = {}", f, v.as_deref().unwrap_or("?"))).collect::<Vec<_>>().join(", "))
        },
    });

    // ── 3. CHANGELOG ──────────────────────────────────────────────
    let changelog_path = scope_dir.join("CHANGELOG.md");
    let changelog_errors = util::precheck_version_changelog(&version, &changelog_path);
    items.push(AuditItem { name: "CHANGELOG", passed: changelog_errors.is_empty(), detail: if changelog_errors.is_empty() { format!("包含 {} 条目", ver) } else { changelog_errors.join("; ") } });

    // ── 4. 工作区干净 ─────────────────────────────────────────────
    let dirty = std::process::Command::new("git").args(["status", "--porcelain"]).current_dir(repo_path).output().map(|o| !o.stdout.is_empty()).unwrap_or(false);
    items.push(AuditItem { name: "工作区状态", passed: !dirty, detail: if dirty { "有未提交变更".into() } else { "干净".into() } });

    // ── 5. 本地 tag 不存在 ─────────────────────────────────────────
    let tag_exists = std::process::Command::new("git").args(["rev-parse", "--verify", &format!("refs/tags/{}", version)]).current_dir(repo_path).output().map(|o| o.status.success()).unwrap_or(false);
    items.push(AuditItem { name: "标签冲突", passed: !tag_exists, detail: if tag_exists { format!("{} 已存在", version) } else { format!("{} 可用", version) } });

    // ── 6. 远程可达性 ─────────────────────────────────────────────
    let remote_name = super::util::get_remote_repo(repo_path);
    items.push(AuditItem { name: "远程仓库", passed: remote_name.is_some(), detail: if let Some(ref r) = remote_name { format!("origin ({})", r) } else { "未配置 origin".into() } });

    // ── 7. GitHub Release 同步 ─────────────────────────────────────
    if tag_exists {
        if let Some(ref repo) = remote_name {
            let out = std::process::Command::new("gh").args(["release", "view", &version, "--repo", repo, "--json", "body", "--jq", ".body"]).output().ok();
            let body = out.and_then(|o| if o.status.success() { Some(String::from_utf8_lossy(&o.stdout).trim().to_string()) } else { None });
            let notes = super::util::extract_notes(&version, &changelog_path);
            let synced = body.as_deref() == notes.as_deref();
            items.push(AuditItem {
                name: "GitHub Release",
                passed: synced,
                detail: if body.is_none() { "不存在".into() } else if synced { "body 与 CHANGELOG 一致".into() } else { "body 与 CHANGELOG 不同步".into() },
            });
        } else {
            items.push(AuditItem { name: "GitHub Release", passed: true, detail: "无远程仓库，跳过".into() });
        }
    } else {
        items.push(AuditItem { name: "GitHub Release", passed: true, detail: "标签未发布，无需检查".into() });
    }

    Ok(items)
}

use std::path::Path;

use crate::contract;

/// 发布审计结果项。
#[derive(Debug)]
pub struct AuditItem {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

/// 逐 scope 发布审计。
pub fn audit_all(repo_path: &Path, scope_filter: Option<&str>) -> Result<Vec<(String, Vec<AuditItem>)>, String> {
    let c = contract::load(repo_path);
    let scopes: Vec<&contract::Scope> = match scope_filter {
        Some(name) => c.scopes.iter().filter(|s| s.name == name).collect(),
        None => c.scopes.iter().collect(),
    };
    if scopes.is_empty() {
        return Err(format!("未找到 scope{}", scope_filter.map(|s| format!(": {}", s)).unwrap_or_default()));
    }

    let mut results = Vec::new();
    for scope in &scopes {
        let version = detect_candidate_version(repo_path, &scope.name);
        let items = audit(Some(&version), repo_path).unwrap_or_else(|e| {
            vec![AuditItem { name: "版本号检测", passed: false, detail: e }]
        });
        results.push((scope.name.clone(), items));
    }
    Ok(results)
}

/// 为 scope 生成候选版本（最新 tag 的 patch+1，无 tag 则 v0.1.0）。
fn detect_candidate_version(repo_path: &Path, scope_name: &str) -> String {
    let root = std::path::Path::new(repo_path);
    let latest = super::detect::get_latest_tag_for_scope(root, Some(scope_name));
    match latest {
        Some(tag) => {
            // tag 格式 "scope/vX.Y.Z[-pre.N]"
            if let Some(ver_str) = tag.strip_prefix(&format!("{}/", scope_name)).and_then(|s| s.strip_prefix('v')) {
                if let Ok(ver) = semver::Version::parse(ver_str) {
                    let bumped = format!("v{}.{}.{}", ver.major, ver.minor, ver.patch + 1);
                    return format!("{}/{}", scope_name, bumped);
                }
            }
            // 回退：加 bump 后缀
            format!("{}/{}-bump", scope_name, tag)
        }
        None => format!("{}/v0.1.0", scope_name),
    }
}

/// 审计 GitHub Release 与 CHANGELOG 的一致性。
fn audit_github_release(items: &mut Vec<AuditItem>, tag_exists: bool, remote_name: Option<&str>, version: &str, changelog_path: &Path) {
    if !tag_exists {
        items.push(AuditItem { name: "GitHub Release", passed: true, detail: "标签未发布，无需检查".into() });
        return;
    }
    let repo = match remote_name {
        Some(r) => r,
        None => { items.push(AuditItem { name: "GitHub Release", passed: true, detail: "无远程仓库，跳过".into() }); return; }
    };
    let gh_ok = std::process::Command::new("gh").args(["--version"]).output().map(|o| o.status.success()).unwrap_or(false);
    if !gh_ok {
        items.push(AuditItem { name: "GitHub Release", passed: false, detail: "gh CLI 未安装或不可用".into() });
        return;
    }
    let out = std::process::Command::new("gh")
        .args(["release", "view", version, "--repo", repo, "--json", "body", "--jq", ".body"])
        .output();
    match out {
        Ok(o) if o.status.success() => {
            let body = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let notes = super::extract_notes(version, changelog_path);
            let synced = body == notes.as_deref().unwrap_or("");
            items.push(AuditItem {
                name: "GitHub Release",
                passed: synced,
                detail: if synced { "body 与 CHANGELOG 一致".into() } else { "body 与 CHANGELOG 不同步".into() },
            });
        }
        Ok(_) => items.push(AuditItem { name: "GitHub Release", passed: false, detail: "GitHub Release 不存在或访问失败".into() }),
        Err(e) => items.push(AuditItem { name: "GitHub Release", passed: false, detail: format!("gh CLI 执行失败: {}", e) }),
    }
}

/// 发布预检审计：不执行任何实际发布操作，仅检查是否具备发布条件。
pub fn audit(version: Option<&str>, repo_path: &Path) -> Result<Vec<AuditItem>, String> {
    let mut items: Vec<AuditItem> = Vec::new();

    // ── 1. 版本号格式 ──────────────────────────────────────────────
    let version = match version {
        Some(v) => {
            let ok = super::validate_version(v);
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

    let ver = super::normalize_version(&version);
    let scope_dir = super::resolve_scope_dir(&version, repo_path);

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
    let changelog_errors = super::precheck_version_changelog(&version, &changelog_path);
    items.push(AuditItem { name: "CHANGELOG", passed: changelog_errors.is_empty(), detail: if changelog_errors.is_empty() { format!("包含 {} 条目", ver) } else { changelog_errors.join("; ") } });

    // ── 4. 工作区干净 ─────────────────────────────────────────────
    let dirty = std::process::Command::new("git").args(["status", "--porcelain"]).current_dir(repo_path).output().map(|o| !o.stdout.is_empty()).unwrap_or(false);
    items.push(AuditItem { name: "工作区状态", passed: !dirty, detail: if dirty { "有未提交变更".into() } else { "干净".into() } });

    // ── 5. 本地 tag 不存在 ─────────────────────────────────────────
    let tag_exists = std::process::Command::new("git").args(["rev-parse", "--verify", &format!("refs/tags/{}", version)]).current_dir(repo_path).output().map(|o| o.status.success()).unwrap_or(false);
    items.push(AuditItem { name: "标签冲突", passed: !tag_exists, detail: if tag_exists { format!("{} 已存在", version) } else { format!("{} 可用", version) } });

    // ── 6. 远程可达性 ─────────────────────────────────────────────
    let remote_name = super::get_remote_repo(repo_path);
    items.push(AuditItem { name: "远程仓库", passed: remote_name.is_some(), detail: if let Some(ref r) = remote_name { format!("origin ({})", r) } else { "未配置 origin".into() } });

    audit_github_release(&mut items, tag_exists, remote_name.as_deref(), &version, &changelog_path);

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // ── detect_candidate_version ─────────────────────────────────

    #[test]
    fn test_detect_candidate_no_tags() {
        let d = tmpdir();
        let version = detect_candidate_version(d.path(), "cli");
        assert_eq!(version, "cli/v0.1.0");
    }

    #[test]
    fn test_detect_candidate_with_existing_tag() {
        let d = tmpdir();
        std::process::Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["config", "user.name", "test"])
            .current_dir(d.path())
            .output()
            .unwrap();
        std::fs::write(d.path().join("file"), "").unwrap();
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
            .args(["tag", "cli/v1.2.3"])
            .current_dir(d.path())
            .output()
            .unwrap();

        let version = detect_candidate_version(d.path(), "cli");
        assert_eq!(version, "cli/v1.2.4");
    }

    // ── audit ────────────────────────────────────────────────────

    #[test]
    fn test_audit_invalid_version() {
        let d = tmpdir();
        let items = audit(Some("bad-version"), d.path()).unwrap();
        assert!(!items.is_empty());
        // 版本号格式检查应失败
        assert!(items.iter().any(|i| !i.passed && i.name == "版本号格式"));
    }

    #[test]
    fn test_audit_valid_version() {
        let d = tmpdir();
        let items = audit(Some("cli/v1.0.0"), d.path()).unwrap();
        assert!(items.len() >= 2, "应有多项检查: {:?}", items);
        assert!(items.iter().any(|i| i.passed && i.name == "版本号格式"));
    }

    #[test]
    fn test_audit_no_remote() {
        let d = tmpdir();
        let items = audit(Some("cli/v1.0.0"), d.path()).unwrap();
        let remote = items.iter().find(|i| i.name == "远程仓库");
        assert!(remote.is_some());
        assert!(!remote.unwrap().passed, "无远程应标记为失败");
    }

    #[test]
    fn test_audit_all_no_scopes() {
        let d = tmpdir();
        let result = audit_all(d.path(), None);
        assert!(result.is_err(), "无 scope 应失败");
    }

    #[test]
    fn test_audit_all_with_scope_filter_empty() {
        let d = tmpdir();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "stages:\n  test:\n    threshold: 80\nscopes:\n  cli:\n    dir: .\n",
        )
        .unwrap();
        let result = audit_all(d.path(), Some("nonexistent"));
        assert!(result.is_err(), "不存在的 scope 应失败");
    }

    #[test]
    fn test_audit_all_found_scope() {
        let d = tmpdir();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "stages:\n  test:\n    threshold: 80\nscopes:\n  cli:\n    dir: .\n",
        )
        .unwrap();
        let result = audit_all(d.path(), Some("cli"));
        assert!(result.is_ok(), "存在的 scope 应通过: {:?}", result.err());
        let items = result.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, "cli");
    }

    // ── audit_github_release ─────────────────────────────────────

    #[test]
    fn test_audit_github_release_no_tag() {
        let mut items = Vec::new();
        audit_github_release(&mut items, false, Some("owner/repo"), "v1.0.0", Path::new("CHANGELOG.md"));
        assert!(items.iter().any(|i| i.passed && i.name == "GitHub Release"));
    }

    #[test]
    fn test_audit_github_release_no_remote() {
        let mut items = Vec::new();
        audit_github_release(&mut items, true, None, "v1.0.0", Path::new("CHANGELOG.md"));
        assert!(items.iter().any(|i| i.passed && i.name == "GitHub Release"));
    }
}

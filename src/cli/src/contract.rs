/// 契约模块 — 基于 `quanttide-devops` toolkit 的适配层。
///
/// 类型定义与 YAML 解析委托给 toolkit，本模块仅保留 CLI 特有的版本检测逻辑。
pub use quanttide_devops::contract::{
    detect_language_by_files, normalize_version, read_all_config_versions, validate_version,
    BuildTool, Contract, Language, Pipeline, Platform, Registry, Scope, SourceControl, SourceType,
    StageBuild, StageRelease, StageTest, VersionSource,
};

use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// 加载（保留向后兼容的行为）
// ═══════════════════════════════════════════════════════════════════════

/// 从 `.quanttide/devops/contract.yaml` 加载完整契约。
///
/// 文件不存在或解析失败时降级为默认契约（兼容旧调用方）。
pub fn load(repo_path: &Path) -> Contract {
    match quanttide_devops::contract::load(repo_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  ℹ contract.yaml: {}，使用默认契约", e);
            Contract::default()
        }
    }
}

/// 快速加载 scope 列表。
pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

/// 根据目录下的标志文件推测编程语言（`detect_language_by_files` 的别名）。
pub fn detect_by_files(dir: &Path) -> Language {
    detect_language_by_files(dir)
}

// ═══════════════════════════════════════════════════════════════════════
// 版本状态（CLI 特有，toolkit 尚不支持）
// ═══════════════════════════════════════════════════════════════════════

/// 版本一致性检查结果。
#[derive(Debug)]
pub struct VersionStatus {
    pub tag_version: Option<String>,
    pub config_version: Option<String>,
    pub consistent: bool,
    /// 所有配置文件的版本号明细。(文件名, 版本号)
    pub config_files: Vec<(String, Option<String>)>,
}

/// 检查 scope 下所有已知配置文件的版本，判断与 tag 是否一致。
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus {
    let tag_version = latest_tag_for_scope(repo_path, &scope.name);
    let scope_dir = repo_path.join(&scope.dir);
    let config_files = quanttide_devops::contract::read_all_config_versions(&scope_dir);
    let config_version = config_files
        .iter()
        .find(|(_, v)| v.is_some())
        .and_then(|(_, v)| v.clone());
    let consistent = match &tag_version {
        Some(t) => config_files.iter().all(|(_, v)| match v {
            Some(cv) => cv == t,
            None => true,
        }),
        None => config_version.is_none(),
    };
    VersionStatus {
        tag_version,
        config_version,
        consistent,
        config_files,
    }
}

fn latest_tag_for_scope(repo_path: &Path, scope_name: &str) -> Option<String> {
    let repo = git2::Repository::open(repo_path).ok()?;
    let tag_names = repo.tag_names(None).ok()?;
    let mut tags: Vec<&str> = tag_names.iter().flatten().collect();
    // ponytail: string reverse-sort 近似 git tag --sort=-version:refname。
    // v10.0.0 vs v9.0.0 会排错，遇到时升级为 semver 比较。
    tags.sort_by(|a, b| b.cmp(a));

    let prefix = format!("{}/", scope_name);
    let filtered: Vec<&&str> = tags
        .iter()
        .filter(|t| t.starts_with(&prefix) || !t.contains('/'))
        .collect();
    let scoped = filtered.iter().find(|t| t.starts_with(&prefix));
    match scoped {
        Some(t) => Some(normalize_version(t)),
        None => filtered.first().map(|t| normalize_version(t)),
    }
}

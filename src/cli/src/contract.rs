/// 契约模块 — 基于 `quanttide-devops` toolkit 的适配层。
pub use quanttide_devops::contract::{
    detect_language_by_files, normalize_version, read_all_config_versions, validate_version,
    BuildTool, Contract, Language, Pipeline, Platform, Registry, Scope, SourceControl, SourceType,
    StageBuild, StageRelease, StageTest, VersionSource,
};
pub use quanttide_devops::source::git::{GitSourceError, VersionStatus};

use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
// 加载（保留向后兼容的行为）
// ═══════════════════════════════════════════════════════════════════════

pub fn load(repo_path: &Path) -> Contract {
    match quanttide_devops::contract::load(repo_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  ℹ contract.yaml: {}，使用默认契约", e);
            Contract::default()
        }
    }
}

pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

pub fn detect_by_files(dir: &Path) -> Language {
    detect_language_by_files(dir)
}

// ═══════════════════════════════════════════════════════════════════════
// 版本状态（适配 toolkit 的 Result → 旧签名）
// ═══════════════════════════════════════════════════════════════════════

/// 检查 scope 版本一致性。失败时返回空的 VersionStatus。
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionStatus {
    quanttide_devops::source::git::version_status(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        VersionStatus {
            tag_version: None,
            config_version: None,
            consistent: false,
            config_files: vec![],
        }
    })
}

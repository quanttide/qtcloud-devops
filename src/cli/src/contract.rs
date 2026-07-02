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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_status_git_error_returns_fallback() {
        // 不存在的路径 → git2 打开失败 → version_status 返回 Err → fallback
        let scope = Scope {
            name: "test".into(),
            dir: ".".into(),
            language: Language::Rust,
            framework: String::new(),
            build_tool: BuildTool::Cargo,
            registry: Registry::None,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let vs = version_status(Path::new("/nonexistent"), &scope);
        assert!(!vs.consistent);
        assert_eq!(vs.tag_version, None);
        assert_eq!(vs.config_version, None);
        assert!(vs.config_files.is_empty());
    }
}

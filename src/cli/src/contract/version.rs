pub use quanttide_devops::contract::{
    check_version_consistency, normalize_version, validate_version, verify_version, VersionState,
};

use std::path::Path;

use crate::contract::Scope;

/// 检查 scope 版本一致性。失败时返回空状态。
pub fn version_status(repo_path: &Path, scope: &Scope) -> VersionState {
    verify_version(repo_path, scope).unwrap_or_else(|e| {
        eprintln!("  ⚠ 版本状态检查失败: {}", e);
        VersionState {
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

    fn tmpdir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_version_status_no_repo() {
        let d = tmpdir();
        let scope = Scope {
            name: "test".into(),
            dir: ".".into(),
            language: crate::contract::Language::Unknown(String::new()),
            framework: String::new(),
            build_tool: crate::contract::BuildTool::Unknown(String::new()),
            registry: crate::contract::Registry::None,
            release: crate::contract::StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let state = version_status(d.path(), &scope);
        assert!(!state.consistent);
    }
}

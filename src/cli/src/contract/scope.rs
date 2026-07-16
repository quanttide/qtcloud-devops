use std::path::Path;

use crate::contract::{
    load, verify_version, BuildTool, Language, Registry, Scope, StageRelease, VersionState,
};

/// 加载所有 scope 列表。
pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    load(repo_path).scopes
}

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
            language: Language::Unknown(String::new()),
            framework: String::new(),
            build_tool: BuildTool::Unknown(String::new()),
            registry: Registry::None,
            release: StageRelease::default(),
            test_threshold: None,
            ci_workflow: None,
        };
        let state = version_status(d.path(), &scope);
        assert!(!state.consistent);
    }
}

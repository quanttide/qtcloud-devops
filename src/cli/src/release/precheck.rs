use std::path::{Path, PathBuf};

/// 预检结果。
pub struct PrecheckResult {
    pub version: String,
    pub normalized: String,
    pub scope_dir: PathBuf,
    pub changelog_path: PathBuf,
    pub changelog_errors: Vec<String>,
}

impl PrecheckResult {
    pub fn changelog_ok(&self) -> bool {
        self.changelog_errors.is_empty()
    }
}

/// 运行预检链：normalize_version → resolve_scope_dir → precheck_version_changelog。
pub fn run_precheck(version: &str, repo_path: &Path) -> PrecheckResult {
    let normalized = super::normalize_version(version);
    let scope_dir = super::resolve_scope_dir(version, repo_path);
    let changelog_path = scope_dir.join("CHANGELOG.md");
    let changelog_errors = super::precheck_version_changelog(version, &changelog_path);
    PrecheckResult {
        version: version.to_string(),
        normalized,
        scope_dir,
        changelog_path,
        changelog_errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_precheck_ok() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
        let result = run_precheck("v1.0.0", d.path());
        assert_eq!(result.version, "v1.0.0");
        assert_eq!(result.normalized, "1.0.0");
        assert!(result.changelog_ok());
    }

    #[test]
    fn test_run_precheck_scoped() {
        let d = tempfile::tempdir().unwrap();
        let result = run_precheck("cli/v1.0.0", d.path());
        assert_eq!(result.version, "cli/v1.0.0");
        assert_eq!(result.normalized, "1.0.0");
    }

    #[test]
    fn test_run_precheck_changelog_not_found() {
        let d = tempfile::tempdir().unwrap();
        let result = run_precheck("v1.0.0", d.path());
        assert!(result.changelog_errors.iter().any(|e| e.contains("不存在")));
    }

    #[test]
    fn test_run_precheck_changelog_missing_entry() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
        let result = run_precheck("v2.0.0", d.path());
        assert!(result.changelog_errors.iter().any(|e| e.contains("未找到")));
    }

    #[test]
    fn test_run_precheck_scope_dir_resolved() {
        let d = tempfile::tempdir().unwrap();
        let contract_dir = d.path().join(".quanttide/devops");
        std::fs::create_dir_all(&contract_dir).unwrap();
        std::fs::write(
            contract_dir.join("contract.yaml"),
            "scopes:\n  cli:\n    dir: packages/cli\n    language: rust\n",
        )
        .unwrap();
        std::fs::create_dir_all(d.path().join("packages/cli")).unwrap();
        let result = run_precheck("cli/v0.1.0", d.path());
        assert!(result.scope_dir.ends_with("packages/cli"), "scope_dir 应指向 packages/cli, 得到 {:?}", result.scope_dir);
    }
}

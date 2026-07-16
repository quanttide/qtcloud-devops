use std::path::Path;

/// 检查工作区是否有未提交的变更。
pub fn is_working_tree_dirty(repo_path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_working_tree_dirty_in_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        let dirty = is_working_tree_dirty(d.path());
        assert!(!dirty);
    }
}

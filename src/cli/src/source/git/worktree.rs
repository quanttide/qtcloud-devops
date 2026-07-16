//! 工作区操作：stage、commit、clean 检查。

use std::path::{Path, PathBuf};

/// Stage 一个或多个文件，返回是否成功。
///
/// 文件路径相对于 `repo_path`。文件不存在时静默跳过。
pub fn stage_files(repo_path: &Path, files: &[PathBuf]) -> bool {
    if files.is_empty() {
        return true;
    }
    let rel_paths: Vec<&str> = files
        .iter()
        .filter_map(|f| f.strip_prefix(repo_path).ok())
        .filter_map(|r| r.to_str())
        .collect();
    if rel_paths.is_empty() {
        return true;
    }
    let mut args = vec!["add"];
    args.extend(rel_paths);
    super::git_check(&args, repo_path)
}

/// 提交当前 staged 变更，返回是否成功。
pub fn commit(repo_path: &Path, message: &str) -> bool {
    super::git_check(&["commit", "-m", message], repo_path)
}

/// 检查工作区是否干净（无未提交变更）。
pub fn is_clean(repo_path: &Path) -> bool {
    super::git_check(&["diff", "--quiet"], repo_path)
        && super::git_check(&["diff", "--cached", "--quiet"], repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_init(path: &Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn write_file(path: &Path, name: &str, content: &str) -> PathBuf {
        let p = path.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn test_stage_files_single() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        let f = write_file(d.path(), "a.txt", "hello");
        assert!(stage_files(d.path(), &[f]));
    }

    #[test]
    fn test_stage_files_empty_list() {
        let d = tempfile::tempdir().unwrap();
        assert!(stage_files(d.path(), &[]));
    }

    #[test]
    fn test_commit_after_stage() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        let f = write_file(d.path(), "f", "content");
        stage_files(d.path(), &[f]);
        assert!(commit(d.path(), "initial"));
    }

    #[test]
    fn test_commit_without_stage_fails() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        write_file(d.path(), "f", "content");
        assert!(!commit(d.path(), "should fail"));
    }
}

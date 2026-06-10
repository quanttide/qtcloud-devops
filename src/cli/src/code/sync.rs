use std::path::PathBuf;

use crate::git::submodule::GitSubmoduleEditor;

pub fn sync(root: PathBuf, name: &str) -> Result<(), String> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name).map_err(|e| format!("同步失败: {}", e))
}

pub fn sync_all(root: PathBuf) -> Result<(), String> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_all_to_parent().map_err(|e| format!("同步失败: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git_init(path: &std::path::Path) {
        std::process::Command::new("git").args(["init", "-b", "main"]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["config", "user.email", "t@t"]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["config", "user.name", "t"]).current_dir(path).output().unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        std::process::Command::new("git").args(["add", "."]).current_dir(path).output().unwrap();
        std::process::Command::new("git").args(["commit", "-m", msg]).current_dir(path).output().unwrap();
    }

    #[test]
    fn test_sync_nonexistent_name() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path()); git_commit(d.path(), "init");
        assert!(sync(d.path().to_path_buf(), "no-such-module").is_err());
    }

    #[test]
    fn test_sync_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(sync(d.path().to_path_buf(), "x").is_err());
    }

    #[test]
    fn test_sync_all_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path()); git_commit(d.path(), "init");
        assert!(sync_all(d.path().to_path_buf()).is_ok());
    }
}

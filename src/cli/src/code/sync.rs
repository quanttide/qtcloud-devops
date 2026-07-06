use std::path::PathBuf;

use crate::git::GitSubmoduleEditor;

pub fn sync(root: PathBuf, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name)
}

pub fn sync_all(root: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_all_to_parent()
}

#[cfg(test)]
mod tests {
    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents).unwrap();
    }

    use super::*;



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

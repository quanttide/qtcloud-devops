use std::path::{Path, PathBuf};
use std::process::Command;

use kse_core::commands::editor::GitSubmoduleEditor;
use kse_core::commands::SubmoduleEditor;
use kse_core::model::{RepoState, SubmoduleStatus};

fn init_repo(path: &Path) -> git2::Repository {
    let repo = git2::Repository::init(path).unwrap();
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "test").ok();
    cfg.set_str("user.email", "test@test.com").ok();

    std::fs::write(path.join("README.md"), "# test\n").unwrap();
    let sig = git2::Signature::now("test", "test@test.com").unwrap();

    // First commit on main branch
    {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "initial commit",
            &tree,
            &[],
        )
        .unwrap();
    }

    // Second commit
    {
        std::fs::write(path.join("file.txt"), "content\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo
            .find_reference("refs/heads/main")
            .unwrap()
            .peel_to_commit()
            .unwrap();
        repo.commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "second commit",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    repo.set_head("refs/heads/main").unwrap();
    repo
}

fn add_submodule_via_git(parent: &Path, remote: &Path, name: &str) -> PathBuf {
    let url = format!("file://{}", remote.canonicalize().unwrap().display());
    let output = Command::new("git")
        .args(["submodule", "add", "--branch", "main", &url, name])
        .current_dir(parent)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .expect("failed to execute git submodule add");
    assert!(output.status.success(), "git submodule add failed");
    parent.to_path_buf()
}

#[test]
#[ignore]
fn test_health_check_empty_repo() {
    let tmp = tempfile::tempdir().unwrap();
    let repo_path = tmp.path().join("repo");
    git2::Repository::init(&repo_path).unwrap();
    let state = RepoState::scan(&repo_path).unwrap();
    assert_eq!(state.total, 0);
}

#[test]
#[ignore]
fn test_status_reports_issues() {
    let tmp = tempfile::tempdir().unwrap();
    let (remote_path, parent_path) = setup_repo_pair(&tmp);
    let _parent = add_submodule_via_git(&parent_path, &remote_path, "lib-hc1");
    std::fs::write(parent_path.join("lib-hc1").join("d.txt"), "d").unwrap();
    let editor = GitSubmoduleEditor::new(parent_path);
    let issues = editor.status().unwrap();
    let dirty_issues: Vec<_> = issues
        .iter()
        .filter(|i| i.status == SubmoduleStatus::Dirty)
        .collect();
    assert!(
        !dirty_issues.is_empty(),
        "dirty submodule should produce issue"
    );
}

#[test]
#[ignore]
fn test_sync_to_parent() {
    let tmp = tempfile::tempdir().unwrap();
    let (remote_path, parent_path) = setup_repo_pair(&tmp);
    let parent = add_submodule_via_git(&parent_path, &remote_path, "lib-sync");
    let editor = GitSubmoduleEditor::new(parent);
    let sub_repo = git2::Repository::open(&parent_path.join("lib-sync")).unwrap();
    let workdir = sub_repo.workdir().unwrap().to_path_buf();
    std::fs::write(workdir.join("new.txt"), "content\n").unwrap();
    let sig = git2::Signature::now("test", "test@test.com").unwrap();
    let mut index = sub_repo.index().unwrap();
    index.add_path(Path::new("new.txt")).unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = sub_repo.find_tree(tree_id).unwrap();
    let head = sub_repo.head().unwrap().peel_to_commit().unwrap();
    sub_repo
        .commit(Some("HEAD"), &sig, &sig, "test commit", &tree, &[&head])
        .unwrap();
    editor.sync_to_parent("lib-sync").unwrap();
    let state = RepoState::scan(&parent_path).unwrap();
    assert_eq!(state.total, 1);
}

#[test]
#[ignore]
fn test_retire_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let (remote_path, parent_path) = setup_repo_pair(&tmp);
    let parent = add_submodule_via_git(&parent_path, &remote_path, "lib-ret");
    assert_eq!(RepoState::scan(&parent_path).unwrap().total, 1);
    let editor = GitSubmoduleEditor::new(parent);
    editor.retire_submodule("lib-ret").unwrap();
    assert_eq!(RepoState::scan(&parent_path).unwrap().total, 0);
}

fn setup_repo_pair(tmp: &tempfile::TempDir) -> (PathBuf, PathBuf) {
    let remote_path = tmp.path().join("remote");
    init_repo(&remote_path);
    let parent_path = tmp.path().join("parent");
    init_repo(&parent_path);
    (remote_path, parent_path)
}

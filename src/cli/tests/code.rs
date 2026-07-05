use std::path::PathBuf;
use std::process::Command;

use qtcloud_devops_cli::git::GitSubmoduleEditor;

fn git_init(path: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(path)
        .output()
        .unwrap();
}

fn git_commit(path: &std::path::Path, msg: &str) {
    std::fs::write(path.join("file"), msg).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(path)
        .output()
        .unwrap();
}

fn setup_repo_with_submodule(tmp: &std::path::Path) -> PathBuf {
    let parent = tmp.join("parent");
    let sub = tmp.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&sub)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&sub)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&sub)
        .output()
        .unwrap();
    std::fs::write(sub.join("file"), "init sub").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&sub)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init sub"])
        .current_dir(&sub)
        .output()
        .unwrap();
    std::fs::create_dir_all(&parent).unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(&parent)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(&parent)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(&parent)
        .output()
        .unwrap();
    std::fs::write(parent.join("file"), "init parent").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&parent)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "init parent"])
        .current_dir(&parent)
        .output()
        .unwrap();
    Command::new("git")
        .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
        .current_dir(&parent)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", "add submodule"])
        .current_dir(&parent)
        .output()
        .unwrap();
    parent
}

fn editor_sync(root: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    GitSubmoduleEditor::new(root.to_path_buf()).sync_to_parent(name)
}

fn editor_sync_all(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    GitSubmoduleEditor::new(root.to_path_buf()).sync_all_to_parent()
}

#[test]
fn test_integration_scan_submodule() {
    let t = tempfile::tempdir().unwrap();
    let p = setup_repo_with_submodule(t.path());
    let s = qtcloud_devops_cli::git::RepoState::scan(&p).unwrap();
    assert_eq!(s.total, 1);
    assert_eq!(s.submodules[0].name, "libs/sub");
}
#[test]
fn test_integration_scan_no_gitmodules() {
    assert!(
        qtcloud_devops_cli::git::RepoState::scan(&tempfile::tempdir().unwrap().path()).is_err()
    );
}
#[test]
fn test_integration_sync_submodule() {
    let t = tempfile::tempdir().unwrap();
    assert!(editor_sync(&setup_repo_with_submodule(t.path()), "libs/sub").is_ok());
}
#[test]
fn test_integration_sync_nonexistent() {
    let t = tempfile::tempdir().unwrap();
    git_init(t.path());
    git_commit(t.path(), "init");
    assert!(editor_sync(t.path(), "no-such-module").is_err());
}
#[test]
fn test_integration_sync_all() {
    let t = tempfile::tempdir().unwrap();
    assert!(editor_sync_all(&setup_repo_with_submodule(t.path())).is_ok());
}
#[test]
fn test_integration_sync_all_no_submodules() {
    let t = tempfile::tempdir().unwrap();
    git_init(t.path());
    git_commit(t.path(), "init");
    assert!(editor_sync_all(t.path()).is_ok());
}
#[test]
fn test_integration_status_clean_submodule() {
    let t = tempfile::tempdir().unwrap();
    assert!(GitSubmoduleEditor::new(setup_repo_with_submodule(t.path()))
        .status()
        .unwrap()
        .is_empty());
}
#[test]
fn test_integration_status_not_a_repo() {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join(".gitmodules"), "").unwrap();
    assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
        .status()
        .is_err());
}
#[test]
fn test_integration_sync_fails_without_submodule_repo() {
    let t = tempfile::tempdir().unwrap();
    git_init(t.path());
    git_commit(t.path(), "init");
    assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
        .sync_to_parent("no-such-module")
        .is_err());
}
#[test]
fn test_integration_sync_all_on_repo_without_submodules() {
    let t = tempfile::tempdir().unwrap();
    git_init(t.path());
    git_commit(t.path(), "init");
    assert!(GitSubmoduleEditor::new(t.path().to_path_buf())
        .sync_all_to_parent()
        .is_ok());
}
#[test]
fn test_integration_status_with_fetch() {
    let t = tempfile::tempdir().unwrap();
    let mut e = GitSubmoduleEditor::new(setup_repo_with_submodule(t.path()));
    e.set_offline(true);
    assert!(e.status().unwrap().is_empty());
}

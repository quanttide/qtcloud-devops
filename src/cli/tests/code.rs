use std::path::PathBuf;
use std::process::Command;

use qtcloud_devops_cli::commands::SubmoduleEditor;

fn git_init(repo: &std::path::Path) {
    Command::new("git").args(["init"]).current_dir(repo).output().unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(repo)
        .output()
        .unwrap();
}

fn git_commit(repo: &std::path::Path, msg: &str) {
    std::fs::write(repo.join("file"), msg).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "-m", msg])
        .current_dir(repo)
        .output()
        .unwrap();
}

fn setup_repo_with_submodule(tmp: &std::path::Path) -> PathBuf {
    let parent = tmp.join("parent");
    let sub = tmp.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    git_init(&sub);
    git_commit(&sub, "init sub");
    std::fs::create_dir_all(&parent).unwrap();
    git_init(&parent);
    git_commit(&parent, "init parent");
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
    let editor = qtcloud_devops_cli::commands::code::GitSubmoduleEditor::new(root.to_path_buf());
    editor.sync_to_parent(name)
}

fn editor_sync_all(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::code::GitSubmoduleEditor::new(root.to_path_buf());
    editor.sync_all_to_parent()
}

fn editor_retire(root: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::code::GitSubmoduleEditor::new(root.to_path_buf());
    editor.retire_submodule(name)
}

#[test]
fn test_integration_scan_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let state = qtcloud_devops_cli::model::RepoState::scan(&parent).unwrap();
    assert_eq!(state.total, 1);
    assert_eq!(state.submodules[0].name, "libs/sub");
}

#[test]
fn test_integration_scan_no_gitmodules() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(qtcloud_devops_cli::model::RepoState::scan(tmp.path()).is_err());
}

#[test]
fn test_integration_sync_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    assert!(editor_sync(&parent, "libs/sub").is_ok());
}

#[test]
fn test_integration_sync_nonexistent() {
    let tmp = tempfile::tempdir().unwrap();
    git_init(tmp.path());
    git_commit(tmp.path(), "init");
    assert!(editor_sync(tmp.path(), "no-such-module").is_err());
}

#[test]
fn test_integration_sync_all() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    assert!(editor_sync_all(&parent).is_ok());
}

#[test]
fn test_integration_sync_all_no_submodules() {
    let tmp = tempfile::tempdir().unwrap();
    git_init(tmp.path());
    git_commit(tmp.path(), "init");
    assert!(editor_sync_all(tmp.path()).is_ok());
}

#[test]
fn test_integration_retire_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    editor_retire(&parent, "libs/sub").unwrap();
    assert!(
        !parent.join(".gitmodules").exists()
            || !std::fs::read_to_string(parent.join(".gitmodules"))
                .unwrap()
                .contains("libs/sub")
    );
}

#[test]
fn test_integration_retire_nonexistent() {
    let tmp = tempfile::tempdir().unwrap();
    git_init(tmp.path());
    git_commit(tmp.path(), "init");
    assert!(editor_retire(tmp.path(), "no-such-module").is_err());
}

#[test]
fn test_integration_status_clean_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let editor = qtcloud_devops_cli::commands::code::GitSubmoduleEditor::new(parent);
    assert!(editor.status().unwrap().is_empty());
}

#[test]
fn test_integration_status_not_a_repo() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".gitmodules"), "").unwrap();
    let editor = qtcloud_devops_cli::commands::code::GitSubmoduleEditor::new(tmp.path().to_path_buf());
    assert!(editor.status().is_err());
}

use std::path::PathBuf;
use std::process::Command;

use qtcloud_devops_cli::commands::SubmoduleEditor;

/// Helper: init a git repo with user config
fn git_init(repo: &std::path::Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(repo)
        .output()
        .unwrap();
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

fn repo_state_scan(root: &std::path::Path) -> qtcloud_devops_cli::model::RepoState {
    qtcloud_devops_cli::model::RepoState::scan(root).unwrap()
}

fn editor_sync(root: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::editor::GitSubmoduleEditor::new(root.to_path_buf());
    editor.sync_to_parent(name)
}

fn editor_sync_all(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::editor::GitSubmoduleEditor::new(root.to_path_buf());
    editor.sync_all_to_parent()
}

fn editor_retire(root: &std::path::Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::editor::GitSubmoduleEditor::new(root.to_path_buf());
    editor.retire_submodule(name)
}

fn editor_status(
    root: &std::path::Path,
) -> Result<Vec<qtcloud_devops_cli::commands::HealthIssue>, Box<dyn std::error::Error>> {
    let editor = qtcloud_devops_cli::commands::editor::GitSubmoduleEditor::new(root.to_path_buf());
    editor.status()
}

#[test]
fn test_integration_scan_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let state = repo_state_scan(&parent);
    assert_eq!(state.total, 1);
    assert_eq!(state.submodules[0].name, "libs/sub");
}

#[test]
fn test_integration_scan_no_gitmodules() {
    let tmp = tempfile::tempdir().unwrap();
    let state = repo_state_scan(tmp.path());
    assert_eq!(state.total, 0);
}

#[test]
fn test_integration_sync_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let result = editor_sync(&parent, "libs/sub");
    assert!(result.is_ok());
}

#[test]
fn test_integration_sync_nonexistent() {
    let tmp = tempfile::tempdir().unwrap();
    git_init(tmp.path());
    git_commit(tmp.path(), "init");
    let result = editor_sync(tmp.path(), "no-such-module");
    assert!(result.is_err());
}

#[test]
fn test_integration_sync_all() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let result = editor_sync_all(&parent);
    assert!(result.is_ok());
}

#[test]
fn test_integration_sync_all_no_submodules() {
    let tmp = tempfile::tempdir().unwrap();
    git_init(tmp.path());
    git_commit(tmp.path(), "init");
    let result = editor_sync_all(tmp.path());
    assert!(result.is_ok());
}

#[test]
fn test_integration_retire_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let result = editor_retire(&parent, "libs/sub");
    assert!(result.is_ok());
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
    let result = editor_retire(tmp.path(), "no-such-module");
    assert!(result.is_err());
}

#[test]
fn test_integration_status_clean_submodule() {
    let tmp = tempfile::tempdir().unwrap();
    let parent = setup_repo_with_submodule(tmp.path());
    let issues = editor_status(&parent).unwrap();
    assert!(issues.is_empty());
}

#[test]
fn test_integration_status_not_a_repo() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join(".gitmodules"), "").unwrap();
    let result = editor_status(tmp.path());
    assert!(result.is_err());
}

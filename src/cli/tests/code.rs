use std::process::Command;

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

fn setup_repo_with_submodule(tmp: &std::path::Path) -> std::path::PathBuf {
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

#[test]
fn test_integration_scan_submodule() {
    let t = tempfile::tempdir().unwrap();
    let p = setup_repo_with_submodule(t.path());
    let s = qtcloud_devops_cli::source::git_submodule::RepoState::scan(&p).unwrap();
    assert_eq!(s.total, 1);
    assert_eq!(s.submodules[0].name, "libs/sub");
}

#[test]
fn test_integration_scan_no_gitmodules() {
    assert!(
        qtcloud_devops_cli::source::git_submodule::RepoState::scan(&tempfile::tempdir().unwrap().path()).is_err()
    );
}

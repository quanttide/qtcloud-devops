use std::io::Write;

fn git_init(path: &std::path::Path) {
    std::process::Command::new("git")
        .args(["init", "-b", "main"]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "t@t"]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "t"]).current_dir(path).output().unwrap();
    std::fs::write(path.join("f"), "").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(path).output().unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"]).current_dir(path).output().unwrap();
}

fn git_commit(repo: &std::path::Path) {
    let mut f = std::fs::File::create(repo.join("f")).unwrap();
    writeln!(f, "x").unwrap();
    std::process::Command::new("git")
        .args(["add", "."]).current_dir(repo).output().unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "x"]).current_dir(repo).output().unwrap();
}

#[test]
fn test_release_create_tag_uses_repo_path() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path()); git_commit(dir.path());
    let tag = "v999.999.999-test-repo-path";
    assert!(qtcloud_devops_cli::release::create_tag(tag, dir.path()));
    let output = std::process::Command::new("git").args(["-C", dir.path().to_str().unwrap(), "tag", "-l"]).output().unwrap();
    assert!(String::from_utf8_lossy(&output.stdout).contains(tag));
    let cwd_tags = std::process::Command::new("git").args(["tag", "-l"]).output().unwrap();
    assert!(!String::from_utf8_lossy(&cwd_tags.stdout).contains(tag));
}

#[test]
fn test_release_publish_rejects_invalid_version() {
    assert!(qtcloud_devops_cli::release::publish("bad", tempfile::tempdir().unwrap().path(), true, false, None).is_err());
}

#[test]
fn test_release_publish_pre_release_rejects_formal() {
    assert!(qtcloud_devops_cli::release::publish("v1.0.0", tempfile::tempdir().unwrap().path(), true, true, None).unwrap_err().to_string().contains("--pre-release"));
}

#[test]
fn test_release_publish_rejects_missing_changelog() {
    let dir = tempfile::tempdir().unwrap(); git_init(dir.path());
    let err = qtcloud_devops_cli::release::publish("v1.0.0", dir.path(), true, false, None).unwrap_err().to_string();
    assert!(err.contains("CHANGELOG"), "预期 CHANGELOG 错误，得到: {}", err);
}

#[test]
fn test_release_publish_idempotent() {
    let dir = tempfile::tempdir().unwrap(); git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap();
    assert!(qtcloud_devops_cli::release::publish("v1.0.0-rc.1", dir.path(), true, true, None).is_ok());
    assert!(qtcloud_devops_cli::release::publish("v1.0.0-rc.1", dir.path(), true, true, None).is_ok());
}

#[test]
fn test_release_publish_without_changelog_entry() {
    let dir = tempfile::tempdir().unwrap(); git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0]\n\ncontent\n").unwrap();
    let err = qtcloud_devops_cli::release::publish("v2.0.0", dir.path(), true, false, None).unwrap_err().to_string();
    assert!(err.contains("未找到"));
}

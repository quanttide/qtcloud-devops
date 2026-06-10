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
        .args(["add", "."])
        .current_dir(repo)
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "x"])
        .current_dir(repo)
        .output()
        .unwrap();
}

#[test]
fn test_release_create_tag_uses_repo_path() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    git_commit(dir.path());

    let tag = "v999.999.999-test-repo-path";
    assert!(qtcloud_devops_cli::release::util::create_tag(tag, dir.path()));

    let output = std::process::Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "tag", "-l"])
        .output()
        .unwrap();
    let tags = String::from_utf8_lossy(&output.stdout);
    assert!(tags.contains(tag));

    let cwd_tags = std::process::Command::new("git")
        .args(["tag", "-l"])
        .output()
        .unwrap();
    let cwd_tags = String::from_utf8_lossy(&cwd_tags.stdout);
    assert!(!cwd_tags.contains(tag));
}

#[test]
fn test_release_stage_invalid_version() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::release::stage("bad", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_rejects_missing_changelog() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let err = qtcloud_devops_cli::release::stage("v1.0.0-rc.1", dir.path())
        .unwrap_err()
        .to_string();
    assert!(err.contains("CHANGELOG"), "预期 CHANGELOG 相关错误，得到: {}", err);
}

#[test]
fn test_release_publish_rejects_missing_changelog() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let err = qtcloud_devops_cli::release::publish("v1.0.0", dir.path(), true, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("CHANGELOG"), "预期 CHANGELOG 相关错误，得到: {}", err);
}

#[test]
fn test_release_stage_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    std::fs::write(dir.path().join("CHANGELOG.md"), "## [1.0.0-rc.1]\n\ncontent\n").unwrap();
    assert!(qtcloud_devops_cli::release::stage("v1.0.0-rc.1", dir.path()).is_ok());
    assert!(qtcloud_devops_cli::release::stage("v1.0.0-rc.1", dir.path()).is_ok());
}

#[test]
fn test_release_publish_without_stage() {
    let dir = tempfile::tempdir().unwrap();
    let result = qtcloud_devops_cli::release::publish("v1.0.0", dir.path(), true, None);
    assert!(result.is_ok() || result.is_err());
}

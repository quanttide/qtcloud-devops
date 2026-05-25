use std::io::Write;

use qtcloud_devops_cli::model::release::{
    FileStorage, ReleaseRecord, ReleaseStatus, Storage,
};

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

fn make_record(version: &str, status: ReleaseStatus) -> ReleaseRecord {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    ReleaseRecord {
        id: uuid::Uuid::new_v4().to_string(),
        version: version.to_string(),
        status,
        created_at: now.clone(),
        updated_at: now,
    }
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

    // create_tag should run in dir, not CWD
    let tag = "v999.999.999-test-repo-path";
    assert!(qtcloud_devops_cli::commands::release::create_tag(tag, dir.path()));

    let output = std::process::Command::new("git")
        .args(["-C", dir.path().to_str().unwrap(), "tag", "-l"])
        .output()
        .unwrap();
    let tags = String::from_utf8_lossy(&output.stdout);
    assert!(tags.contains(tag));

    // verify tag did NOT leak to CWD
    let cwd_tags = std::process::Command::new("git")
        .args(["tag", "-l"])
        .output()
        .unwrap();
    let cwd_tags = String::from_utf8_lossy(&cwd_tags.stdout);
    assert!(!cwd_tags.contains(tag));
}

#[test]
fn test_release_prerelease_tag_exists() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    qtcloud_devops_cli::commands::release::stage("v1.0.0-rc.1", dir.path()).unwrap();
    let s = FileStorage::new(dir.path());
    let r = s.load("v1.0.0-rc.1").unwrap();
    assert_eq!(r.status, ReleaseStatus::Staged);
}

#[test]
fn test_release_retire_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::retire("v9.9.9", dir.path()).is_err()
    );
}

#[test]
fn test_release_retire_not_published() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0", ReleaseStatus::Staged))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::retire("v1.0.0", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_invalid_version() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::stage("bad", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_published_rejected() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0-rc.1", ReleaseStatus::Published))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::stage("v1.0.0-rc.1", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_retired_rejected() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0-rc.1", ReleaseStatus::Retired))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::stage("v1.0.0-rc.1", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    git_init(dir.path());
    let id1 = qtcloud_devops_cli::commands::release::stage("v1.0.0-rc.1", dir.path()).unwrap();
    let id2 = qtcloud_devops_cli::commands::release::stage("v1.0.0-rc.1", dir.path()).unwrap();
    assert_eq!(id1, id2);
}

#[test]
fn test_release_publish_not_staged() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0", ReleaseStatus::Cancelled)).unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::publish("v1.0.0", dir.path(), true).is_err()
    );
}

#[test]
fn test_release_publish_not_found() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::release::publish("v1.0.0", dir.path(), true).is_err()
    );
}

#[test]
fn test_release_retire_from_published() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    let r = make_record("v1.0.0", ReleaseStatus::Published);
    s.save(&r).unwrap();
    qtcloud_devops_cli::commands::release::retire("v1.0.0", dir.path()).unwrap();
    let s = FileStorage::new(dir.path());
    assert_eq!(s.load("v1.0.0").unwrap().status, ReleaseStatus::Retired);
}

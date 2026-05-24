use qtcloud_devops_cli::model::release::{
    FileStorage, ReleaseRecord, ReleaseStatus, Storage,
};

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

#[test]
fn test_release_stage_then_cancel() {
    let dir = tempfile::tempdir().unwrap();
    qtcloud_devops_cli::commands::stage::run("v1.0.0", dir.path()).unwrap();

    let s = FileStorage::new(dir.path());
    let r = s.load("v1.0.0").unwrap();
    assert_eq!(r.status, ReleaseStatus::Staged);

    qtcloud_devops_cli::commands::cancel::run("v1.0.0", dir.path()).unwrap();
    let s = FileStorage::new(dir.path());
    let r = s.load("v1.0.0").unwrap();
    assert_eq!(r.status, ReleaseStatus::Cancelled);
}

#[test]
fn test_release_cancel_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::cancel::run("v9.9.9", dir.path()).is_err()
    );
}

#[test]
fn test_release_retire_nonexistent() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::retire::run("v9.9.9", dir.path()).is_err()
    );
}

#[test]
fn test_release_retire_not_published() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0", ReleaseStatus::Staged))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::retire::run("v1.0.0", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_invalid_version() {
    let dir = tempfile::tempdir().unwrap();
    assert!(
        qtcloud_devops_cli::commands::stage::run("bad", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_published_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0", ReleaseStatus::Published))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::stage::run("v1.0.0", dir.path()).is_err()
    );
}

#[test]
fn test_release_stage_retired_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let mut s = FileStorage::new(dir.path());
    s.save(&make_record("v1.0.0", ReleaseStatus::Retired))
        .unwrap();
    assert!(
        qtcloud_devops_cli::commands::stage::run("v1.0.0", dir.path()).is_err()
    );
}

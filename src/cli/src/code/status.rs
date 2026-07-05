use std::path::PathBuf;

use super::model::{ComponentStatus, StatusReport, SyncStatus};
use crate::git::{RepoState, SubmoduleStatus};

fn map_status(s: &SubmoduleStatus) -> SyncStatus {
    match s {
        SubmoduleStatus::Clean => SyncStatus::Synced,
        SubmoduleStatus::AheadOfParent => SyncStatus::PendingPush,
        SubmoduleStatus::BehindRemote => SyncStatus::PendingPull,
        _ => SyncStatus::Conflict,
    }
}

pub fn status(root: PathBuf, offline: bool) -> Result<StatusReport, String> {
    let state = if offline {
        RepoState::scan_offline(&root)
    } else {
        RepoState::scan(&root)
    }
    .map_err(|e| format!("扫描失败: {}", e))?;

    let mut components = Vec::with_capacity(state.submodules.len());
    for sm in &state.submodules {
        let s = map_status(&sm.status);
        components.push(ComponentStatus {
            name: sm.name.clone(),
            status: s,
            ahead: sm.ahead_count,
            behind: sm.behind_count,
        });
    }

    let total = components.len();
    let synced = components
        .iter()
        .filter(|c| c.status == SyncStatus::Synced)
        .count();
    let pending = total - synced;

    Ok(StatusReport {
        root: state.root_path.to_string_lossy().to_string(),
        components,
        total,
        synced,
        pending,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{git_init, git_commit};
    use crate::git::SubmoduleStatus;

    // ---- map_status ----

    #[test]
    fn test_map_clean() {
        assert_eq!(map_status(&SubmoduleStatus::Clean), SyncStatus::Synced);
    }
    #[test]
    fn test_map_ahead() {
        assert_eq!(
            map_status(&SubmoduleStatus::AheadOfParent),
            SyncStatus::PendingPush
        );
    }
    #[test]
    fn test_map_behind() {
        assert_eq!(
            map_status(&SubmoduleStatus::BehindRemote),
            SyncStatus::PendingPull
        );
    }
    #[test]
    fn test_map_detached() {
        assert_eq!(map_status(&SubmoduleStatus::Detached), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_dirty() {
        assert_eq!(map_status(&SubmoduleStatus::Dirty), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_orphaned() {
        assert_eq!(map_status(&SubmoduleStatus::Orphaned), SyncStatus::Conflict);
    }
    #[test]
    fn test_map_uninitialized() {
        assert_eq!(
            map_status(&SubmoduleStatus::Uninitialized),
            SyncStatus::Conflict
        );
    }

    // ---- status (integration) ----



    #[test]
    fn test_status_non_git_dir() {
        let d = tempfile::tempdir().unwrap();
        assert!(status(d.path().to_path_buf(), false).is_err());
    }

    #[test]
    fn test_status_empty_repo() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        let report = status(d.path().to_path_buf(), false).unwrap();
        assert_eq!(report.total, 0);
        assert_eq!(report.synced, 0);
        assert_eq!(report.pending, 0);
    }

    #[test]
    fn test_status_with_synced_submodule() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("parent");
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        git_init(&sub);
        git_commit(&sub, "init sub");
        std::fs::create_dir_all(&parent).unwrap();
        git_init(&parent);
        git_commit(&parent, "init parent");
        std::process::Command::new("git")
            .args(["submodule", "add", &sub.to_string_lossy(), "libs/sub"])
            .current_dir(&parent)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "add submodule"])
            .current_dir(&parent)
            .output()
            .unwrap();
        let report = status(parent, false).unwrap();
        assert_eq!(report.total, 1);
        assert_eq!(report.components[0].status, SyncStatus::Synced);
    }

    #[test]
    fn test_status_offline_flag() {
        let d = tempfile::tempdir().unwrap();
        git_init(d.path());
        git_commit(d.path(), "init");
        // offline should not prevent scanning a repo without submodules
        assert!(status(d.path().to_path_buf(), true).is_ok());
    }
}

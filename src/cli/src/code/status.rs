use std::path::PathBuf;

use crate::source::git::submodule::{RepoState, SubmoduleStatus, scan_repo_state, scan_repo_state_offline};

// ═══════════════════════════════════════════════════════════════════════
// 模型
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Synced,
    PendingPush,
    PendingPull,
    Conflict,
}

impl SyncStatus {
    pub fn label(&self) -> &str {
        match self {
            Self::Synced => "已同步",
            Self::PendingPush => "待推送",
            Self::PendingPull => "待拉取",
            Self::Conflict => "冲突",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ComponentStatus {
    pub name: String,
    pub status: SyncStatus,
    pub ahead: usize,
    pub behind: usize,
}

#[derive(Debug, Clone)]
pub struct StatusReport {
    pub root: String,
    pub components: Vec<ComponentStatus>,
    pub total: usize,
    pub synced: usize,
    pub pending: usize,
}

// ═══════════════════════════════════════════════════════════════════════
// status — 子模块同步状态
// ═══════════════════════════════════════════════════════════════════════

fn map_status(s: &SubmoduleStatus) -> SyncStatus {
    match s {
        SubmoduleStatus::Clean => SyncStatus::Synced,
        SubmoduleStatus::AheadOfParent => SyncStatus::PendingPush,
        SubmoduleStatus::BehindRemote => SyncStatus::PendingPull,
        _ => SyncStatus::Conflict,
    }
}

pub fn status(root: PathBuf, offline: bool) -> Result<StatusReport, Box<dyn std::error::Error>> {
    let state = if offline {
        scan_repo_state_offline(&root)
    } else {
        scan_repo_state(&root)
    }?;
    let mut components = Vec::with_capacity(state.submodules.len());
    for sm in &state.submodules {
        components.push(ComponentStatus {
            name: sm.name.clone(),
            status: map_status(&sm.status),
            ahead: sm.ahead_count,
            behind: sm.behind_count,
        });
    }
    let total = components.len();
    let synced = components
        .iter()
        .filter(|c| c.status == SyncStatus::Synced)
        .count();
    Ok(StatusReport {
        root: state.root_path.to_string_lossy().to_string(),
        components,
        total,
        synced,
        pending: total - synced,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── SyncStatus ──────────────────────────────────────────────

    #[test]
    fn test_sync_status_labels() {
        assert_eq!(SyncStatus::Synced.label(), "已同步");
        assert_eq!(SyncStatus::PendingPush.label(), "待推送");
        assert_eq!(SyncStatus::PendingPull.label(), "待拉取");
        assert_eq!(SyncStatus::Conflict.label(), "冲突");
    }

    #[test]
    fn test_sync_status_clone_eq() {
        assert_eq!(SyncStatus::Synced, SyncStatus::Synced);
        assert_ne!(SyncStatus::Synced, SyncStatus::PendingPush);
    }

    #[test]
    fn test_component_status_builder() {
        let c = ComponentStatus {
            name: "libs/foo".into(),
            status: SyncStatus::PendingPush,
            ahead: 3,
            behind: 0,
        };
        assert_eq!(c.name, "libs/foo");
        assert_eq!(c.ahead, 3);
    }

    #[test]
    fn test_status_report_counts() {
        let report = StatusReport {
            root: "/tmp".into(),
            components: vec![
                ComponentStatus {
                    name: "a".into(),
                    status: SyncStatus::Synced,
                    ahead: 0,
                    behind: 0,
                },
                ComponentStatus {
                    name: "b".into(),
                    status: SyncStatus::PendingPush,
                    ahead: 1,
                    behind: 0,
                },
                ComponentStatus {
                    name: "c".into(),
                    status: SyncStatus::PendingPull,
                    ahead: 0,
                    behind: 2,
                },
                ComponentStatus {
                    name: "d".into(),
                    status: SyncStatus::Conflict,
                    ahead: 0,
                    behind: 0,
                },
            ],
            total: 4,
            synced: 1,
            pending: 3,
        };
        assert_eq!(report.total, 4);
        assert_eq!(report.synced, 1);
        assert_eq!(report.pending, 3);
    }

    // ── map_status ──────────────────────────────────────────────

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

    // ── status (integration) ──────────────────────────────────

    fn git_init(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.email", "t@t").unwrap();
        cfg.set_str("user.name", "t").unwrap();
    }

    fn git_commit(path: &std::path::Path, msg: &str) {
        std::fs::write(path.join("f"), msg).unwrap();
        let repo = git2::Repository::open(path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("f")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().and_then(|h| h.peel_to_commit()).ok();
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
            .unwrap();
    }

    #[test]
    fn test_status_non_git_dir() {
        assert!(status(tempfile::tempdir().unwrap().path().to_path_buf(), false).is_err());
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
        assert!(status(d.path().to_path_buf(), true).is_ok());
    }
}

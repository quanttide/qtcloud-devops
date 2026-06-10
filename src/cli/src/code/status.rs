use std::path::PathBuf;

use super::model::{ComponentStatus, StatusReport, SyncStatus};
use crate::git::submodule::{RepoState, SubmoduleStatus};

pub fn status(root: PathBuf, offline: bool) -> Result<StatusReport, String> {
    let state = if offline {
        RepoState::scan_offline(&root)
    } else {
        RepoState::scan(&root)
    }
    .map_err(|e| format!("扫描失败: {}", e))?;

    let mut components = Vec::with_capacity(state.submodules.len());
    for sm in &state.submodules {
        let s = match sm.status {
            SubmoduleStatus::Clean => SyncStatus::Synced,
            SubmoduleStatus::AheadOfParent => SyncStatus::PendingPush,
            SubmoduleStatus::BehindRemote => SyncStatus::PendingPull,
            _ => SyncStatus::Conflict,
        };
        components.push(ComponentStatus {
            name: sm.name.clone(),
            status: s,
            ahead: sm.ahead_count,
            behind: sm.behind_count,
        });
    }

    let total = components.len();
    let synced = components.iter().filter(|c| c.status == SyncStatus::Synced).count();
    let pending = total - synced;

    Ok(StatusReport {
        root: state.root_path.to_string_lossy().to_string(),
        components,
        total,
        synced,
        pending,
    })
}

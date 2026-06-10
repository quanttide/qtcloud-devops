use std::path::PathBuf;

use crate::git::submodule::GitSubmoduleEditor;

use super::model::SyncStatus;

/// 同步单个组件：将本地变更推送到远端并更新父仓库指针。
/// 返回业务状态（已同步 / 待推送 / 待拉取 / 冲突）。
pub fn sync(root: PathBuf, name: &str) -> Result<SyncStatus, String> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name).map_err(|e| format!("同步失败: {}", e))?;
    Ok(SyncStatus::Synced)
}

/// 同步所有组件。
pub fn sync_all(root: PathBuf) -> Vec<(String, Result<SyncStatus, String>)> {
    let editor = GitSubmoduleEditor::new(root);
    let repo = match git2::Repository::open(&editor.root()) {
        Ok(r) => r,
        Err(e) => return vec![("(全局)".into(), Err(format!("无法打开仓库: {}", e)))],
    };
    let submodules = match repo.submodules() {
        Ok(s) => s,
        Err(e) => return vec![("(全局)".into(), Err(format!("无法读取子模块: {}", e)))],
    };
    let mut results = Vec::new();
    for sm in submodules.iter() {
        let name = sm.name().unwrap_or("unknown").to_string();
        let editor = GitSubmoduleEditor::new(editor.root().to_path_buf());
        let result = editor.sync_to_parent(&name).map(|_| SyncStatus::Synced).map_err(|e| format!("同步失败: {}", e));
        results.push((name, result));
    }
    results
}

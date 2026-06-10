use std::path::PathBuf;

/// 扫描仓库子模块状态并返回聚合摘要文本。
/// 输出业务语言（同步/待推送/待拉取/冲突），不暴露子模块概念。
pub fn status(root: PathBuf, offline: bool) -> Result<String, Box<dyn std::error::Error>> {
    let state = crate::git::submodule::RepoState::scan(&root)?;
    let total = state.total;
    let clean = state.clean_count;
    let pending = total - clean;

    let mut parts = Vec::new();
    for sm in &state.submodules {
        let label = match sm.status {
            crate::git::submodule::SubmoduleStatus::Clean => continue,
            crate::git::submodule::SubmoduleStatus::AheadOfParent => format!("{}: 待推送", sm.name),
            crate::git::submodule::SubmoduleStatus::BehindRemote => format!("{}: 待拉取", sm.name),
            crate::git::submodule::SubmoduleStatus::Detached | crate::git::submodule::SubmoduleStatus::Dirty => format!("{}: 有修改", sm.name),
            crate::git::submodule::SubmoduleStatus::Orphaned | crate::git::submodule::SubmoduleStatus::Uninitialized => format!("{}: 异常", sm.name),
        };
        parts.push(label);
    }

    if parts.is_empty() {
        Ok(format!("全部 {} 个组件已同步", total))
    } else {
        Ok(format!("{} / {} 个组件未同步:\n  {}", pending, total, parts.join("\n  ")))
    }
}

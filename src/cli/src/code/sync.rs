use crate::git::submodule::GitSubmoduleEditor;
use std::path::PathBuf;

/// 将子模块指针同步到父仓库。
/// 封装 fetch → push 子模块 → 更新父仓库指针 → push 父仓库 四步操作。
pub fn sync(root: PathBuf, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name)
}

/// 同步所有子模块。
pub fn sync_all(root: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_all_to_parent()
}

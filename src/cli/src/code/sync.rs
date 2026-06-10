use std::path::PathBuf;

use crate::git::submodule::GitSubmoduleEditor;

pub fn sync(root: PathBuf, name: &str) -> Result<(), String> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_to_parent(name).map_err(|e| format!("同步失败: {}", e))
}

pub fn sync_all(root: PathBuf) -> Result<(), String> {
    let editor = GitSubmoduleEditor::new(root);
    editor.sync_all_to_parent().map_err(|e| format!("同步失败: {}", e))
}

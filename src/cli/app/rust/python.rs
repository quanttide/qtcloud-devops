use crate::commands::code::GitSubmoduleEditor;
use crate::commands::SubmoduleEditor;
use crate::model;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;

fn resolve_path(path: &str) -> PyResult<PathBuf> {
    let root = PathBuf::from(path);
    std::fs::canonicalize(&root)
        .map_err(|e| PyValueError::new_err(format!("无法解析路径 '{}': {}", path, e)))
}

fn state_to_dict(state: &model::RepoState) -> PyResult<PyObject> {
    let json_str = serde_json::to_string_pretty(state)
        .map_err(|e| PyRuntimeError::new_err(format!("序列化失败: {}", e)))?;
    Python::with_gil(|py| {
        let json_mod = py.import("json")?;
        let result: PyObject = json_mod.call_method1("loads", (json_str,))?.into();
        Ok(result)
    })
}

#[pyfunction]
fn scan_repo(path: String) -> PyResult<PyObject> {
    let canonical = resolve_path(&path)?;
    let state = model::RepoState::scan(&canonical)
        .map_err(|e| PyRuntimeError::new_err(format!("扫描仓库失败: {}", e)))?;
    state_to_dict(&state)
}

#[pyfunction]
fn sync_single(name: String, path: String) -> PyResult<PyObject> {
    let canonical = resolve_path(&path)?;
    let editor = GitSubmoduleEditor::new(canonical);
    editor
        .sync_to_parent(&name)
        .map_err(|e| PyRuntimeError::new_err(format!("同步子模块 '{}' 失败: {}", name, e)))?;
    Python::with_gil(|py| Ok(py.None()))
}

#[pyfunction]
fn sync_all(path: String) -> PyResult<PyObject> {
    let canonical = resolve_path(&path)?;
    let editor = GitSubmoduleEditor::new(canonical);
    editor
        .sync_all_to_parent()
        .map_err(|e| PyRuntimeError::new_err(format!("同步所有子模块失败: {}", e)))?;
    Python::with_gil(|py| Ok(py.None()))
}

#[pyfunction]
fn retire_submodule(name: String, path: String) -> PyResult<PyObject> {
    let canonical = resolve_path(&path)?;
    let editor = GitSubmoduleEditor::new(canonical);
    editor
        .retire_submodule(&name)
        .map_err(|e| PyRuntimeError::new_err(format!("退役子模块 '{}' 失败: {}", name, e)))?;
    Python::with_gil(|py| Ok(py.None()))
}

#[pymodule]
fn qtcloud_devops_cli(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_repo, m)?)?;
    m.add_function(wrap_pyfunction!(sync_single, m)?)?;
    m.add_function(wrap_pyfunction!(sync_all, m)?)?;
    m.add_function(wrap_pyfunction!(retire_submodule, m)?)?;
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "python")]
mod tests {
    use super::*;

    // ---- resolve_path ----

    #[test]
    fn test_py_resolve_path_valid() {
        let result = resolve_path(".");
        assert!(result.is_ok());
    }

    #[test]
    fn test_py_resolve_path_invalid() {
        let result = resolve_path("/__kse_no_such_path__");
        assert!(result.is_err());
    }

    // ---- state_to_dict ----

    #[test]
    fn test_state_to_dict_empty() {
        let state = model::RepoState {
            root_path: std::path::PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
        };
        let result = state_to_dict(&state);
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_to_dict_with_submodule() {
        let sm = model::Submodule {
            name: "libs/foo".into(),
            path: std::path::PathBuf::from("libs/foo"),
            url: "https://example.com/foo.git".into(),
            tracked_branch: "main".into(),
            parent_pointer: model::CommitHash("abc123".into()),
            local_head: model::CommitHash("def456".into()),
            remote_head: model::CommitHash("ghi789".into()),
            status: model::SubmoduleStatus::Clean,
            ahead_count: 0,
            behind_count: 0,
            remote_unreachable: false,
        };
        let state = model::RepoState {
            root_path: std::path::PathBuf::from("/tmp"),
            submodules: vec![sm],
            total: 1,
            clean_count: 1,
            needs_attention: vec![],
        };
        let result = state_to_dict(&state);
        assert!(result.is_ok());
    }
}

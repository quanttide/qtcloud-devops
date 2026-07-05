use crate::git::{GitSubmoduleEditor, RepoState};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use std::path::PathBuf;

fn resolve_path(path: &str) -> PyResult<PathBuf> {
    let root = PathBuf::from(path);
    std::fs::canonicalize(&root)
        .map_err(|e| PyValueError::new_err(format!("无法解析路径 '{}': {}", path, e)))
}

fn state_to_dict(state: &RepoState) -> PyResult<PyObject> {
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
    let state = crate::git::RepoState::scan(&canonical)
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

#[pymodule]
fn _native(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_repo, m)?)?;
    m.add_function(wrap_pyfunction!(sync_single, m)?)?;
    m.add_function(wrap_pyfunction!(sync_all, m)?)?;
    Ok(())
}

#[cfg(test)]
#[cfg(feature = "python")]
mod tests {
    use super::*;
    use crate::git::*;

    #[test]
    fn test_py_resolve_path_valid() {
        assert!(resolve_path(".").is_ok());
    }
    #[test]
    fn test_py_resolve_path_invalid() {
        assert!(resolve_path("/__kse_no_such_path__").is_err());
    }
    #[test]
    fn test_state_to_dict_empty() {
        let state = RepoState {
            root_path: std::path::PathBuf::from("/tmp"),
            submodules: vec![],
            total: 0,
            clean_count: 0,
            needs_attention: vec![],
        };
        assert!(state_to_dict(&state).is_ok());
    }
    #[test]
    fn test_state_to_dict_with_submodule() {
        let sm = Submodule {
            name: "libs/foo".into(),
            path: std::path::PathBuf::from("libs/foo"),
            url: "https://example.com/foo.git".into(),
            tracked_branch: "main".into(),
            parent_pointer: gix::ObjectId::null(),
            local_head: gix::ObjectId::null(),
            remote_head: gix::ObjectId::null(),
            status: SubmoduleStatus::Clean,
            ahead_count: 0,
            behind_count: 0,
            remote_unreachable: false,
        };
        let state = RepoState {
            root_path: std::path::PathBuf::from("/tmp"),
            submodules: vec![sm],
            total: 1,
            clean_count: 1,
            needs_attention: vec![],
        };
        assert!(state_to_dict(&state).is_ok());
    }
}

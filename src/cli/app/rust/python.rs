use crate::commands::editor::GitSubmoduleEditor;
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
fn qtcloud_devops_code(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_repo, m)?)?;
    m.add_function(wrap_pyfunction!(sync_single, m)?)?;
    m.add_function(wrap_pyfunction!(sync_all, m)?)?;
    m.add_function(wrap_pyfunction!(retire_submodule, m)?)?;
    Ok(())
}

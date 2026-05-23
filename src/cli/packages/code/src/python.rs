use crate::model;
use pyo3::prelude::*;
use std::path::PathBuf;

/// Scan a git repository and return its submodule status as a Python dict.
#[pyfunction]
fn scan_repo(path: String) -> PyResult<PyObject> {
    let root = PathBuf::from(&path);
    let canonical = std::fs::canonicalize(&root).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("无法解析路径 '{}': {}", path, e))
    })?;

    let state = model::RepoState::scan(&canonical)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("扫描仓库失败: {}", e)))?;

    // Serialize to JSON string, then parse into Python dict
    let json_str = serde_json::to_string_pretty(&state)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("序列化失败: {}", e)))?;

    Python::with_gil(|py| {
        let json_mod = py.import("json")?;
        let result: PyObject = json_mod.call_method1("loads", (json_str,))?.into();
        Ok(result)
    })
}

/// A Python module implemented in Rust for KSE (Kernel Submodule Editor).
#[pymodule]
fn kse_core(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(scan_repo, m)?)?;
    Ok(())
}

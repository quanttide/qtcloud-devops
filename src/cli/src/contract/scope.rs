pub use quanttide_devops::contract::{
    BuildTool, Language, Scope,
};

use std::path::Path;

/// 加载所有 scope 列表。
pub fn load_scopes(repo_path: &Path) -> Vec<Scope> {
    crate::contract::load(repo_path).scopes
}

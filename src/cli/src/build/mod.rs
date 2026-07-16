use std::path::Path;

use crate::contract;

pub(crate) mod status;
pub(crate) mod ci;
pub(crate) mod check;
pub(crate) mod clean;
pub(crate) mod audit;

pub use status::status;
pub use clean::clean;
pub use audit::audit;

/// CI 运行记录。
#[derive(Debug, PartialEq)]
pub(crate) struct CiRun {
    pub conclusion: String,
    pub title: String,
    pub branch: String,
    pub number: String,
}

pub(crate) struct ScopeInfo<'a> {
    name: &'a str,
    dir: &'a Path,
    lang: &'a contract::Language,
    c: &'a contract::Contract,
    vs: &'a contract::VersionState,
    release: &'a contract::StageRelease,
}

/// 检查工作区是否有未提交变更。
pub(crate) fn is_working_tree_dirty(repo_path: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;

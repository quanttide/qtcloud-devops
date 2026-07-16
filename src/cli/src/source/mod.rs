/// source 命令：检查系统依赖的外部命令状态。
pub mod changelog;
pub mod git;
pub mod git_tag;
pub mod roadmap;
pub mod git_submodule;

use std::path::Path;

/// 系统诊断委托给 `crate::diagnostics`。
pub fn status(repo_path: &Path) {
    crate::diagnostics::status(repo_path)
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        // source 模块的 status 已委托给 diagnostics，测试在 diagnostics 中
    }
}

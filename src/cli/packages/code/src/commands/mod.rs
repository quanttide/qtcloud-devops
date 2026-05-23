use crate::model::SubmoduleStatus;
use std::path::Path;

pub mod editor;

#[derive(Debug, Clone)]
pub struct HealthIssue {
    pub submodule_name: String,
    pub status: SubmoduleStatus,
    pub description: String,
    pub suggested_action: String,
}

pub trait SubmoduleEditor {
    fn root(&self) -> &Path;

    /// 核心贡献：子模块 → 父仓库的指针同步
    fn sync_to_parent(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn sync_all_to_parent(&self) -> Result<(), Box<dyn std::error::Error>>;

    /// 半贡献：自动反注册子模块
    fn retire_submodule(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;

    /// 核心贡献：三路 commit 比对 + 7 种状态分类
    fn status(&self) -> Result<Vec<HealthIssue>, Box<dyn std::error::Error>>;

}

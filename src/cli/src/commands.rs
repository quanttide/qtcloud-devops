use crate::model::SubmoduleStatus;
use std::path::Path;

pub mod cancel;
pub mod code;
pub mod publish;
pub mod release;
pub mod retire;
pub mod stage;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_issue_builder() {
        let issue = HealthIssue {
            submodule_name: "libs/foo".into(),
            status: SubmoduleStatus::Dirty,
            description: "有未提交的修改".into(),
            suggested_action: "提交或 stash".into(),
        };
        assert_eq!(issue.submodule_name, "libs/foo");
        assert_eq!(issue.status, SubmoduleStatus::Dirty);
    }

    #[test]
    fn test_health_issue_clone() {
        let a = HealthIssue {
            submodule_name: "a".into(),
            status: SubmoduleStatus::Clean,
            description: "d".into(),
            suggested_action: "s".into(),
        };
        let b = a.clone();
        assert_eq!(a.submodule_name, b.submodule_name);
        assert_eq!(a.status, b.status);
    }

    #[test]
    fn test_health_issue_debug() {
        let issue = HealthIssue {
            submodule_name: "m".into(),
            status: SubmoduleStatus::Orphaned,
            description: "x".into(),
            suggested_action: "y".into(),
        };
        let debug = format!("{:?}", issue);
        assert!(debug.contains("Orphaned"));
        assert!(debug.contains("m"));
    }
}

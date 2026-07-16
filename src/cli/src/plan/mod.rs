/// plan 命令：ROADMAP.md / TODO.md 规划管理。
///
/// 子命令：
/// - status — 查看 scope 规划进度
/// - clean — 删除已完成条目
/// - doctor — 修复格式问题（规则修复 + LLM 修复）
/// - audit — 审计规划与待办关系
/// - todo-from-audit / roadmap-from-audit — 从审计 JSON 更新规划文件

pub use crate::source::roadmap::Issue;

use quanttide_devops::source::roadmap::RoadmapError;

impl From<RoadmapError> for PlanError {
    fn from(e: RoadmapError) -> Self {
        PlanError::Other(e.to_string())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}


mod status;
mod clean;
mod doctor;
mod audit;
mod from_audit;

pub use status::{print_status, print_status_to, parse_roadmap, parse_roadmap_str,
    resolve_roadmap_path, resolve_roadmap_dir};
pub use clean::{clean_done_items, clean_roadmap};
pub use doctor::{doctor_file, edit_roadmap};
pub use audit::{plan_audit};
pub use from_audit::{todo_from_audit, roadmap_from_audit};



#[cfg(test)]
mod tests;

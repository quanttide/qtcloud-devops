/// 契约模块 — 适配层，委托给 `quanttide-devops` toolkit。
///
/// 按 Contract 的领域骨架组织：
/// - platform — CI/CD 平台和制品仓库
/// - source   — 配置文件检测和版本源
/// - scope    — 子项目边界定义
/// - version  — 语义化版本校验和状态
/// - stage    — 构建/测试/发布管道阶段
mod core;
mod platform;
mod scope;
mod source;
mod stage;
mod version;

pub use core::*;
pub use platform::*;
pub use scope::*;
pub use source::*;
pub use stage::*;
pub use version::*;

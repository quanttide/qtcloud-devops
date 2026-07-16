# CHANGELOG




## [0.2.0] - 2026-07-16

### Added

- 发布流程支持 publish 前后各执行一次 status 检查  
- 新增 `plan roadmap-from-audit` 命令，从审计 JSON 自动生成 ROADMAP  
- 引入 `code audit --json` 及 `plan todo-from-audit` 流水线，支持 JSON 输出和待办自动生成  
- 合并 `code.rs` 中的函数级与模块级检测指标，统一审计检查项  

### Changed

- 重构 CLI 命令结构：拆分 source 模块（git/submodule/tag）、contract 模块（按 platform/source/scope/version/stage 分解）、build/test/plan 等模块，并组织为子命令文件夹  
- 重构发布流程为两阶段架构（Plan→Confirm→Execute），调整 `release/status/publish` 及 `code/audit` 等模块  
- 重构数据模型与依赖：git 操作归入 `source/git/worktree`，子模块数据模型下沉至 toolkit，切换依赖至 crates.io，更新 gix 版本  
- 更新 ROADMAP 与 TODO 文档：精简为战略级目标，门禁对齐，去除重复，并同步文档与实际审计实现（9 项检查 + ScannedFile/RuleResult）  
- 重构审计模块：拆分 `count_markers` 上帝函数为每个检查返回 `RuleResult` 的独立函数，提取纯辅助函数  

### Fixed

- 修复 `publish` 对 `Cargo.lock` 的处理缺陷  
- 修复 `source→doctor` 重命名后未更新集成测试的问题  
- 修复 `source→release` 反向依赖，提取 `diagnostics` 模块解决循环依赖  
- 修复 `resolve_roadmap_path` 未相对化 `current_dir` 导致无法匹配 auto-detected scope 的问题
## [0.10.2] - 2026-07-15

**Changed**
- 重构 release 模块，完成四阶段重构，提取 util 子模块，并将 util/detect 迁移至 source 模块
- 将 roadmap 纯函数提取到 source/roadmap.rs，并将 git.rs 子模块扫描合并到 source/git.rs
- 简化 status 模块，使用 ReleaseState 和 toolkit 工具函数，移除不必要的测试样板
- 将 P1/P2 待办项从 TODO.md 移至 ROADMAP.md，并同步更新相关文档
- 升级 CI Action 版本以消除 Node.js 20 废弃警告

**Fixed**
- 修复 tests/cli、tests/code.rs、tests/cli.rs 中缺失的 category header 以及 gh_not_found 测试

**Removed**
- 删除过时的 TODO.md 和 ROADMAP.md 文件
- 删除 release/util 和 release/detect 旧文件（已完成模块迁移）
## [0.1.0] - 2026-05-22

### Added

- docs/index.md：产品文档首版，定位"量潮发布规范封装"
- src/cli：qtcloud-devops-cli v0.1.0 发布到 PyPI
- AGENTS.md CLI 设计规则固化
- ROADMAP.md 优先级体系建立

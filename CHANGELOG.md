# CHANGELOG



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

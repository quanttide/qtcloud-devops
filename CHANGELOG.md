# CHANGELOG

## [0.8.3] - 2026-07-04

### Fixed
- `plan doctor` 非标准 `##` 头/`###` 分类检测
- `plan status` 解析结果为 0 时输出 warning 诊断
- `plan status`/`plan doctor` LLM 先判断 → 规则校验架构
- `plan doctor` 自由格式 → LLM 转换 + 规则校验
- `plan clean` 级联删除空版本时误删相邻版本头
- `release publish` scope dir 被当作独立 git 仓库执行的问题
- `release publish` update_config_version 移至一致性检查前

## [0.1.0] - 2026-05-22

### Added

- docs/index.md：产品文档首版，定位"量潮发布规范封装"
- src/cli：qtcloud-devops-cli v0.1.0 发布到 PyPI
- AGENTS.md CLI 设计规则固化
- ROADMAP.md 优先级体系建立

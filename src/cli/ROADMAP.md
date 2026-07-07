# ROADMAP — qtcloud-devops-cli plan 命令升级

> 当前 `plan doctor/clean/audit` 覆盖范围不一致：doctor 已同时处理 ROADMAP 和 TODO，clean 只处理 ROADMAP，audit 结构检查不足。本版本统一三个命令的行为。

## [0.11.0]

### Changed

- [ ] `plan clean` 同时清理 ROADMAP 和 TODO 的已完成条目
- [ ] `plan audit` 新增路径存在性、粒度达标、孤儿 ROADMAP 条目三项结构检查

### Fixed

- [ ] `plan doctor` LLM prompt 标题同时覆盖 ROADMAP 和 TODO

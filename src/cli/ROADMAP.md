# ROADMAP — qtcloud-devops-cli

## [0.8.3]

### Fixed
- [ ] `plan status`：遇到非标准 `##` 头时输出 warning，而非静默返回空
- [ ] `plan doctor`：检测非标准 `##` / `###` 行（如 `## P0 — 阻塞`）
- [ ] `plan doctor`：LLM 先判断 → 规则校验（当前是反的）
- [ ] `plan clean`：级联删除空版本时不牵连相邻非空版本
- [ ] `release publish`：scope dir 不是独立 git 仓库时报错

## [0.8.2]

### Added
- [x] `plan doctor` 接入 LLM 修复（基于 `quanttide-agent`）
- [x] `build status` deps 依赖检查（path / git 检测）

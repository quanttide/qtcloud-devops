# ROADMAP — qtcloud-devops-cli

## [0.8.3]

### Fixed
- [x] `plan doctor` 检测非标准 `##` 头：`## P0 — 阻塞` 应被检测为格式问题，而非跳过返回 ✅
- [x] `plan doctor` 检测非标准 `###` 分类：`### 0.1 xxx`、`### 1.1 xxx` 等非标准分类应被报告
- [x] `plan status` 诊断输出：解析结果为 0 但文件含 `##` 行时，输出 warning 标明行号和内容
- [ ] `plan status` / `plan doctor` 架构：LLM 先判断 → 规则校验（当前规则先，LLM 被跳过了）
- [ ] `plan doctor` 应能将自由格式 ROADMAP 转换为标准格式（当前只修 3 类细节）
- [x] `plan clean` 级联删除空版本时连带删了相邻非空版本头（空行间隔误判）
- [ ] `release publish` scope dir 被当作独立 git 仓库执行 git 命令，monorepo 子目录报错
- [ ] `release publish` 不自动更新 scope 版本号（只检查一致性，不修改）

## [0.8.2]

### Added
- [x] `plan doctor` 接入 LLM 修复（基于 `quanttide-agent`）
- [x] `build status` deps 依赖检查（path / git 检测）

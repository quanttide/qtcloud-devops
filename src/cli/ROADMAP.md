# ROADMAP — qtcloud-devops-cli

## [0.10.0] — 基于 quanttide-devops 0.2 重构 + code 命令重新设计

### Added
- [ ] quanttide-devops 0.2 升级
  - [ ] 更新 Cargo.toml 依赖版本至 0.2
  - [ ] 适配 contract 模块 API 变更
- [ ] `Git` trait 接入
  - [ ] 删除 `src/git/` 手写模块，委托给 `quanttide-devops::git_core`
  - [ ] 删除 `src/code/` 适配层
  - [ ] `RealGit` 由 `quanttide-devops` 提供
  - [ ] `MockGit` 集成到测试
- [ ] code 命令重新设计：简化 status/sync 接口
- [ ] `release publish --dry-run` 支持 cwd scope 自动检测

### Changed
- [ ] 移除 `git2` 依赖，全部读操作走 `gix` + `quanttide-devops` trait

- [ ] 测试辅助函数从 git2 改为 `MockGit`

### Fixed
- [ ] `release publish` 从 monorepo root 执行误检测 scope
- [ ] `gh release create` 超时无重试
- [ ] release 后主仓库子模组指针未自动更新


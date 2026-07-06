# ROADMAP — qtcloud-devops-cli

## [0.10.0] — 基于 quanttide-devops 0.2 重构 + code 命令重新设计

### Added
- [ ] 基于 quanttide-devops 0.2 的 `Git` trait 重构 git 模块
- [ ] `Git` trait 双实现：`RealGit`（gix） + `MockGit`（测试）
- [ ] code 命令重新设计：简化 status/sync 接口
- [ ] `release publish --dry-run` 支持 cwd scope 自动检测

### Changed
- [ ] 移除 `git2` 依赖，全部读操作走 `gix` + `quanttide-devops` trait

- [ ] 测试辅助函数从 git2 改为 `MockGit`

### Fixed
- [ ] `release publish` 从 monorepo root 执行误检测 scope
- [ ] `gh release create` 超时无重试
- [ ] `plan clean/status` scope 路径解析：无 contract 时只查仓库根目录，未搜索 scope 子目录内的 ROADMAP
- [ ] release 后主仓库子模组指针未自动更新


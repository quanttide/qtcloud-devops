# ROADMAP — qtcloud-devops-cli

## [0.10.0] — 基于 quanttide-devops 0.2 重构 + code 命令重新设计 + plan 命令统一

### Added
- [ ] 基于 quanttide-devops 0.2 的 `Git` trait 重构 git 模块
- [ ] `Git` trait 双实现：`RealGit`（gix） + `MockGit`（测试）
- [ ] `src/code.rs` code 命令重新设计：简化 status/sync 接口
- [ ] `release publish --dry-run` 支持 cwd scope 自动检测
- [ ] `plan clean` 同时清理 ROADMAP 和 TODO 的已完成条目
- [ ] `plan audit` 新增路径存在性、粒度达标、孤儿 ROADMAP 条目三项结构检查

### Changed
- [ ] 移除 `git2` 依赖，全部读操作走 `gix` + `quanttide-devops` trait
- [ ] 测试辅助函数从 git2 改为 `MockGit`
- [ ] `plan doctor` LLM prompt 标题同时覆盖 ROADMAP 和 TODO
- [ ] 清理长函数技术债：拆分 10+ 个 MUST 级长函数（`main` 190 行、`publish` 165 行、`plan_audit` 126 行等），目标单函数 ≤ 60 行
- [ ] 补充 `release/audit.rs`、`contract.rs` 缺失的单元测试
- [ ] 精简 `determine_submodule_status` 等长参数列表（≥6 参数），优先用 struct 传参

### Fixed
- [ ] `release publish` 从 monorepo root 执行误检测 scope
- [ ] `gh release create` 超时无重试
- [ ] `plan clean/status` scope 路径解析：无 contract 时只查仓库根目录，未搜索 scope 子目录内的 ROADMAP
- [ ] `src/release/publish.rs` release 后主仓库子模组指针未自动更新
# ROADMAP

## 已完成

### Core — Rust 子模块管理引擎

Rust 核心已从 `examples/default` 迁入并完成适配。代码结构从 `packages/code/` 重构为 `app/rust/` + `app/python/`。

**交付物**：
- `app/rust/python.rs`：`scan_repo` / `sync_single` / `sync_all` / `retire_submodule` 全部 4 个 pyfunction 绑定
- `app/python/code.py`：Python 封装层，对接 Rust native 调用
- `app/python/cli.py`：注册 `code` 子命令组（status / sync / retire）
- `integrated_tests/`：17 个集成测试覆盖 CLI 结构、参数校验、错误处理
- `tests/`：51 个单元测试覆盖 release 逻辑和 config
- 编译：`cargo build` + `cargo test` (22 tests) 通过
- 构建：maturin 单构建系统（`source-dir = "."`，module-name = `qtcloud_devops_cli`）

## 待规划（按优先级）

### P0 — 发布目标支持

先做具体实现，从实现中提炼模型。

- **PyPI 发布集成**：release 命令支持发布到 PyPI（版本校验、构建、发布、验证）
- **pub.dev 发布集成**：release 命令支持发布到 pub.dev
- **发布目标抽象**：从 PyPI/pub.dev 的具体实现中提取"发布目标"模型，支持通过契约声明灵活配置

### P1 — 体验修复

- **CHANGELOG 路径智能检测**：自动查找 pyproject.toml 同层的 CHANGELOG.md，减少 `--changelog` 使用场景

### P2 — 配置扩展

- 放宽分支限制（可配置允许的分支列表）
- 支持非 semver 版本策略
- CI Action 版本升级（Node.js 20 弃用）
- GitLink 镜像容灾同步（优先级较低，可手动维护）

## 基本假设

| # | 假设 | 说明 |
|---|------|------|
| 1 | GitHub 为中心 | 主开发在 GitHub，使用 `gh` CLI。GitLink 仅作镜像容灾 |
| 2 | 有 CHANGELOG.md | 格式 `## [X.Y.Z]`，默认查找当前目录 |
| 3 | semver 版本号 | 版本号 `vX.Y.Z` 或 `scope/vX.Y.Z` |
| 4 | 工作区干净 | 发布前无未提交变更 |
| 5 | 发布分支受限 | 仅 `main` / `master` / `release/*` 可发布 |
| 6 | git remote 可达 | 从 `git remote get-url origin` 自动检测仓库 |
| 7 | 用户可交互 | 发布确认需 TTY 交互，CI 需 `-y` 跳过 |

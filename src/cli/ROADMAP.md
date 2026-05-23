# ROADMAP

## 已完成

### Core — Rust 子模块管理引擎

`packages/code/` Rust crate 已从 `examples/default` 迁入，`app/code.py` 已封装 native 调用，`pyproject.toml` 已配置 maturin 构建。

**剩余工作**：
- `src/python.rs` 中 `fn kse_core` 需重命名为 `fn qtcloud_devops_code`，并补全 sync/retire 的 pyfunction 绑定
- `cargo build` 验证编译通过
- `pip install -e .[code]` 验证 Python 集成

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

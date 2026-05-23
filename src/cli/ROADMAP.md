# ROADMAP

## 开发中

### Core — Rust 子模块管理引擎（当前迭代）

将 `quanttide-example-of-devops` 仓库中的子模块管理 Rust crate 迁移到 `packages/code`，通过 pyo3 绑定集成到 CLI。

**架构**：

```
app/cli.py (Typer)
  └── app/code.py              ← 新增，封装 native 调用
        └── packages/code/      ← Rust crate，编译为 cdylib
              ├── src/python.rs ← PyO3 绑定层
              │     ├── scan_repo()  → status
              │     ├── sync_single / sync_all → sync
              │     └── retire()     → retire
              ├── src/lib.rs    ← crate 入口
              ├── src/model/    ← 核心模型（SubmoduleStatus, RepoState）
              └── src/commands/ ← SubmoduleEditor trait + GitSubmoduleEditor impl
```

**步骤**：

| # | 任务 | 预期产物 |
|---|------|----------|
| 1 | 从 `examples/default` 复制 Rust 代码到 `packages/code` | `packages/code/` 目录完整 |
| 2 | 调整 `src/python.rs`（重命名 pymodule `kse_core` → `qtcloud_devops_code`；新增 sync/retire 绑定）；Cargo.toml 包名/lib 名已正确，核实即可 | `cargo build` 通过 |
| 3 | 配置 maturin 构建（`pyproject.toml`：`source-dir = "../../packages/code"`，`features = ["python"]`） | `pip install -e .[code]` 自动编译 Rust |
| 4 | 新增 `app/code.py` 封装 native 调用 + `app/cli.py` 注册子命令 | `qtcloud-devops code status` 可用 |
| 5 | 更新文档和 AGENTS.md | 明确 Rust 开发环境需求 |
| 6 | 清理 `examples/default`（标记废弃或移除） | 职责集中到 qtcloud-devops |

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

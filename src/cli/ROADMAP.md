# ROADMAP

## v0.3.0 — 纯 Rust CLI，移除 Python 入口 `[BREAKING]`

用 Rust 重写 `release` 逻辑，移除 Python CLI 入口，使 `qtcloud-devops` 成为纯 Rust CLI 工具。PyPI 包保留为 native library 分发渠道（通过 `_native.so` 暴露 scan_repo / sync / retire）。

### 动机

- 当前 `release` 命令是纯 Python，与 Rust 实现的子模块管理引擎架构不一致
- Python 入口（`cli.py`）是多余的间接层——用户始终通过 Rust 二进制使用工具
- 维护两套 CLI 框架（Typer + clap）和两种构建系统增加认知负担

### BREAKING：CLI 接口变更

```
之前                             之后
─────────────────────           ─────────────────────
qtcloud-devops release          qtcloud-devops stage -v v0.1.0     ← 标记版本
  --version v0.1.0              qtcloud-devops publish -v v0.1.0   ← 发布上线
  [--tag-only]                  qtcloud-devops cancel -v v0.1.0    ← 取消
  [--release-only]              qtcloud-devops retire -v v0.1.0    ← 退役
  [--dry-run]
  [-y]
```

- 删除 `release` 命令，只提供 `stage`/`publish`/`cancel`/`retire` 四个子命令
- `release --version v0.1.0 -y` → `qtcloud-devops stage -v v0.1.0 && qtcloud-devops publish -v v0.1.0 -y`
- `release --tag-only` 无直接等价（`stage` 不打标签，`publish` 始终打标签），用户需手动 `git tag`

### 待办

1. **Rust 重写 release 逻辑**
   - 实现状态机模型：`ReleaseRecord`、`ReleaseEntry`、`ReleaseStatus`、`Storage` trait、`FileStorage`（基于 `release-journal.jsonl` 事件溯源）
   - 实现四个命令：`stage`（标记版本）、`publish`（发布上线）、`cancel`（取消）、`retire`（退役）
   - `release` 命令**删除**（v0.2.x 用户改为 `stage && publish` 组合）
   - 整合断言检查（git 工作区干净、合法分支等）到 Rust 层
   - 注册子命令到 `src/main.rs`

2. **移除 Python CLI 入口**
   - 删除 `src/qtcloud_devops_cli/release.py`
   - 更新 `pyproject.toml`：移除 `[project.scripts]` 入口点

3. **清理 Python 封装层**
   - 删除 `src/qtcloud_devops_cli/code.py`、`config.py`、`cli.py`
   - `__init__.py` 只保留一行注释
   - `_native.so` 作为 maturin 构建副产品保留，**不主动维护也不删除**

4. **保留 PyPI 分发**
   - `_native` 库通过 maturin 构建，保留 `python` feature
   - GitHub Releases 分发 Rust 二进制 + `cargo install` 作为额外安装方式
   - 双渠道：`pip install` 给 Python 开发者，`cargo install` / GitHub Releases 给 Rust 开发者

5. **测试迁移**
   - `tests/python/` 下所有测试**全部删除**（逻辑由 Rust 单元测试覆盖）
   - `integrated_tests/` 中依赖 Python CLI 的用例**全部删除**
   - Rust 单元测试覆盖全部 release 逻辑路径（目标 ≥ 40 个新测试）
   - 验收标准：`cargo test` 全部通过，无 Python 测试残留

6. **文档更新**
   - `docs/release.md` 更新为 Rust CLI 用法，标注 BREAKING 变更
   - `docs/index.md` 更新构建/安装说明

## v0.3.1 — release status

新增 `release status` 命令，查看当前项目的发布状态。每次操作开始和结束时执行一次，形成操作前后的状态对比。

**功能**：
- 当前版本号
- 最新发布记录
- 未发布的变更摘要
- 预发布版本列表

## v0.4.0 — plan / build / test

以 `release` 命令组为蓝本，新增三个命令组，覆盖完整开发工作流：

| 命令 | 职责 |
|------|------|
| `plan` | 围绕项目规划文件的命令。扫描 BUGS、ROADMAP、TODO 等项目管理文件，生成变更摘要或进度报告 |
| `build` | 围绕 CI 的构建命令。触发或查询 CI 构建状态，与 GitHub Actions 等 CI 系统交互 |
| `test` | 围绕测试的测试命令。运行测试套件并报告结果，支持过滤和摘要输出 |

**风格**：与现有 `release` 命令组一致——Rust 实现、状态驱动、原子操作。

## 待规划

### P0 — 发布目标支持

- **pub.dev 发布集成**：release 命令支持发布到 pub.dev
- **发布目标抽象**：从 PyPI/pub.dev 的具体实现中提取"发布目标"模型

### P1 — 体验修复

- **Orphaned 状态拆分**（推迟自"开发中"）：将 `Orphaned` 拆分为更精确的子状态（rebase force push、squash merge、仓库替换等），更新 `RepoState::scan()` 判定逻辑和 `describe_issue()` 建议

### P2 — 配置扩展

- 放宽分支限制（可配置允许的分支列表）
- 支持非 semver 版本策略
- CI Action 版本升级（Node.js 20 弃用）
- GitLink 镜像容灾同步

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

# ROADMAP

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

# ROADMAP

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

## 待规划（按优先级）

### P0 — 体验修复

- **CHANGELOG 路径智能检测**：自动查找 pyproject.toml 同层的 CHANGELOG.md，减少 `--changelog` 使用场景

### P1 — 基础设施

- **GitLink 镜像容灾同步**：发布时同步推送 tag 到 GitLink remote

### P2 — 架构设计（只设计，不实现）

- **语言/框架契约**：设计数据模型，显式声明项目语言、包类型、发布目标。不急于实现，为后续功能提供架构基础
- **发布目标管理**：基于契约模型，将 PyPI/pub.dev/Docker Hub 等作为系统级概念接入，派生发布策略

### P3 — 配置扩展

- 放宽分支限制（可配置允许的分支列表）
- 支持非 semver 版本策略
- CI Action 版本升级（Node.js 20 弃用）

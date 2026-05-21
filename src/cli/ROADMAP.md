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

## 待规划（由用户决定）

### 发布目标管理

发布目标（PyPI、pub.dev、Docker Hub 等）是 devops 系统的核心概念，需要显式建模：

- **包注册表作为发布目标**：PyPI、pub.dev、npm 等是 release 的消费端，不是某个项目的私事
- **契约声明发布目标**：项目通过契约声明"发布到 PyPI"，devops 工具据此选择发布策略
- **发布后验证**：推送到注册表后验证是否成功（如 `pip install` 检查版本是否存在）
- **多目标发布**：同一项目可能同时发布到 PyPI 和镜像源
- GitLink 镜像容灾同步（发布时同步推送 tag 到 GitLink remote）

### devops 工具功能

- CHANGELOG 路径智能检测（自动查找 pyproject.toml 同层的 CHANGELOG.md）
- 支持非 semver 版本策略
- 放宽分支限制（可配置允许的分支列表）
- CI Action 版本升级（Node.js 20 弃用）

### 语言/框架契约（谨慎设计）

- 通过契约显式声明项目语言、包类型、发布目标
- `init` 命令：在当前目录初始化 DevOps 契约
- `check` 命令：运行所有预定义契约检查
- 集成到 release 预检查中

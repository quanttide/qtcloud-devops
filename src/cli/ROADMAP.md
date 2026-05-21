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

### devops 工具功能

- CHANGELOG 路径智能检测（自动查找 pyproject.toml 同层的 CHANGELOG.md）
- GitLink 镜像容灾同步（发布时同步推送 tag 到 GitLink remote）
- 支持非 semver 版本策略
- 放宽分支限制（可配置允许的分支列表）
- CI Action 版本升级（Node.js 20 弃用）

### 语言/框架契约（谨慎设计）

- 通过契约显式声明项目语言、包类型（Python/Dart/Rust/…）
- `init` 命令：在当前目录初始化 DevOps 契约
- `check` 命令：运行所有预定义契约检查
- 集成到 release 预检查中
- 发布行为（如推 PyPI/pub.dev）由契约派生而非硬编码

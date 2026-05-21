# ROADMAP

## 基本假设

| # | 假设 | 说明 |
|---|------|------|
| 1 | GitHub 为中心 | 仓库托管在 GitHub，使用 `gh` CLI |
| 2 | 有 CHANGELOG.md | 格式 `## [X.Y.Z]`，默认查找当前目录 |
| 3 | semver 版本号 | 版本号 `vX.Y.Z` 或 `scope/vX.Y.Z` |
| 4 | 工作区干净 | 发布前无未提交变更 |
| 5 | 发布分支受限 | 仅 `main` / `master` / `release/*` 可发布 |
| 6 | git remote 可达 | 从 `git remote get-url origin` 自动检测仓库 |
| 7 | 用户可交互 | 发布确认需 TTY 交互，CI 需 `-y` 跳过 |

## v0.1.0 ✅

CLI 接口重构、文档体系建立、PyPI 发布。

### Done

- `--tag-only` / `--release-only` 参数，分开执行 tag 和 GitHub Release
- 自动检测 remote 仓库，移除 `--repo` 参数
- 默认模式幂等（tag 已存在时跳过创建，继续发 release）
- `--release-only` 预检查验证 tag 必须存在
- docs/index.md、commands.md、low-level-api.md、README.md
- AGENTS.md CLI 设计规则固化
- .github/workflows/publish-cli.yml（PyPI 自动发布）
- 发布到 PyPI（v0.1.0）

## v0.2.0（计划）

### CHANGELOG 路径智能检测

- 默认查找 `pyproject.toml` 同层目录的 `CHANGELOG.md`，而非 cwd
- 减少 `--changelog` 的使用场景

### CLI 自检验证

- `--check-pypi`：发布后验证 PyPI 是否成功
- 发布后自动建议执行验证命令

### CI Action 版本升级

- 升级 `actions/checkout` 和 `actions/setup-python` 至支持 Node.js 24 的版本

### 后续考虑

- 契约检查集成到 `release` 预检查中
- `check` 命令：运行所有预定义契约检查
- `init` 命令：在当前目录初始化 DevOps 契约
- 支持非 GitHub 仓库（GitLab、Gitea 等）
- 支持非 semver 版本策略
- 放宽分支限制（可配置允许的分支列表）

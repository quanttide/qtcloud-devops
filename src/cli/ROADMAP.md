# ROADMAP

## 基本假设

当前流程基于以下假设，后续迭代可能放宽或变更：

| # | 假设 | 说明 |
|---|------|------|
| 1 | GitHub 为中心 | 仓库托管在 GitHub，使用 `gh` CLI，repo 格式 `owner/name` |
| 2 | 有 CHANGELOG.md | 版本记录在项目根目录 `CHANGELOG.md`，格式 `## [X.Y.Z]` |
| 3 | semver 版本号 | 版本号 `vX.Y.Z` 格式 |
| 4 | 工作区干净 | 发布前无未提交变更 |
| 5 | 发布分支受限 | 仅 `main` / `master` / `release/*` 可发布 |
| 6 | git tag = Release | 每个版本对应 tag，推送到 origin 后创建 GitHub Release |
| 7 | 用户可交互 | 发布确认需 TTY 交互，CI 需 `-y` 跳过 |
| 8 | 单仓库发布 | 子模块发布是独立流程 |

## v0.0.1 ✅

发布 `release` 命令 — 预检查、打标签、推送、创建 GitHub Release。

### Done

- `release` 命令：预检查（版本号格式、CHANGELOG 存在性、tag 重复、工作区干净、分支检查）
- `release` 命令：发布前确认交互（🧠 AI 介入点）
- `release` 命令：发布后验证（`gh release view`）
- `release` 命令：异常回滚（标签推送失败 / Release 创建失败自动回滚）

### Todo

- 契约检查集成到 `release` 预检查中
- 子模块发布支持
- `check` 命令：运行所有预定义契约检查
- `init` 命令：在当前目录初始化 DevOps 契约
- 支持非 GitHub 仓库（GitLab、Gitea 等）
- 支持非 semver 版本策略
- 放宽分支限制（可配置允许的分支列表）

# qtcloud-devops-cli 设计文档

## 概述

qtcloud-devops-cli 是量潮科技的 DevOps 命令行工具，提供子模块管理（code）和发布管理（stage/publish/cancel/retire）能力。纯 Rust 实现，通过 pip / cargo install / GitHub Releases 分发。

## 设计原则

### 发布操作为独立步骤

Git 标签和 GitHub Release 是两个独立操作，允许分开执行：

- **默认**：标签 + GitHub Release 一步完成，适用于常规发布
- **`--tag-only`**：仅创建并推送 Git 标签，适用于需要先发标签后补 Release 的场景
- **`--release-only`**：仅为已有标签创建 GitHub Release，适用于 tag 已存在需要补 Release 的场景

三者关系：

```
默认（无 flag）= --tag-only + --release-only
--tag-only       = 跳过 --release-only
--release-only   = 跳过 --tag-only
```

### Tag 已存在的处理策略

| 模式 | Tag 不存在 | Tag 已存在 |
|------|-----------|-----------|
| 默认 | 创建 tag + 发 release | 跳过 tag，继续发 release |
| `--tag-only` | 创建 tag | 跳过，静默成功 |
| `--release-only` | 拒绝（错误） | 直接发 release |

这种设计避免了你遇到的"标签已存在无法补 Release"的问题——默认模式幂等，`--release-only` 专门为补发场景设计。

### 仓库自动检测

仓库名通过 `get_remote_repo()` 函数从 `git remote get-url origin` 自动解析，支持 SSH（`git@github.com:owner/name.git`）和 HTTPS（`https://github.com/owner/name`）两种格式。

这简化了 CLI 参数，避免调用者每次都要手动输入仓库名。

`get_remote_repo()` 的返回值取决于当前工作目录所在的 Git 仓库：

| 当前目录 | 所属 Git 仓库 | remote origin | 解析结果 |
|---------|--------------|---------------|---------|
| 根目录 （主仓库） | quanttide-platform | `quanttide/quanttide-platform` | 主仓库 |
| `apps/qtcloud-devops` | qtcloud-devops 子模块 | `quanttide/qtcloud-devops` | 子模块自身 |
| `apps/qtcloud-devops/src/cli` | 同上（向上查找到 qtcloud-devops） | `quanttide/qtcloud-devops` | 子模块自身 |

所以子模块发布时只需 `cd apps/qtcloud-devops/src/cli && qtcloud-devops release`，命令自动使用子模块的 remote。

## 错误处理与回滚

### 回滚策略

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 直接返回错误，无副作用 |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 创建失败 | 若之前创建了 tag 则删除 tag 和远程 tag |

所有回滚都是自动的（在函数内部通过 `rollback_tag()` 完成），调用者无需额外处理。

### 预检查项

- 版本号格式（`vX.Y.Z` 或 `scope/vX.Y.Z`）
- CHANGELOG.md 是否存在且包含目标版本
- Tag 是否已存在（`--release-only` 时要求必须存在，否则不检查）
- 工作区是否干净
- 当前是否在可发布分支（main / master / release/*）


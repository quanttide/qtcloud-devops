# qtcloud-devops-cli 设计文档

## 概述

qtcloud-devops-cli 是量潮科技的 DevOps 命令行工具，提供子模块管理（`code`）和发布管理（`stage`/`publish`/`cancel`/`retire`）能力。纯 Rust 实现，通过 pip / cargo install / GitHub Releases 分发。

## 设计原则

### 发布管理：状态机而非脚本

发布流程建模为有限状态机，四个命令对应合法状态转换：

```
stage  →  Staged     — 标记版本，准备发布
publish →  Published  — 创建标签 + GitHub Release
cancel  →  Cancelled  — 取消发布，回滚制品
retire  →  Retired    — 标记退役，终态不可逆
```

非法操作（如未 stage 就 publish、退役后再 stage）在模型层拒绝。

### 事件溯源（Event Sourcing）

每次状态变更追加记录到 `.quanttide/devops/release-journal.jsonl`，启动时回放所有事件重建当前状态。单一事实来源，不存在"快照 vs 日志"不一致的问题。

### 仓库自动检测

仓库名通过 `get_remote_repo()` 从 `git remote get-url origin` 自动解析：

| 当前目录 | remote origin | 解析结果 |
|---------|--------------|---------|
| 主仓库根目录 | `quanttide/quanttide-platform` | 主仓库 |
| `apps/qtcloud-devops`（子模块） | `quanttide/qtcloud-devops` | 子模块自身 |

```bash
cd apps/qtcloud-devops/src/cli
qtcloud-devops stage -v cli/v0.3.0   # 自动使用子模块 remote
```

### 回滚策略

| 失败点 | 行为 |
|--------|------|
| 创建标签失败 | 无副作用，直接返回错误 |
| 推送标签失败 | 删除本地标签 |
| GitHub Release 创建失败 | 删除本地和远程标签 |

## 分发方式

| 渠道 | 命令 | 适用场景 |
|------|------|---------|
| PyPI | `pip install qtcloud-devops-cli` | CI / 大多数开发者，最低门槛 |
| crates.io | `cargo install qtcloud-devops-cli` | Rust 开发者，获得最新提交 |
| GitHub Releases | 下载预编译二进制 | 无法使用 pip/cargo 的环境 |

PyPI 包通过 maturin 构建，`_native.so` 为构建副产品，不主动维护。

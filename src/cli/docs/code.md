# code 命令

Git 子模块管理工具，作为 `qtcloud-devops` CLI  的 `code` 子命令集提供。

`code` 命令**不做** `git` 已有的事（添加、初始化、更新、切换分支等直接用 `git` 命令），只做 `git` **做不到**的事：**三路 commit 比对 + 7 种状态分类** 和 **子模块 → 父仓库指针同步**。

## 安装

Rust 核心已集成在 CLI 包中，通过 maturin 自动编译。无需手动构建。

## CLI 快速参考

```
qtcloud-devops code <COMMAND> [选项] [参数]
```

## 命令详解

### `status` — 查看仓库及子模块状态

```bash
qtcloud-devops code status
qtcloud-devops code status /path/to/repo
```

输出 JSON 格式：

| 字段 | 含义 |
|------|------|
| `parent_dirty` | 主仓库是否有未提交的变更 |
| `submodules` | 子模块列表（无子模块时为 `[]`） |
| `total` | 子模块总数 |
| `clean_count` | Clean 状态的子模块数 |
| `needs_attention` | 需要关注的子模块名称列表 |

无子模块时退化到普通 git 仓库状态检测，仅报告 `parent_dirty`。

#### 子模块 7 种状态

通过三路 commit 比对（父仓库指针 / 本地 HEAD / 远程 HEAD）判定：

| 状态 | 含义 | 建议 |
|------|------|------|
| Clean | 三方 commit 一致 | 无需操作 |
| AheadOfParent | 本地有父仓库未记录的新提交 | `code sync <name>` |
| BehindRemote | 远程有更新，本地落后 | `git submodule update --remote <name>` |
| Detached | 游离 HEAD | `git checkout <name> <branch>` |
| Dirty | 有未提交的修改 | 手动 commit 或 stash |
| Orphaned | 父仓库记录的 commit 在远程已不存在 | 手动干预 |
| Uninitialized | 尚未初始化 | `git submodule update --init <name>` |

远程不可达时跳过 Orphaned/BehindRemote 判定。

### `sync` — 端到端子模块同步

三步完成子模块同步：

1. **推送子模块** — 将子模块本地 commit 推送到子模块的 remote
2. **更新父指针** — 更新父仓库中的子模块指针并本地提交
3. **推送父仓库** — 将父仓库的指针更新推送到 remote

省略名称时同步全部。

```bash
qtcloud-devops code sync lib-a
qtcloud-devops code sync           # 同步全部
```

### `retire` — 退役子模块

完整自动化反注册：`deinit` + `.gitmodules` + index 清理。

```bash
qtcloud-devops code retire lib-old
```

## 常见场景

```bash
# 查看状态（含主仓库自身）
qtcloud-devops code status

# 同步子模块并推送到 remote
qtcloud-devops code sync lib-a

# 退役
qtcloud-devops code retire lib-old
```

## 故障排除

```bash
qtcloud-devops code --help
qtcloud-devops code <COMMAND> --help
```

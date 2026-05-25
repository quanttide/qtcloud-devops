# code 命令用户指南

`code` 管理 Git 子模块。适合有多仓库项目的团队。

## 场景一：查看状态

```bash
cd /path/to/your/repo
qtcloud-devops code status
```

输出每个子模块的状态。7 种状态中，需要关注的是非 Clean 的项：

| 状态 | 含义 | 怎么做 |
|------|------|--------|
| AheadOfParent | 子模块有新提交 | `code sync <name>` |
| BehindRemote | 远程有更新 | `git submodule update --remote <name>` |
| Dirty | 有未提交修改 | 先 commit，再 `code sync` |
| Detached | 游离 HEAD | `git checkout <name> <branch>` |
| Orphaned | 提交在远程不存在 | 手动修复父指针 |
| Uninitialized | 未初始化 | `git submodule update --init <name>` |

`code status --offline` 跳过网络 fetch，使用本地缓存。

## 场景二：同步子模块

```bash
# 同步单个
qtcloud-devops code sync my-module

# 同步全部
qtcloud-devops code sync
```

同步包含三个步骤：推送子模块 → 更新父指针 → 推送父仓库。

## 场景三：退役子模块

```bash
qtcloud-devops code retire old-module
```

自动执行：`deinit` → 清理 `.gitmodules` → 清理 index。

## 场景四：预览（dry-run）

```bash
# 先看会做什么，不实际执行
qtcloud-devops code sync my-module --dry-run
qtcloud-devops code retire old-module --dry-run
```

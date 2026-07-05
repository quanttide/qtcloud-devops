# 子模块管理教程

以 qtcloud-devops 自身为例，展示完整的子模块管理流程。

## 子模块结构

```
quanttide-devops（主仓库）
├── apps/qtcloud-devops          → qtcloud-devops 应用
├── packages/quanttide-devops-toolkit               → devops 工具包
└── examples/default             → 参考示例
```

## 查看状态

```bash
cd /home/iguo/repos/quanttide/domains/quanttide-devops
qtcloud-devops code status
```

输出每个子模块的状态。Clean 表示正常，其他状态需要关注：

| 状态 | 含义 |
|------|------|
| AheadOfParent | 子模块有新提交，父仓库指针未更新 |
| BehindRemote | 远程仓库有更新，本地子模块未拉取 |
| Dirty | 子模块工作区有未提交修改 |
| Detached | 子模块处于游离 HEAD 状态 |
| Orphaned | 父仓库记录的提交在远程已不存在 |
| Uninitialized | 子模块尚未初始化 |

远程不可达时标记 🛰，跳过 Orphaned/BehindRemote 判定。

## 同步子模块

子模块有 AheadOfParent 或本地有修改需要推送到远程时：

```bash
qtcloud-devops code sync apps/qtcloud-devops
```

同步包含三个步骤：

1. **推送子模块** — 将子模块本地提交推送到子模块的 remote
2. **更新父指针** — 更新父仓库中的子模块指针并本地提交
3. **推送父仓库** — 将父仓库的指针更新推送到 remote

省略名称时同步全部子模块：

```bash
qtcloud-devops code sync
```

## 退役子模块

当子模块不再需要时：

```bash
qtcloud-devops code retire examples/default
```

自动完成反注册：`deinit` → 清理 `.gitmodules` → 清理 index。

## 预览模式

```bash
# 先看会做什么，不实际执行
qtcloud-devops code sync apps/qtcloud-devops --dry-run
qtcloud-devops code retire examples/default --dry-run
```

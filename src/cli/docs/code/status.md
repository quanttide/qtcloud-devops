# code 命令设计

## 定位

管理 Git 子模块的同步状态查看和同步操作。对应 DevOps 生命周期中的 **Code** 阶段——组件间版本协调。

## 命令

```
qtcloud-devops code status [--offline]    查看组件同步状态
qtcloud-devops code sync [name]           同步组件到远端
qtcloud-devops code sync [name] --dry-run 预览同步操作
```

## 架构

`code` 模块是对 `git` 模块的 CLI 封装：

```
code/              CLI 层（参数解析 + 错误格式化）
├── model.rs       SyncStatus / ComponentStatus / StatusReport 模型
├── status.rs      将 git::RepoState 映射为 StatusReport
└── sync.rs        委托 git::GitSubmoduleEditor 执行同步

git/               领域层（纯 git 操作）
├── mod.rs         re-export
├── types.rs       RepoState / SubmoduleStatus 等核心类型
├── scan.rs        扫描 .gitmodules / 检测 ahead/behind
└── editor.rs      GitSubmoduleEditor（status / sync / sync_all）
```

## code status — 查看组件同步状态

按子模块列出 sync 状态，分四档：

| 状态 | 含义 | 触发条件 |
|------|------|---------|
| `已同步` | 本地与远端一致 | Clean |
| `待推送` | 本地有未推送提交 | AheadOfParent |
| `待拉取` | 远端有更新 | BehindRemote |
| `冲突` | 两边都有新提交 | Ahead + Behind 同时存在 |

### 输出示例

```
仓库: /home/user/repo
组件总数: 3
待处理: 1
  libs/sub             待推送 (领先 2 提交)
全部组件已同步
```

### 参数

- `--offline`: 仅扫描本地 .gitmodules，不 fetch 远端，不检测 ahead/behind

## code sync — 同步组件

将子模块的当前 commit 更新到父仓库的索引中并提交。

| 参数 | 行为 |
|------|------|
| 指定 `name` | 仅同步该子模块 |
| 省略 `name` | 同步全部子模块 |
| `--dry-run` | 仅预览，不执行 |

## 公共 API

```rust
// code module
pub enum SyncStatus { Synced, PendingPush, PendingPull, Conflict }
pub struct ComponentStatus { name, status, ahead, behind }
pub struct StatusReport { root, components, total, synced, pending }
pub fn status(root: PathBuf, offline: bool) -> Result<StatusReport, String>
pub fn sync(root: PathBuf, name: &str) -> Result<(), String>
pub fn sync_all(root: PathBuf) -> Result<(), String>
```

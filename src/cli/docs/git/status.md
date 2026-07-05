# git 模块设计

## 定位

封装 Git 子模块的底层操作：扫描 `.gitmodules`、检测子模块状态（ahead/behind）、同步子模块 commit 引用。

被 `code` 模块调用，不直接暴露 CLI。

## RepoState — 仓库状态快照

由 `scan()` 或 `scan_offline()` 生成，表示仓库的完整子模块状态。

```rust
pub struct RepoState {
    pub root: PathBuf,
    pub total: usize,
    pub submodules: Vec<SubmoduleInfo>,
}

pub struct SubmoduleInfo {
    pub name: String,
    pub path: PathBuf,
    pub status: SubmoduleStatus,
}

pub enum SubmoduleStatus {
    Clean,
    AheadOfParent,    // 子模块领先父仓库记录的 commit
    BehindRemote,     // 远端有更新
    AheadAndBehind,   // 两边都有
    NotInitialized,   // 未初始化
    NoRemote,         // 无远端
    Unknown(String),
}
```

## GitSubmoduleEditor — 编辑器

```rust
pub struct GitSubmoduleEditor { root: PathBuf, offline: bool }

impl GitSubmoduleEditor {
    pub fn new(root: PathBuf) -> Self
    pub fn set_offline(&mut self, offline: bool)
    pub fn status(&self) -> Result<Vec<SubmoduleInfo>, GitError>
    pub fn sync_to_parent(&self, name: &str) -> Result<(), GitError>
    pub fn sync_all_to_parent(&self) -> Result<(), GitError>
}
```

### 状态检测逻辑

1. 读取 `.gitmodules` → 获取子模块列表
2. 对每个子模块：
   - 检查是否初始化
   - `git rev-parse HEAD` 获取当前 commit
   - `git rev-parse :path` 获取父仓库记录的 commit
   - 比较两者 → AheadOfParent 或 Clean
   - `git ls-remote` 获取远端 → BehindRemote
3. 组合判定最终状态

### 同步逻辑

`sync_to_parent(name)`:
1. 进入子模块目录
2. `git checkout` 到父仓库记录的 commit
3. 回到父仓库，`git add` 子模块路径
4. `git commit`（message: `chore: sync {name}`）

## 公共 API

```rust
// scan
pub fn scan(root: &Path) -> Result<RepoState, GitError>
pub fn scan_offline(root: &Path) -> Result<RepoState, GitError>

// editor
pub struct GitSubmoduleEditor { ... }

// types
pub enum SubmoduleStatus { Clean, AheadOfParent, ... }
pub struct SubmoduleInfo { name, path, status }
pub struct RepoState { root, total, submodules }
pub struct GitError(pub String);
```

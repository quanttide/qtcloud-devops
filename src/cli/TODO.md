# TODO — quanttide-devops 0.2 迁移

## 1. 升级依赖

- [ ] `Cargo.toml`: `quanttide-devops = "0.2"`
- [ ] 编译，修复所有 API 断变
  - [ ] `contract.rs` re-export 的路径/签名变更
  - [ ] `VersionStatus` 等类型变更
  - [ ] `Language::Unknown` → `Language::Other`

## 2. `Git` trait 接入

quanttide-devops 0.2 的 `git_core` 提供 `Git` trait 和 `RealGit`：

```rust
pub trait Git {
    fn scan(&self, root: &Path) -> Result<RepoState, Error>;
    fn scan_offline(&self, root: &Path) -> Result<RepoState, Error>;
    fn sync_to_parent(&self, root: &Path, name: &str) -> Result<(), Error>;
    fn sync_all_to_parent(&self, root: &Path) -> Result<(), Error>;
    fn status(&self, root: &Path) -> Result<Vec<HealthIssue>, Error>;
}
```

- [ ] 删除 `src/git/scan.rs`，调用改为 `git::RealGit::scan()`
- [ ] 删除 `src/git/editor.rs`，调用改为 `git::RealGit::sync_to_parent()`
- [ ] 删除 `src/git/types.rs`，类型由 `quanttide-devops::git_core` 提供
  - [ ] `Submodule` / `RepoState` / `SubmoduleStatus` / `AggregateStatus`
  - [ ] `HealthIssue` / `describe_issue()`
- [ ] 保留 `src/git/mod.rs` 作为 re-export 桥接层（可选）

## 3. MockGit 集成测试

- [ ] `MockGit` 由 `quanttide-devops` 提供（或自己实现）
- [ ] 替换 `tests/mock.rs` 中的 PATH mock 为 `MockGit`
- [ ] 删除 `tests/code.rs`（不再需要真实的 git 子模块操作）

## 4. code 模块简化

- [ ] `src/code/status.rs`: 委托给 `RealGit::scan()`
- [ ] `src/code/sync.rs`: 委托给 `RealGit::sync_to_parent()`
- [ ] `src/code/model.rs`: 保留或改为使用 `git_core::Submodule`

## 5. 扫尾

- [ ] 删除 `src/git/` 模块（如果桥接层不保留）
- [ ] 删除 `git2` crate 依赖
- [ ] 确认 `cargo test` 419 不变
